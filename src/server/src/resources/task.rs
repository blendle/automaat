//! A [`Task`] is a job that is scheduled to be ran, or already ran in the past.
//!
//! It is similar to a [`Pipeline`], but a pipeline represents a set of steps
//! that _can be ran_ by providing a set of variables, whereas a task represents
//! a set of steps that are _ready to run_ and have their variables swapped for
//! real values.

use crate::resources::{NewTaskStep, Pipeline, TaskStep, TaskStepStatus, VariableValue};
use crate::schema::tasks;
use crate::Database;
use automaat_core::Context;
use diesel::prelude::*;
use juniper::GraphQLEnum;
use serde::{Deserialize, Serialize};
use std::convert::{Into, TryInto};
use std::error;
use std::thread;

pub(crate) mod step;

/// The status of the [`Task`].
#[derive(Clone, Copy, Debug, Serialize, Deserialize, GraphQLEnum, DbEnum)]
#[PgType = "TaskStatus"]
#[graphql(name = "TaskStatus")]
pub(crate) enum Status {
    /// The task is scheduled for execution in the future.
    Scheduled,

    /// The task is ready to be executed and waiting for the scheduler to pick
    /// it up.
    Pending,

    /// The task is currently running its steps one by one.
    Running,

    /// One of the task steps failed, resulting in the task itself to fail.
    Failed,

    /// The task was cancelled.
    Cancelled,

    /// All task steps ran successfully.
    Ok,
}

impl From<TaskStepStatus> for Status {
    fn from(status: TaskStepStatus) -> Self {
        use Status::*;

        match status {
            TaskStepStatus::Initialized => Scheduled,
            TaskStepStatus::Pending => Pending,
            TaskStepStatus::Running => Running,
            TaskStepStatus::Failed => Failed,
            TaskStepStatus::Cancelled => Cancelled,
            TaskStepStatus::Ok => Ok,
        }
    }
}

#[derive(
    Clone, Debug, Deserialize, Serialize, AsChangeset, Associations, Identifiable, Queryable,
)]
#[belongs_to(Pipeline, foreign_key = "pipeline_reference")]
#[table_name = "tasks"]
/// The model representing a task stored in the database.
pub(crate) struct Task {
    pub(crate) id: i32,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) status: Status,

    // This is a weak reference, meaning that pipelines can be removed, which
    // breaks the link between a task and the pipeline it was created from. This
    // is acceptable, it just means the UI can't link back to the pipeline.
    //
    // Similarly, a task can be created separately from a reference, in which
    // case this field is also `None`.
    pub(crate) pipeline_reference: Option<i32>,
}

impl Task {
    pub(crate) fn as_running(&mut self, conn: &Database) -> QueryResult<Self> {
        self.status = Status::Running;
        self.save_changes(&**conn)
    }

    pub(crate) fn as_failed(&mut self, conn: &Database) -> QueryResult<Self> {
        self.status = Status::Failed;
        self.save_changes(&**conn)
    }

    pub(crate) fn pipeline(&self, conn: &Database) -> QueryResult<Option<Pipeline>> {
        use crate::schema::pipelines::dsl::*;

        match self.pipeline_reference {
            None => Ok(None),
            Some(pipeline_id) => pipelines
                .filter(id.eq(pipeline_id))
                .first(&**conn)
                .optional(),
        }
    }

    pub(crate) fn steps(&self, conn: &Database) -> QueryResult<Vec<TaskStep>> {
        use crate::schema::task_steps::dsl::*;

        TaskStep::belonging_to(self)
            .order(position.asc())
            .load(&**conn)
    }

    /// Mark a task ready to run by changing its status to `Pending`.
    pub(crate) fn enqueue(&mut self, conn: &Database) -> QueryResult<Self> {
        self.status = Status::Pending;
        self.save_changes(&**conn)
    }

    // TODO: implement some kind of `TaskRunner`, that has a reference to
    // &Database, and then impl `Drop` so that if the runner stops, we can check
    // the result, and update the database based on the final status.
    pub(crate) fn run(&self, conn: &Database) -> Result<(), Box<dyn error::Error>> {
        use crate::schema::tasks::dsl::*;

        let data: Option<String> = None;
        let context = Context::new()?;
        let mut steps = self.steps(conn)?;

        let _ = steps.iter_mut().try_fold(data, |input, step| {
            step.run(conn, &context, input.as_ref().map(String::as_str))
        })?;

        // TODO: need to test this, I believe this will always take the status
        // of the last step, which might not be the step that failed.
        match steps.last() {
            Some(step) => diesel::update(self)
                .set(status.eq(Status::from(step.status)))
                .execute(&**conn)
                .map(|_| ())
                .map_err(Into::into),
            None => Ok(()),
        }
    }
}

/// This is the top-level task runner that gets executed when the server is
/// booted. It continuously polls the database for new tasks with status
/// `Pending`, and will run them.
pub(crate) fn poll(conn: &Database) {
    loop {
        // Fetch all pending tasks, and set them to running in one transaction,
        // after that, we'll start running them one by one...
        let result = conn
            .transaction(|| {
                use crate::schema::tasks::dsl::*;
                tasks
                    .filter(status.eq(Status::Pending))
                    .load::<Task>(&**conn)?
                    .into_iter()
                    .map(|mut task| task.as_running(conn))
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(Into::into)
            .and_then(|tasks| {
                tasks.into_iter().try_for_each(|mut task| {
                    task.run(conn).or_else(|err| {
                        let _ = task.as_failed(conn)?;
                        Err(err)
                    })
                })
            });

        if let Err(err) = result {
            eprintln!("failed to run task: {}", err);
        }

        thread::sleep(std::time::Duration::from_millis(1000));
    }
}

/// Contains all the details needed to store a task in the database.
///
/// The fields are private, use [`NewTask::new`] to initialize this struct.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct NewTask<'a> {
    name: &'a str,
    description: Option<&'a str>,
    status: Status,
    pipeline_reference: Option<i32>,
    steps: Vec<NewTaskStep<'a>>,
}

impl<'a> NewTask<'a> {
    /// Initialize a `NewTask` struct, which can be inserted into the
    /// database using the [`NewTask#create`] method.
    pub(crate) fn new(name: &'a str, description: Option<&'a str>) -> Self {
        Self {
            name,
            description,
            status: Status::Pending,
            pipeline_reference: None,
            steps: vec![],
        }
    }

    pub(crate) fn create_from_pipeline(
        conn: &Database,
        pipeline: &'a Pipeline,
        variable_values: &[VariableValue],
    ) -> Result<Task, Box<dyn error::Error>> {
        let steps = pipeline.steps(conn)?;
        let steps = steps
            .iter()
            .map(|s| (s, variable_values))
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()?;

        let mut task = Self::new(
            &pipeline.name,
            pipeline.description.as_ref().map(String::as_ref),
        );
        task.with_pipeline_reference(pipeline.id);
        task.with_steps(steps);

        task.create(conn).map_err(Into::into)
    }

    pub(crate) fn with_pipeline_reference(&mut self, pipeline_id: i32) {
        self.pipeline_reference = Some(pipeline_id)
    }

    /// Attach zero or more steps to this pipeline.
    ///
    /// `NewPipeline` takes ownership of the steps, but you are required to
    /// call [`NewPipeline#create`] to persist the pipeline and its steps.
    ///
    /// Can be called multiple times to append more steps.
    fn with_steps(&mut self, mut steps: Vec<NewTaskStep<'a>>) {
        self.steps.append(&mut steps)
    }

    /// Persist the task into the database.
    pub(crate) fn create(self, conn: &Database) -> Result<Task, Box<dyn error::Error>> {
        use crate::schema::tasks::dsl::*;

        let mut task_name = self.name.to_owned();

        // Task names are unique over (name, pipeline_reference). If a reference
        // exists, we add a count (such as "My Task #3") to the name of the
        // pipeline, based on the total amount of tasks for that pipeline ID.
        //
        // Non-pipeline based tasks will simply return an error if their name
        // isn't unique.
        if let Some(pipeline_id) = self.pipeline_reference {
            use crate::schema::tasks::dsl::*;

            let pipeline: Pipeline = {
                use crate::schema::pipelines::dsl::*;
                pipelines.filter(id.eq(pipeline_id)).first(&**conn)
            }?;

            let total = tasks
                .filter(pipeline_reference.eq(pipeline_id))
                .count()
                .get_result::<i64>(&**conn)?;

            task_name = format!("{} #{}", pipeline.name, total + 1);
        }

        conn.transaction(|| {
            // waiting on https://github.com/diesel-rs/diesel/issues/860
            let values = (
                name.eq(&task_name),
                description.eq(&self.description),
                status.eq(self.status),
                pipeline_reference.eq(self.pipeline_reference),
            );

            let task = diesel::insert_into(tasks)
                .values(&values)
                .get_result(&**conn)?;

            self.steps
                .into_iter()
                .try_for_each(|s| s.add_to_task(conn, &task))?;

            Ok(task)
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
    use crate::resources::VariableValueInput;
    use juniper::{object, FieldResult, GraphQLInputObject, ID};

    /// Contains all the data needed to create a new `Pipeline`.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct CreateTaskFromPipelineInput {
        /// The `id` of the pipeline from which to create this task.
        #[serde(with = "juniper_serde")]
        pub(crate) pipeline_id: ID,

        /// An optional list of variable values required by the pipeline.
        ///
        /// Note that the eventual `Task` object has no concept of "variables".
        ///
        /// The provided variable values are used in-place of the templated
        /// variables in the pipeline before creating the task. The final step
        /// configs are then stored alongside the task in the database.
        pub(crate) variables: Vec<VariableValueInput>,
    }

    /// Contains all the data needed to replace templated processor
    /// configurations.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct TaskVariableInput {
        pub(crate) key: String,
        pub(crate) value: String,
    }

    #[object(Context = Database)]
    impl Task {
        /// The unique identifier for a specific task.
        fn id() -> ID {
            ID::new(self.id.to_string())
        }

        /// A unique and descriptive name of the task.
        fn name() -> &str {
            self.name.as_ref()
        }

        /// An (optional) detailed description of the functionality provided by
        /// this task.
        ///
        /// A description _might_ be markdown formatted, and should be parsed
        /// accordingly by the client.
        fn description() -> Option<&str> {
            self.description.as_ref().map(String::as_ref)
        }

        /// The status of the task.
        fn status() -> Status {
            self.status
        }

        /// The steps belonging to the task.
        ///
        /// This field can return `null`, but _only_ if a database error
        /// prevents the data from being retrieved.
        ///
        /// If no steps are attached to a task, an empty array is returned
        /// instead.
        ///
        /// If a `null` value is returned, it is up to the client to decide the
        /// best course of action. The following actions are advised, sorted by
        /// preference:
        ///
        /// 1. continue execution if the information is not critical to success,
        /// 2. retry the request to try and get the relevant information,
        /// 3. disable parts of the application reliant on the information,
        /// 4. show a global error, and ask the user to retry.
        fn steps(context: &Database) -> FieldResult<Option<Vec<TaskStep>>> {
            self.steps(context).map(Some).map_err(Into::into)
        }

        /// The pipeline from which the task was created.
        ///
        /// A task _can_ but _does not have to_ be created from an existing
        /// pipeline.
        ///
        /// If a task was created from a pipeline, this will return the relevant
        /// `Pipeline` object.
        ///
        /// If a task was not created from an existing pipeline, this will
        /// return `null`.
        ///
        /// If a pipeline has been removed since the task was created, this will
        /// also return `null`.
        ///
        /// There is also the possibility of this task being created from a
        /// pipeline, but the database lookup to fetch the pipeline details
        /// failed. In this case, the value will also be `null`, but an `errors`
        /// object will be attached to the result, explaining the problem that
        /// occurred.
        ///
        /// If a `null` value is returned as the result of a lookup error, it is
        /// up to the client to decide the best course of action. The following
        /// actions are advised, sorted by preference:
        ///
        /// 1. continue execution if the information is not critical to success,
        /// 2. retry the request to try and get the relevant information,
        /// 3. disable parts of the application reliant on the information,
        /// 4. show a global error, and ask the user to retry.
        fn pipeline(context: &Database) -> FieldResult<Option<Pipeline>> {
            self.pipeline(context).map_err(Into::into)
        }
    }
}
