//! A [`Job`] is a job that is scheduled to be ran, or already ran in the past.
//!
//! It is similar to a [`Task`], but a task represents a set of steps
//! that _can be ran_ by providing a set of variables, whereas a job represents
//! a set of steps that are _ready to run_ and have their variables swapped for
//! real values.

use crate::resources::{JobStep, JobStepStatus, JobVariable, NewJobStep, NewJobVariable, Task};
use crate::schema::jobs;
use crate::{server::RequestState, ENCRYPTION_SECRET};
use automaat_core::Context;
use diesel::prelude::*;
use juniper::GraphQLEnum;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::{Into, TryInto};
use std::error::Error;

pub(crate) mod step;
pub(crate) mod variable;

/// The status of the [`Job`].
#[derive(Clone, Copy, Debug, Serialize, Deserialize, GraphQLEnum, DbEnum)]
#[PgType = "JobStatus"]
#[graphql(name = "JobStatus")]
pub(crate) enum Status {
    /// The job is scheduled for execution in the future.
    Scheduled,

    /// The job is ready to be executed and waiting for the scheduler to pick
    /// it up.
    Pending,

    /// The job is currently running its steps one by one.
    Running,

    /// One of the job steps failed, resulting in the job itself to fail.
    Failed,

    /// The job was cancelled.
    Cancelled,

    /// All job steps ran successfully.
    Ok,
}

impl From<JobStepStatus> for Status {
    fn from(status: JobStepStatus) -> Self {
        use Status::*;

        match status {
            JobStepStatus::Initialized => Scheduled,
            JobStepStatus::Pending => Pending,
            JobStepStatus::Running => Running,
            JobStepStatus::Failed => Failed,
            JobStepStatus::Cancelled => Cancelled,
            JobStepStatus::Ok => Ok,
        }
    }
}

#[derive(
    Clone, Debug, Deserialize, Serialize, AsChangeset, Associations, Identifiable, Queryable,
)]
#[belongs_to(Task, foreign_key = "task_reference")]
#[table_name = "jobs"]
/// The model representing a job stored in the database.
pub(crate) struct Job {
    pub(crate) id: i32,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) status: Status,

    // This is a weak reference, meaning that tasks can be removed, which
    // breaks the link between a job and the task it was created from. This
    // is acceptable, it just means the UI can't link back to the task.
    //
    // Similarly, a job can be created separately from a task, in which case
    // this field is also `None`.
    pub(crate) task_reference: Option<i32>,
}

impl Job {
    pub(crate) fn find_next_unlocked_pending(conn: &PgConnection) -> QueryResult<Option<Self>> {
        jobs::table
            .filter(jobs::status.eq(Status::Pending))
            .order(jobs::id)
            .for_update()
            .skip_locked()
            .first(conn)
            .optional()
    }

    pub(crate) fn as_running(&mut self, conn: &PgConnection) -> QueryResult<Self> {
        self.status = Status::Running;
        self.save_changes(conn)
    }

    pub(crate) fn as_failed(&mut self, conn: &PgConnection) -> QueryResult<Self> {
        self.status = Status::Failed;
        self.save_changes(conn)
    }

    pub(crate) fn task(&self, conn: &PgConnection) -> QueryResult<Option<Task>> {
        use crate::schema::tasks::dsl::*;

        match self.task_reference {
            None => Ok(None),
            Some(task_id) => tasks.filter(id.eq(task_id)).first(conn).optional(),
        }
    }

    pub(crate) fn steps(&self, conn: &PgConnection) -> QueryResult<Vec<JobStep>> {
        use crate::schema::job_steps::dsl::*;

        JobStep::belonging_to(self).order(position.asc()).load(conn)
    }

    pub(crate) fn variables(&self, conn: &PgConnection) -> QueryResult<Vec<JobVariable>> {
        use crate::schema::job_variables::dsl::*;

        let secret = ENCRYPTION_SECRET.as_str();
        JobVariable::belonging_to(self)
            .select((id, key, variable::pgp_sym_decrypt(value, secret), job_id))
            .load(conn)
    }

    // TODO: implement some kind of `JobRunner`, that has a reference to
    // &Database, and then impl `Drop` so that if the runner stops, we can check
    // the result, and update the database based on the final status.
    pub(crate) fn run(&self, conn: &PgConnection) -> Result<(), Box<dyn Error>> {
        use crate::schema::jobs::dsl::*;

        let output: HashMap<String, String> = HashMap::default();
        let context = Context::new()?;
        let mut steps = self.steps(conn)?;

        let _ = steps
            .iter_mut()
            .try_fold(output, |output, step| step.run(conn, &context, output))?;

        match steps.last() {
            Some(step) => diesel::update(self)
                .set(status.eq(Status::from(step.status)))
                .execute(conn)
                .map(|_| ())
                .map_err(Into::into),
            None => Ok(()),
        }
    }
}

/// Contains all the details needed to store a job in the database.
///
/// The fields are private, use [`NewJob::new`] to initialize this struct.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct NewJob<'a> {
    name: &'a str,
    description: Option<&'a str>,
    status: Status,
    task_reference: Option<i32>,
    steps: Vec<NewJobStep<'a>>,
    variables: Vec<NewJobVariable<'a>>,
}

impl<'a> NewJob<'a> {
    /// Initialize a `NewJob` struct, which can be inserted into the
    /// database using the [`NewJob#create`] method.
    pub(crate) fn new(name: &'a str, description: Option<&'a str>) -> Self {
        Self {
            name,
            description,
            status: Status::Pending,
            task_reference: None,
            steps: vec![],
            variables: vec![],
        }
    }

    pub(crate) fn create_from_task(
        conn: &PgConnection,
        task: &'a Task,
        variables: Vec<NewJobVariable<'a>>,
    ) -> Result<Job, Box<dyn Error>> {
        let steps = task.steps(conn)?;
        let steps = steps
            .iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()?;

        let mut job = Self::new(&task.name, task.description.as_ref().map(String::as_ref));
        job.with_task_reference(task.id);
        job.with_steps(steps);
        job.with_variables(variables);

        job.create(conn).map_err(Into::into)
    }

    pub(crate) fn with_task_reference(&mut self, task_id: i32) {
        self.task_reference = Some(task_id)
    }

    /// Attach zero or more steps to this job.
    ///
    /// `NewJob` takes ownership of the steps, but you are required to
    /// call [`NewJob#create`] to persist the job and its steps.
    ///
    /// Can be called multiple times to append more steps.
    fn with_steps(&mut self, mut steps: Vec<NewJobStep<'a>>) {
        self.steps.append(&mut steps)
    }

    /// Attach zero or more variables to this job.
    ///
    /// `NewJob` takes ownership of the variables, but you are required to
    /// call [`NewJob#create`] to persist the job and its variables.
    ///
    /// Can be called multiple times to append more variables.
    fn with_variables(&mut self, mut variables: Vec<NewJobVariable<'a>>) {
        self.variables.append(&mut variables)
    }

    /// Persist the job into the database.
    pub(crate) fn create(self, conn: &PgConnection) -> Result<Job, Box<dyn Error>> {
        use crate::schema::jobs::dsl::*;

        let mut job_name = self.name.to_owned();

        // Job names are unique over (name, task_reference). If a reference
        // exists, we add a count (such as "My Job #3") to the name of the
        // job, based on the total amount of jobs for that task ID.
        //
        // Non-task based jobs will simply return an error if their name
        // isn't unique.
        if let Some(task_id) = self.task_reference {
            use crate::schema::jobs::dsl::*;

            let task: Task = {
                use crate::schema::tasks::dsl::*;
                tasks.filter(id.eq(task_id)).first(conn)
            }?;

            let total = jobs
                .filter(task_reference.eq(task_id))
                .count()
                .get_result::<i64>(conn)?;

            job_name = format!("{} #{}", task.name, total + 1);

            // If we're dealing with a job created from a task, we also need to
            // validate that the task variables are all present.
            self.validate_task_variables(&task, conn)?;
        }

        conn.transaction(|| {
            // waiting on https://github.com/diesel-rs/diesel/issues/860
            let values = (
                name.eq(&job_name),
                description.eq(&self.description),
                status.eq(self.status),
                task_reference.eq(self.task_reference),
            );

            let job = diesel::insert_into(jobs).values(&values).get_result(conn)?;

            self.variables
                .into_iter()
                .try_for_each(|s| s.add_to_job(conn, &job))?;

            self.steps
                .into_iter()
                .try_for_each(|s| s.add_to_job(conn, &job))?;

            Ok(job)
        })
    }

    /// Validate that all required variables are present.
    ///
    /// This is only relevant if the job is created from an existing task, in
    /// which case the task can have any number of variables, and the provided
    /// job variables should match those.
    fn validate_task_variables(
        &self,
        task: &Task,
        conn: &PgConnection,
    ) -> Result<(), Box<dyn Error>> {
        let task_variables = task.variables(conn)?;

        let missing = task_variables
            .iter()
            .filter_map(|variable| {
                self.variables
                    .iter()
                    .find(|v| v.key() == variable.key)
                    .map_or(Some(variable.key.as_str()), |_| None)
            })
            .collect::<Vec<_>>();

        if missing.is_empty() {
            return Ok(());
        }

        Err(format!("missing variable values: {}", missing.join(", ")).into())
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
    use crate::resources::JobVariableInput;
    use juniper::{object, FieldResult, GraphQLInputObject, ID};

    /// Contains all the data needed to create a new `Task`.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct CreateJobFromTaskInput {
        /// The `id` of the task from which to create this job.
        pub(crate) task_id: ID,

        /// An optional list of variable values required by the task.
        ///
        /// Note that the eventual `Job` object has no concept of "variables".
        ///
        /// The provided variable values are used in-place of the templated
        /// variables in the task before creating the job. The final step
        /// configs are then stored alongside the job in the database.
        pub(crate) variables: Vec<JobVariableInput>,
    }

    #[object(Context = RequestState)]
    impl Job {
        /// The unique identifier for a specific job.
        fn id() -> ID {
            ID::new(self.id.to_string())
        }

        /// A unique and descriptive name of the job.
        fn name() -> &str {
            self.name.as_ref()
        }

        /// An (optional) detailed description of the functionality provided by
        /// this job.
        ///
        /// A description _might_ be markdown formatted, and should be parsed
        /// accordingly by the client.
        fn description() -> Option<&str> {
            self.description.as_ref().map(String::as_ref)
        }

        /// The status of the job.
        fn status() -> Status {
            self.status
        }

        /// The steps belonging to the job.
        ///
        /// This field can return `null`, but _only_ if a database error
        /// prevents the data from being retrieved.
        ///
        /// If no steps are attached to a job, an empty array is returned
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
        fn steps(context: &RequestState) -> FieldResult<Option<Vec<JobStep>>> {
            self.steps(&context.conn).map(Some).map_err(Into::into)
        }

        /// The task from which the job was created.
        ///
        /// A job _can_ but _does not have to_ be created from an existing
        /// task.
        ///
        /// If a job was created from a task, this will return the relevant
        /// `Task` object.
        ///
        /// If a job was not created from an existing task, this will
        /// return `null`.
        ///
        /// If a task has been removed since the job was created, this will
        /// also return `null`.
        ///
        /// There is also the possibility of this job being created from a
        /// task, but the database lookup to fetch the task details
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
        fn task(context: &RequestState) -> FieldResult<Option<Task>> {
            self.task(&context.conn).map_err(Into::into)
        }
    }
}
