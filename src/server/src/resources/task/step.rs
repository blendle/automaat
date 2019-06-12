//! A [`TaskStep`] is the grouping of a [`Processor`], some identification
//! details such as a name and description, and the position within a series of
//! steps.
//!
//! It is similar to a [`Step`], except that it is tied to a [`Task`], instead
//! of a [`Pipeline`]. The difference is that pipelines are pre-defined task
//! templates that can be executed. Once a pipeline is executed, it will spin
//! off a task, with its own steps, and run those steps.
//!
//! [`Processor`]: crate::Processor
//! [`Step`]: crate::resources::Step

use crate::resources::{Step, Task, VariableValue};
use crate::schema::task_steps;
use crate::Database;
use crate::Processor;
use automaat_core::Context;
use chrono::prelude::*;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use juniper::GraphQLEnum;
use serde::{Deserialize, Serialize};
use std::convert::{AsRef, TryFrom};
use std::error;

/// The status of the task step.
#[derive(Clone, Copy, Debug, DbEnum, GraphQLEnum, Serialize, Deserialize)]
#[PgType = "TaskStepStatus"]
#[graphql(name = "TaskStepStatus")]
pub enum Status {
    /// The task step has been created, but is not yet ready to run.
    Initialized,

    /// The task step is waiting and ready to run.
    Pending,

    /// The task step is currently running and will either fail, or succeed.
    Running,

    /// The task step failed to run due to an unforeseen error.
    Failed,

    /// The task step was cancelled, and will not run anymore.
    Cancelled,

    /// The task step ran and succeeded.
    Ok,
}

/// The model representing a task step stored in the database.
#[derive(
    Clone, Debug, Deserialize, Serialize, AsChangeset, Associations, Identifiable, Queryable,
)]
#[belongs_to(Task)]
#[table_name = "task_steps"]
pub(crate) struct TaskStep {
    pub(crate) id: i32,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) processor: serde_json::Value,
    pub(crate) position: i32,
    pub(crate) started_at: Option<NaiveDateTime>,
    pub(crate) finished_at: Option<NaiveDateTime>,
    pub(crate) status: Status,
    pub(crate) output: Option<String>,
    pub(crate) task_id: i32,
}

impl TaskStep {
    /// Returns the processor object attached to this task step.
    ///
    /// Given that tasks are historical entities, and processor object layouts
    /// can change between versions, this method returns an Option enum.
    ///
    /// If `None` is returned, it means the processor data could not be
    /// deserialized into the processor type.
    pub(crate) fn processor(&self) -> Option<Processor> {
        serde_json::from_value(self.processor.clone()).ok()
    }

    pub(crate) fn task(&self, conn: &Database) -> QueryResult<Task> {
        use crate::schema::tasks::dsl::*;

        tasks.filter(id.eq(self.task_id)).first(&**conn)
    }

    pub(crate) fn run(
        &mut self,
        conn: &Database,
        context: &Context,
        input: Option<&str>,
    ) -> Result<Option<String>, Box<dyn error::Error>> {
        self.start(conn)?;

        // TODO: this needs to go in a transaction, and the changes reverted if
        // they can't be saved... Also goes for many other places.

        let result = match self.processor_with_input_and_context(input, context) {
            Ok(p) => p.run(context),
            Err(err) => Err(format!("task processor cannot be deserialized: {}", err).into()),
        };

        match result {
            Ok(output) => {
                self.finished(conn, Status::Ok, output.clone())?;
                Ok(output)
            }
            Err(err) => {
                self.finished(conn, Status::Failed, Some(err.to_string()))?;
                Err(err)
            }
        }
    }

    fn start(&mut self, conn: &Database) -> QueryResult<()> {
        self.status = Status::Running;
        self.started_at = Some(Utc::now().naive_utc());

        match self.save_changes::<Self>(&**conn) {
            Ok(_) => Ok(()),
            Err(err) => {
                self.status = Status::Failed;
                Err(err)
            }
        }
    }

    fn finished(
        &mut self,
        conn: &Database,
        status: Status,
        output: Option<String>,
    ) -> QueryResult<()> {
        self.finished_at = Some(Utc::now().naive_utc());
        self.status = status;
        self.output = output;

        self.save_changes::<Self>(&**conn).map(|_| ())
    }

    fn value_replace(&self, value: &mut serde_json::Value, find: &str, replace: &str) {
        if value.is_array() {
            value
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .for_each(|v| self.value_replace(v, find, replace));
        };

        if !value.is_string() {
            return;
        }

        let string = value.as_str().unwrap().to_owned();
        let string = string.replace(find, replace);
        *value = string.into();
    }

    /// Takes the associated task step processor, and swaps the templated
    /// variables `{$input}` and `{$workspace}` for the actual values.
    fn processor_with_input_and_context(
        &mut self,
        input: Option<&str>,
        context: &Context,
    ) -> Result<Processor, serde_json::Error> {
        let mut processor = self.processor.clone();

        let workspace = context.workspace_path().to_str().expect("valid path");

        processor
            .as_object_mut()
            .expect("unexpected serialized data stored in database")
            .values_mut()
            .for_each(|v| {
                v.as_object_mut()
                    .expect("unexpected serialized data stored in database")
                    .values_mut()
                    .for_each(|v| {
                        self.value_replace(v, "{$input}", input.as_ref().unwrap_or(&""));
                        self.value_replace(v, "${$workspace}", workspace)
                    })
            });

        serde_json::from_value(processor)
    }
}

/// Contains all the details needed to store a step in the database.
///
/// Use [`NewStep::new`] to initialize this struct.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct NewTaskStep<'a> {
    name: &'a str,
    description: Option<&'a str>,
    processor: Processor,
    position: i32,
    started_at: Option<NaiveDateTime>,
    finished_at: Option<NaiveDateTime>,
    output: Option<&'a str>,
    status: Status,
}

impl<'a> NewTaskStep<'a> {
    /// Initialize a `NewStep` struct, which can be inserted into the
    /// database using the [`NewStep#add_to_pipeline`] method.
    pub(crate) const fn new(
        name: &'a str,
        description: Option<&'a str>,
        processor: Processor,
        position: i32,
    ) -> Self {
        Self {
            name,
            description,
            processor,
            position,
            started_at: None,
            finished_at: None,
            output: None,
            status: Status::Initialized,
        }
    }

    /// Add a step to a [`Task`], by storing it in the database as an
    /// association.
    ///
    /// Requires a reference to a `Task`, in order to create the correct data
    /// reference.
    ///
    /// This method can return an error if the database insert failed, or if the
    /// associated processor is invalid.
    pub(crate) fn add_to_task(
        self,
        conn: &Database,
        task: &Task,
    ) -> Result<(), Box<dyn error::Error>> {
        use crate::schema::task_steps::dsl::*;

        self.processor.validate()?;

        let values = (
            name.eq(&self.name),
            description.eq(&self.description),
            processor.eq(serde_json::to_value(self.processor)?),
            position.eq(self.position),
            started_at.eq(self.started_at),
            finished_at.eq(self.finished_at),
            status.eq(Status::Pending),
            output.eq(&self.output),
            task_id.eq(task.id),
        );

        diesel::insert_into(task_steps)
            .values(values)
            .execute(&**conn)
            .map(|_| ())
            .map_err(Into::into)
    }
}

pub(crate) mod graphql {
    //! All GraphQL related functionality is encapsulated in this module. The
    //! relevant functions and structs are re-exported through
    //! [`crate::graphql`].
    //!
    //! API documentation in this module is also used in the GraphQL API itself
    //! as documentation for the clients.
    //!
    //! You can browse to `/graphql/playground` to see all relevant query,
    //! mutation, and type documentation.

    use super::*;
    use juniper::{object, FieldResult, ID};

    #[object(Context = Database)]
    impl TaskStep {
        /// The unique identifier for a specific task step.
        fn id() -> ID {
            ID::new(self.id.to_string())
        }

        /// A descriptive name of the task step.
        fn name() -> &str {
            &self.name
        }

        /// An (optional) detailed description of the functionality provided by
        /// this task step.
        ///
        /// A description _might_ be markdown formatted, and should be parsed
        /// accordingly by the client.
        fn description() -> Option<&str> {
            self.description.as_ref().map(String::as_ref)
        }

        /// The processor used to run the task step.
        fn processor() -> Option<Processor> {
            self.processor()
        }

        /// The position of the step in a task, compared to other steps in the
        /// same task. A lower number means the step runs earlier in the task.
        fn position() -> i32 {
            self.position
        }

        fn started_at() -> Option<DateTime<Utc>> {
            self.started_at.map(|t| DateTime::from_utc(t, Utc))
        }

        fn finished_at() -> Option<DateTime<Utc>> {
            self.finished_at.map(|t| DateTime::from_utc(t, Utc))
        }

        fn status() -> Status {
            self.status
        }

        fn output() -> Option<&str> {
            self.output.as_ref().map(String::as_ref)
        }

        /// The task to which the step belongs.
        ///
        /// This field can return `null`, but _only_ if a database error
        /// prevents the data from being retrieved.
        ///
        /// Every task step is _always_ attached to a task, so in normal
        /// circumstances, this field will always return the relevant task
        /// details.
        ///
        /// If a `null` value is returned, it is up to the client to decide the
        /// best course of action. The following actions are advised, sorted by
        /// preference:
        ///
        /// 1. continue execution if the information is not critical to success,
        /// 2. retry the request to try and get the relevant information,
        /// 3. disable parts of the application reliant on the information,
        /// 4. show a global error, and ask the user to retry.
        fn task(context: &Database) -> FieldResult<Option<Task>> {
            self.task(context).map(Some).map_err(Into::into)
        }
    }
}

impl<'a> TryFrom<(&'a Step, &[VariableValue])> for NewTaskStep<'a> {
    type Error = serde_json::Error;

    fn try_from(
        (step, variable_values): (&'a Step, &[VariableValue]),
    ) -> Result<Self, Self::Error> {
        use serde_json::{from_value, Value};

        // Replace any templated variables inside the providedd `value` object,
        // based on the provided set of `variable_values`.
        //
        // For example: value "{hello} world" with a `VariableValue` with key
        // `hello` and value `hey there` would result in `hey there world`.
        //
        // This only works for string-based value objects. If the value is an
        // array, this function is recursed. Any other value type is ignored.
        fn replace(value: &mut Value, variable_values: &[VariableValue]) {
            if value.is_array() {
                value
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .for_each(|v| replace(v, variable_values));
            };

            if !value.is_string() {
                return;
            }

            variable_values.iter().for_each(|vv| {
                let string = value.as_str().unwrap().to_owned();
                let string = string.replace(&format!("{{{}}}", vv.key), &vv.value);

                *value = string.into();
            });
        }

        let mut processor: Value = step.processor.clone();

        // This is a bit cryptic, but here's what's happening:
        processor
            // We've serialized the processor as a JSON object, so read it in.
            // That will give us back `{ "ProcessorName": { ~properties~ } }`.
            .as_object_mut()
            .expect("unexpected serialized data stored in database")
            // We want the `{ ~properties~ }` object. So take "all" values.
            .values_mut()
            .for_each(|v| {
                // Take the key/value properties stored in `{ ~properties~ }`
                v.as_object_mut()
                    .expect("unexpected serialized data stored in database")
                    // Loop over all of them, and call the `replace` fn.
                    .values_mut()
                    .for_each(|v| replace(v, variable_values))
            });

        Ok(Self::new(
            &step.name,
            step.description.as_ref().map(String::as_ref),
            from_value(processor)?,
            step.position,
        ))
    }
}
