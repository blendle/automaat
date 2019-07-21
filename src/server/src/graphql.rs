use crate::models::{NewGlobalVariable, Session};
use crate::resources::{
    CreateJobFromTaskInput, CreateTaskInput, GlobalVariableInput, Job, NewJob, NewJobVariable,
    NewTask, OnConflict, SearchTaskInput, Task,
};
use crate::schema::*;
use crate::server::RequestState;
use diesel::prelude::*;
use juniper::{object, Context, FieldResult, RootNode, ID};
use std::convert::TryFrom;

impl Context for RequestState {}

pub(crate) type Schema = RootNode<'static, QueryRoot, MutationRoot>;
pub(crate) struct QueryRoot;
pub(crate) struct MutationRoot;

#[object(Context = RequestState)]
impl QueryRoot {
    /// Return a list of tasks.
    ///
    /// You can optionally filter the returned set of tasks by providing the
    /// `SearchTaskInput` value.
    fn tasks(context: &RequestState, search: Option<SearchTaskInput>) -> FieldResult<Vec<Task>> {
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

        query.load(&context.conn).map_err(Into::into)
    }

    /// Return a list of jobs.
    fn jobs(context: &RequestState) -> FieldResult<Vec<Job>> {
        jobs::table
            .order(jobs::id)
            .load(&context.conn)
            .map_err(Into::into)
    }

    /// Return a single task, based on the task ID.
    ///
    /// This query can return `null` if no task is found matching the
    /// provided ID.
    fn task(context: &RequestState, id: ID) -> FieldResult<Option<Task>> {
        tasks::table
            .filter(tasks::id.eq(id.parse::<i32>()?))
            .first(&context.conn)
            .optional()
            .map_err(Into::into)
    }

    /// Return a single job, based on the job ID.
    ///
    /// This query can return `null` if no job is found matching the
    /// provided ID.
    fn job(context: &RequestState, id: ID) -> FieldResult<Option<Job>> {
        jobs::table
            .filter(jobs::id.eq(id.parse::<i32>()?))
            .first(&context.conn)
            .optional()
            .map_err(Into::into)
    }
}

#[object(Context = RequestState)]
impl MutationRoot {
    /// Create a new task.
    fn createTask(context: &RequestState, task: CreateTaskInput) -> FieldResult<Task> {
        NewTask::try_from(&task)?
            .create(&context.conn)
            .map_err(Into::into)
    }

    /// Create a job from an existing task ID.
    ///
    /// Once the job is created, it will be scheduled to run immediately.
    fn createJobFromTask(context: &RequestState, job: CreateJobFromTaskInput) -> FieldResult<Job> {
        let task = tasks::table
            .filter(tasks::id.eq(job.task_id.parse::<i32>()?))
            .first(&context.conn)?;

        let variables = job
            .variables
            .iter()
            .map(Into::into)
            .collect::<Vec<NewJobVariable<'_>>>();

        NewJob::create_from_task(&context.conn, &task, variables).map_err(Into::into)
    }

    /// Create a new global variable.
    ///
    /// Global variables can be accessed in task templates, without having to
    /// supply their values on runtime.
    ///
    /// By default, this mutation will return an error if the variable key
    /// already exists. If you want to override an existing key, set the
    /// `onConflict` key to `UPDATE`.
    fn createGlobalVariable(
        context: &RequestState,
        variable: GlobalVariableInput,
    ) -> FieldResult<bool> {
        use OnConflict::*;

        let global_variable = NewGlobalVariable::from(&variable);
        let global_variable = match &variable.on_conflict.as_ref().unwrap_or(&Abort) {
            Abort => global_variable.create(&context.conn),
            Update => global_variable.create_or_update(&context.conn),
        };

        global_variable.map(|_| true).map_err(Into::into)
    }

    /// Create a new session.
    ///
    /// The returned session token can be used to authenticate with the GraphQL
    /// API by using the `Authorization` header.
    ///
    /// In the future, this token can also be used to store user-related
    /// preferences.
    fn createSession(context: &RequestState) -> FieldResult<String> {
        Session::create(&context.conn)
            .map(|s| s.token.to_string())
            .map_err(Into::into)
    }
}
