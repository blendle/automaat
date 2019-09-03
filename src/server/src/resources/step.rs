//! A [`Step`] is the grouping of a [`Processor`], some identification details
//! such as a name and description, and the position within a series of steps.
//!
//! A step is one "action" in a series of [`Task`] steps.
//!
//! [`Processor`]: crate::Processor

use crate::resources::Task;
use crate::schema::{steps, variable_advertisements};
use crate::{server::RequestState, Processor};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::convert::{AsRef, TryFrom, TryInto};
use std::error::Error;

/// The model representing a step stored in the database.
#[derive(Clone, Debug, Deserialize, Serialize, Associations, Identifiable, Queryable)]
#[belongs_to(Task)]
#[table_name = "steps"]
pub(crate) struct Step {
    pub(crate) id: i32,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) processor: serde_json::Value,
    pub(crate) position: i32,
    pub(crate) task_id: i32,
}

impl Step {
    pub(crate) fn processor(&self) -> Result<Processor, serde_json::Error> {
        serde_json::from_value(self.processor.clone())
    }

    pub(crate) fn task(&self, conn: &PgConnection) -> QueryResult<Task> {
        use crate::schema::tasks::dsl::*;

        tasks.filter(id.eq(self.task_id)).first(conn)
    }
}

/// Contains all the details needed to store a step in the database.
///
/// Use [`NewStep::new`] to initialize this struct.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct NewStep<'a> {
    pub(crate) name: &'a str,
    description: Option<&'a str>,
    processor: Processor,
    position: i32,
    advertised_variable_key: Option<&'a str>,
    task_id: Option<i32>,
}

impl<'a> NewStep<'a> {
    /// Initialize a `NewStep` struct, which can be inserted into the
    /// database using the [`NewStep#add_to_task`] method.
    pub(crate) const fn new(
        name: &'a str,
        description: Option<&'a str>,
        processor: Processor,
        position: i32,
        advertised_variable_key: Option<&'a str>,
    ) -> Self {
        Self {
            name,
            description,
            processor,
            position,
            advertised_variable_key,
            task_id: None,
        }
    }

    /// Add a step to a [`Task`], by storing it in the database as an
    /// association.
    ///
    /// Requires a reference to a Task, in order to create the correct data
    /// reference.
    ///
    /// If the step has an advertised variable key configured, it will be saved
    /// in the database as well. This happens within a transaction so both
    /// inserts have to succeed.
    ///
    /// This method can return an error if the database insert failed, or if the
    /// step processor cannot be serialized.
    ///
    /// If a step with the same name is already assigned to the task, it will be
    /// updated.
    pub(crate) fn create_or_update(
        self,
        conn: &PgConnection,
        task: &Task,
    ) -> Result<(), Box<dyn Error>> {
        use crate::models::NewVariableAdvertisement;

        let values = (
            steps::name.eq(&self.name),
            steps::description.eq(&self.description),
            steps::processor.eq(serde_json::to_value(self.processor)?),
            steps::position.eq(&self.position),
            steps::task_id.eq(self.task_id.unwrap_or(task.id)),
        );

        let advertised_key = &self.advertised_variable_key;

        conn.transaction(move || {
            let _ = conn.execute("SET CONSTRAINTS ALL DEFERRED")?;

            let step: Step = diesel::insert_into(steps::table)
                .values(&values)
                .on_conflict((steps::name, steps::task_id))
                .do_update()
                .set(values.clone())
                .get_result(conn)
                .map_err(Into::<Box<dyn Error>>::into)?;

            if let Some(key) = advertised_key {
                let _ = NewVariableAdvertisement::new(key, step.id).create_or_update(conn)?;
            } else {
                let filter = variable_advertisements::table
                    .filter(variable_advertisements::step_id.eq(step.id));
                let _ = diesel::delete(filter).execute(conn)?;
            };

            Ok(())
        })
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
    use crate::ProcessorInput;
    use juniper::{object, FieldResult, GraphQLInputObject, ID};

    /// Contains all the data needed to create a new `Step`.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct CreateStepInput {
        /// The name of the step.
        ///
        /// Take care to give a short, but descriptive name, to provide maximum
        /// flexibility for the UI, _and_ provide maximum understanding for the
        /// user.
        pub(crate) name: String,

        /// An optional description of the step.
        ///
        /// While the description is optional, it is best-practice to provide
        /// relevant information so that the user knows what to expect when
        /// adding a step to a task.
        pub(crate) description: Option<String>,

        /// The processor used by this step to perform the required action.
        ///
        /// **note**
        ///
        /// Due to the fact that the GraphQL spec does not (yet) support
        /// union-based input types, this is a wrapper type with a separate
        /// field for each processor type supported by the server.
        ///
        /// All fields are nullable, but you MUST provide EXACTLY ONE processor
        /// input type. Providing anything else will result in an error from the
        /// API.
        pub(crate) processor: ProcessorInput,

        /// Advertise the key of a variable this step can provide a value for.
        ///
        /// This an optional, free-form name.
        ///
        /// As an example, say that this step were to fetch a customer UUID from
        /// an external data store and provide that UUID as the output value of
        /// this step, then the `advertised_variable_key` value could be set to
        /// `Customer UUID`.
        ///
        /// Now, any other task that needs a variable with that exact name as
        /// its input, can use the task this step belongs to to fetch that
        /// value.
        pub(crate) advertised_variable_key: Option<String>,
    }

    #[object(Context = RequestState)]
    impl Step {
        /// The unique identifier for a specific step.
        fn id() -> ID {
            ID::new(self.id.to_string())
        }

        /// A descriptive name of the step.
        fn name() -> &str {
            &self.name
        }

        /// An (optional) detailed description of the functionality provided by
        /// this step.
        ///
        /// A description _might_ be markdown formatted, and should be parsed
        /// accordingly by the client.
        fn description() -> Option<&str> {
            self.description.as_ref().map(String::as_ref)
        }

        /// The processor type used to run the step.
        ///
        /// This query can fail, if the processor failed to be deserialized.
        fn processor() -> FieldResult<Processor> {
            self.processor().map_err(Into::into)
        }

        /// The position of the step in a task, compared to other steps in
        /// the same task. A lower number means the step runs earlier in the
        /// task.
        fn position() -> i32 {
            self.position
        }

        /// The task to which the step belongs.
        ///
        /// This field can return `null`, but _only_ if a database error
        /// prevents the data from being retrieved.
        ///
        /// Every step is _always_ attached to a task, so in normal
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
        fn task(context: &RequestState) -> FieldResult<Option<Task>> {
            self.task(&context.conn).map(Some).map_err(Into::into)
        }
    }
}

#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
impl<'a> TryFrom<(usize, &'a graphql::CreateStepInput)> for NewStep<'a> {
    type Error = String;

    fn try_from((index, input): (usize, &'a graphql::CreateStepInput)) -> Result<Self, String> {
        Ok(Self::new(
            &input.name,
            input.description.as_ref().map(String::as_str),
            input.processor.clone().try_into()?,
            index as i32,
            input.advertised_variable_key.as_ref().map(String::as_str),
        ))
    }
}
