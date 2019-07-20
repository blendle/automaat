use crate::models::NewGlobalVariable;
use crate::resources::{
    CreateJobFromTaskInput, CreateTaskInput, GlobalVariableInput, Job, NewJob, NewJobVariable,
    NewTask, OnConflict, SearchTaskInput, Task,
};
use crate::schema::*;
use crate::State;
use diesel::prelude::*;
use juniper::{object, Context, FieldResult, RootNode, ID};
use std::convert::TryFrom;

impl Context for State {}

pub(crate) type Schema = RootNode<'static, QueryRoot, MutationRoot>;
pub(crate) struct QueryRoot;
pub(crate) struct MutationRoot;

#[object(Context = State)]
impl QueryRoot {
    /// Return a list of tasks.
    ///
    /// You can optionally filter the returned set of tasks by providing the
    /// `SearchTaskInput` value.
    fn tasks(context: &State, search: Option<SearchTaskInput>) -> FieldResult<Vec<Task>> {
        let conn = context.pool.get()?;

        let mut query = tasks::table.order(tasks::id).into_boxed();

        if let Some(search) = &search {
            if let Some(search_name) = &search.name {
                query = query.filter(tasks::name.ilike(format!("%{}%", search_name)));
            };

            if let Some(search_description) = &search.description {
                query =
                    query.or_filter(tasks::description.ilike(format!("%{}%", search_description)));
            };
        };

        query.load(&conn).map_err(Into::into)
    }

    /// Return a list of jobs.
    fn jobs(context: &State) -> FieldResult<Vec<Job>> {
        let conn = context.pool.get()?;

        jobs::table.order(jobs::id).load(&conn).map_err(Into::into)
    }

    /// Return a single task, based on the task ID.
    ///
    /// This query can return `null` if no task is found matching the
    /// provided ID.
    fn task(context: &State, id: ID) -> FieldResult<Option<Task>> {
        let conn = context.pool.get()?;

        tasks::table
            .filter(tasks::id.eq(id.parse::<i32>()?))
            .first(&conn)
            .optional()
            .map_err(Into::into)
    }

    /// Return a single job, based on the job ID.
    ///
    /// This query can return `null` if no job is found matching the
    /// provided ID.
    fn job(context: &State, id: ID) -> FieldResult<Option<Job>> {
        let conn = context.pool.get()?;

        jobs::table
            .filter(jobs::id.eq(id.parse::<i32>()?))
            .first(&conn)
            .optional()
            .map_err(Into::into)
    }
}

#[object(Context = State)]
impl MutationRoot {
    /// Create a new task.
    fn createTask(context: &State, task: CreateTaskInput) -> FieldResult<Task> {
        let conn = context.pool.get()?;

        NewTask::try_from(&task)?.create(&conn).map_err(Into::into)
    }

    /// Create a job from an existing task ID.
    ///
    /// Once the job is created, it will be scheduled to run immediately.
    fn createJobFromTask(context: &State, job: CreateJobFromTaskInput) -> FieldResult<Job> {
        let conn = context.pool.get()?;

        let task = tasks::table
            .filter(tasks::id.eq(job.task_id.parse::<i32>()?))
            .first(&conn)?;

        let variables = job
            .variables
            .iter()
            .map(Into::into)
            .collect::<Vec<NewJobVariable<'_>>>();

        NewJob::create_from_task(&conn, &task, variables).map_err(Into::into)
    }

    /// Create a new global variable.
    ///
    /// Global variables can be accessed in task templates, without having to
    /// supply their values on runtime.
    ///
    /// By default, this mutation will return an error if the variable key
    /// already exists. If you want to override an existing key, set the
    /// `onConflict` key to `UPDATE`.
    fn createGlobalVariable(context: &State, variable: GlobalVariableInput) -> FieldResult<bool> {
        use OnConflict::*;
        let conn = context.pool.get()?;

        let global_variable = NewGlobalVariable::from(&variable);
        let global_variable = match &variable.on_conflict.as_ref().unwrap_or(&Abort) {
            Abort => global_variable.create(&conn),
            Update => global_variable.create_or_update(&conn),
        };

        global_variable.map(|_| true).map_err(Into::into)
    }
}
