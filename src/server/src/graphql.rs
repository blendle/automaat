use crate::models::{NewGlobalVariable, NewSession, Session};
use crate::resources::{
    CreateJobFromTaskInput, CreateSessionInput, CreateTaskInput, GlobalVariableInput, Job, NewJob,
    NewJobVariable, NewTask, OnConflict, SearchTaskInput, Task, UpdatePrivilegesInput,
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
        let name = search
            .as_ref()
            .and_then(|s| s.name.as_ref().map(String::as_str));

        let description = search
            .as_ref()
            .and_then(|s| s.description.as_ref().map(String::as_str));

        Task::search(name, description, &context.conn).map_err(Into::into)
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

    /// Get details of the current session, if any.
    fn session(context: &RequestState) -> Option<&Session> {
        context.session.as_ref()
    }
}

#[object(Context = RequestState)]
impl MutationRoot {
    /// Create a new task.
    ///
    /// # Privileges
    ///
    /// This mutation requires the `mutation_create_task` privilege to be set
    /// for the provided session.
    fn createTask(context: &RequestState, task: CreateTaskInput) -> FieldResult<Task> {
        authorization_guard(&["mutation_create_task"], &context.session)?;

        let new_task = NewTask::try_from(&task)?;
        match &task.on_conflict.as_ref().unwrap_or(&OnConflict::Abort) {
            OnConflict::Abort => new_task.create(&context.conn),
            OnConflict::Update => new_task.create_or_update(&context.conn),
        }
        .map_err(Into::into)
    }

    /// Create a job from an existing task ID.
    ///
    /// Once the job is created, it will be scheduled to run immediately.
    ///
    /// # Privileges
    ///
    /// This mutation supports both creating jobs from unauthenticated sessions,
    /// and authenticated ones.
    ///
    /// If a task has no labels attached, then anyone can create jobs for that
    /// task. If a task has one or more labels, then an authenticated session
    /// must exist, and at least one privilege must match one of the task
    /// labels.
    fn createJobFromTask(context: &RequestState, job: CreateJobFromTaskInput) -> FieldResult<Job> {
        let task: Task = tasks::table
            .filter(tasks::id.eq(job.task_id.parse::<i32>()?))
            .first(&context.conn)?;

        authorization_guard(
            &task.labels.iter().map(String::as_str).collect::<Vec<_>>(),
            &context.session,
        )?;

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
    ///
    /// # Privileges
    ///
    /// This mutation requires the `mutation_create_global_variable` privilege
    /// to be set for the provided session.
    fn createGlobalVariable(
        context: &RequestState,
        variable: GlobalVariableInput,
    ) -> FieldResult<bool> {
        use OnConflict::*;

        authorization_guard(&["mutation_create_global_variable"], &context.session)?;

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
    /// # Privileges
    ///
    /// This mutation requires the `mutation_create_session` privilege to
    /// be set for the provided session.
    fn createSession(context: &RequestState, session: CreateSessionInput) -> FieldResult<String> {
        authorization_guard(&["mutation_create_session"], &context.session)?;

        NewSession::from(&session)
            .create(&context.conn)
            .map(|s| s.token.to_string())
            .map_err(Into::into)
    }

    /// Update the set of privileges of an existing session.
    ///
    /// The privileges defined in this update will be set as-is as the new
    /// session privileges. This means that any privileges existing before, but
    /// missing in this update will be removed.
    ///
    /// In other words, if you want to _add_ an extra privilege to an existing
    /// set of privileges, you will first have to fetch all privileges of the
    /// session, add the new privilege, and then run this mutation.
    ///
    /// # Privileges
    ///
    /// This mutation requires the `mutation_update_privileges` privilege to be
    /// set for the provided session.
    fn updatePrivileges(
        context: &RequestState,
        privileges: UpdatePrivilegesInput,
    ) -> FieldResult<Session> {
        authorization_guard(&["mutation_update_privileges"], &context.session)?;

        let session = sessions::table.filter(sessions::id.eq(privileges.id.parse::<i32>()?));

        diesel::update(session)
            .set(sessions::privileges.eq(privileges.privileges))
            .get_result(&context.conn)
            .map_err(Into::into)
    }
}

/// A guard function that returns an error if none of the defined labels are
/// present in the provided session privileges.
///
/// If no labels are provided, the request is considered to be authorized.
///
/// If no session is provided, its privileges are considered to be empty.
fn authorization_guard(labels: &[&str], session: &Option<Session>) -> FieldResult<()> {
    if labels.is_empty() {
        return Ok(());
    }

    for label in labels {
        if session
            .as_ref()
            .map_or(&vec![], |s| &s.privileges)
            .iter()
            .any(|x| x == label)
        {
            return Ok(());
        }
    }

    Err("Unauthorized".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn stub_session(privileges: &[&str]) -> Option<Session> {
        Some(Session {
            id: 0,
            token: Uuid::new_v4(),
            privileges: privileges.iter().map(|p| p.to_string()).collect::<Vec<_>>(),
        })
    }

    #[test]
    fn test_authorization_guard_empty_labels() {
        let session = stub_session(&[]);
        authorization_guard(&[], &session).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_authorization_guard_no_session() {
        authorization_guard(&["required"], &None).unwrap();
    }

    #[test]
    fn test_authorization_matching_privilege() {
        let session = stub_session(&["required"]);
        authorization_guard(&["required"], &session).unwrap();
    }

    #[test]
    fn test_authorization_one_matching_guard() {
        let session = stub_session(&["one"]);
        authorization_guard(&["one", "two"], &session).unwrap();
    }

    #[test]
    fn test_authorization_one_matching_privilege() {
        let session = stub_session(&["two", "three"]);
        authorization_guard(&["two"], &session).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_authorization_no_matching_privilege() {
        let session = stub_session(&["three"]);
        authorization_guard(&["one", "two"], &session).unwrap();
    }
}
