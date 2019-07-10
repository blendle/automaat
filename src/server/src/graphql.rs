use crate::resources::{
    variable, CreateJobFromTaskInput, CreateTaskInput, Job, NewJob, NewTask, SearchTaskInput, Task,
    VariableValue,
};
use crate::Database;
use diesel::prelude::*;
use juniper::{object, Context, FieldError, FieldResult, RootNode, ID};
use std::convert::TryFrom;

impl Context for Database {}

pub(crate) type Schema = RootNode<'static, QueryRoot, MutationRoot>;
pub(crate) struct QueryRoot;
pub(crate) struct MutationRoot;

#[object(Context = Database)]
impl QueryRoot {
    /// Return a list of tasks.
    ///
    /// You can optionally filter the returned set of tasks by providing the
    /// `SearchTaskInput` value.
    fn tasks(context: &Database, search: Option<SearchTaskInput>) -> FieldResult<Vec<Task>> {
        use crate::schema::tasks::dsl::*;
        let conn = &context.0;

        let mut query = tasks.order(id).into_boxed();

        if let Some(search) = &search {
            if let Some(search_name) = &search.name {
                query = query.filter(name.ilike(format!("%{}%", search_name)));
            };

            if let Some(search_description) = &search.description {
                query = query.or_filter(description.ilike(format!("%{}%", search_description)));
            };
        };

        query.load(conn).map_err(Into::into)
    }

    /// Return a list of jobs.
    fn jobs(context: &Database) -> FieldResult<Vec<Job>> {
        use crate::schema::jobs::dsl::*;

        jobs.order(id).load(&**context).map_err(Into::into)
    }

    /// Return a single task, based on the task ID.
    ///
    /// This query can return `null` if no task is found matching the
    /// provided ID.
    fn task(context: &Database, id: ID) -> FieldResult<Option<Task>> {
        use crate::schema::tasks::dsl::{id as pid, tasks};

        tasks
            .filter(pid.eq(id.parse::<i32>()?))
            .first(&**context)
            .optional()
            .map_err(Into::into)
    }

    /// Return a single job, based on the job ID.
    ///
    /// This query can return `null` if no job is found matching the
    /// provided ID.
    fn job(context: &Database, id: ID) -> FieldResult<Option<Job>> {
        use crate::schema::jobs::dsl::{id as tid, jobs};

        jobs.filter(tid.eq(id.parse::<i32>()?))
            .first(&**context)
            .optional()
            .map_err(Into::into)
    }
}

#[object(Context = Database)]
impl MutationRoot {
    /// Create a new task.
    fn createTask(context: &Database, task: CreateTaskInput) -> FieldResult<Task> {
        NewTask::try_from(&task)?
            .create(context)
            .map_err(Into::into)
    }

    /// Create a job from an existing task ID.
    ///
    /// Once the job is created, it will be scheduled to run immediately.
    fn createJobFromTask(context: &Database, job: CreateJobFromTaskInput) -> FieldResult<Job> {
        let task: Task = {
            use crate::schema::tasks::dsl::*;

            tasks
                .filter(id.eq(job.task_id.parse::<i32>()?))
                .first(&**context)
        }?;

        let variable_values = job
            .variables
            .into_iter()
            .map(Into::into)
            .collect::<Vec<VariableValue>>();

        let task_variables = task.variables(context)?;

        if let Some(variables) = variable::missing_values(&task_variables, &variable_values) {
            let keys = variables.iter().map(|v| v.key.as_str()).collect::<Vec<_>>();

            return Err(format!(r#"missing variable values: {}"#, keys.join(", ")).into());
        }

        if let Some(results) =
            variable::selection_constraint_mismatch(&task_variables, &variable_values)
        {
            let variable = results[0].0;
            let value = results[0].1;

            // TODO: turn this into a structured error object, so we can expose
            // all the invalid variables at once.
            return Err(format!(
                r#"invalid variable value: "{}", must be one of: {:?}"#,
                value.key,
                variable
                    .selection_constraint
                    .as_ref()
                    .unwrap_or(&vec![])
                    .join(", ")
            )
            .into());
        }

        let mut new_job = NewJob::create_from_task(context, &task, &variable_values)
            .map_err(Into::<FieldError>::into)?;

        // TODO: when we have scheduling, we probably want this to be optional,
        // so that a job isn't always scheduled instantly.
        new_job.enqueue(context).map_err(Into::into)
    }
}
