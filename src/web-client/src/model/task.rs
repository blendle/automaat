//! A task that can be run by starting a job.

use crate::graphql::fetch_task_details::{FetchTaskDetailsTask, FetchTaskDetailsTaskVariables};
use crate::graphql::search_tasks::SearchTasksTasks;
use crate::model::session::{AccessMode, Session};
use crate::model::{job, tasks, variable};
use dodrio::{RootRender, VdomWeak};
use futures::future::Future;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

/// The task model.
#[derive(Clone, Debug)]
pub(crate) struct Task {
    /// A set of known jobs that were created from this task. This is not the
    /// full set known by the server, but the set of jobs started during the
    /// active session of the application.
    pub(crate) jobs: Vec<job::Job>,

    /// The index of the job that is currently marked as "active" for this task.
    ///
    /// When a job is active, it influences the state of the task in the UI,
    /// such as if the UI is in a loading state if the job is still running, or
    /// if the result of the task is shown in the UI once the task is finished.
    active_job_idx: Option<usize>,

    /// The details object returned by the GraphQL server.
    ///
    /// The values of the object are used internally to expose the relevant
    /// details via the designated methods.
    details: SearchTasksTasks,

    /// The variable objects returned by the GraphQL server.
    ///
    /// The values of the objects are used internally to expose the relevant
    /// details via the designated methods.
    ///
    /// This is an optional value, because the variables aren't fetched when
    /// searching for tasks, but are only returned when the task is opened in
    /// the UI. From that point on, the variables are cached and won't have to
    /// be fetched again during the active application session.
    variables: Option<Vec<FetchTaskDetailsTaskVariables>>,

    /// Controls whether or not to show the login field when the task is active.
    pub(crate) show_login: bool,
}

impl Task {
    /// The ID of the task as set by the server.
    pub(crate) fn id(&self) -> Id {
        Id(self.details.id.clone())
    }

    /// The name of the task.
    pub(crate) fn name(&self) -> &str {
        self.details.name.as_ref()
    }

    /// The description of the task.
    ///
    /// A task description is not required by the server, but in our case, we
    /// can return an empty string if there is no description defined.
    pub(crate) fn description(&self) -> &str {
        self.details.description.as_ref().map_or("", String::as_str)
    }

    /// The labels attached to the task.
    ///
    /// Task labels are used to match session privileges against. If a task has
    /// one or more labels, then the session is expected to match at least one
    /// privilege to those labels in order to be able to run the task.
    pub(crate) fn labels(&self) -> Vec<&str> {
        self.details
            .labels
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    }

    /// The optional set of variables.
    ///
    /// This returns `None` if the variables haven't been fetched from the
    /// server yet.
    pub(crate) fn variables(&self) -> Option<Vec<variable::Variable<'_>>> {
        match &self.variables {
            None => None,
            Some(variables) => Some(variables.iter().map(Into::into).collect()),
        }
    }

    /// Determine if a session is allowed to run a task.
    pub(crate) fn run_access_mode(&self, session: &Option<Session>) -> AccessMode {
        // A task without labels can be run by anyone with access to the
        // client, both unauthenticated and authenticated.
        if self.labels().is_empty() {
            return AccessMode::Ok;
        }

        // If the task has any labels defined, and no authenticated session
        // exists, the job cannot be run at this point without first
        // authenticating.
        if session.is_none() {
            return AccessMode::Unauthenticated;
        }

        // If any of the task labels are defined in the list of session
        // privileges, this session is allowed to run the task.
        for label in &self.labels() {
            if session
                .as_ref()
                .map_or(&vec![], |s| &s.privileges)
                .iter()
                .any(|x| x == label)
            {
                return AccessMode::Ok;
            }
        }

        // Once here, the current authenticated session lacks sufficient
        // privileges. The only way to run this task is to increase the session
        // privileges.
        AccessMode::Unauthorized
    }

    /// Provide a job to the task, which will be cached, and marked as the
    /// "active job" (meaning the last job being processed by the server).
    pub(crate) fn activate_job(&mut self, job: job::Job) {
        self.jobs.push(job);
        self.active_job_idx = Some(self.jobs.len() - 1);
    }

    /// Take the latest job added to the task (if any), and marks it as active.
    pub(crate) fn activate_last_job(&mut self) {
        if self.jobs.is_empty() {
            return;
        }

        self.active_job_idx = Some(self.jobs.len() - 1)
    }

    /// Hide the login view and unset any non-running active job as inactive,
    /// but keep the job around in the cache.
    ///
    /// If the job is still running, the job is kept active, so that its
    /// progress is shown whenever the task is opened again.
    pub(crate) fn deactivate(&mut self) {
        self.show_login = false;

        if let Some(job) = self.active_job() {
            if !job.is_running() {
                self.active_job_idx = None
            }
        }
    }

    /// Get the job that is currently marked as "active", if any.
    pub(crate) fn active_job(&self) -> Option<&job::Job> {
        match self.active_job_idx {
            None => None,
            Some(idx) => self.jobs.get(idx),
        }
    }

    /// Get all finished jobs (failed or succeeded)
    pub(crate) fn finished_jobs(&self) -> Vec<&job::Job> {
        self.jobs.iter().filter(|j| j.is_completed()).collect()
    }
}

impl From<SearchTasksTasks> for Task {
    fn from(details: SearchTasksTasks) -> Self {
        Self {
            details,
            active_job_idx: None,
            variables: None,
            jobs: vec![],
            show_login: false,
        }
    }
}

impl<'a> From<variable::ValueAdvertiser<'a>> for Task {
    fn from(input: variable::ValueAdvertiser<'a>) -> Self {
        Self {
            details: SearchTasksTasks {
                id: input.task_id.to_owned().to_string(),
                name: input.name.to_owned(),
                description: input.description.map(str::to_owned),
                labels: vec![],
            },
            active_job_idx: None,
            variables: None,
            jobs: vec![],
            show_login: false,
        }
    }
}

impl From<FetchTaskDetailsTask> for Vec<Task> {
    fn from(input: FetchTaskDetailsTask) -> Self {
        let mut tasks: Self = input
            .variables
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .flat_map(|v| &v.value_advertisers)
            .map(Into::<variable::ValueAdvertiser<'_>>::into)
            .map(Into::into)
            .collect();

        let details = SearchTasksTasks {
            id: input.id.clone(),
            name: input.name.clone(),
            description: input.description.clone(),
            labels: input.labels,
        };

        let task = Task {
            details,
            active_job_idx: None,
            variables: input.variables,
            jobs: vec![],
            show_login: false,
        };

        tasks.push(task);
        tasks
    }
}

/// The ID of the task, as provided by the server.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct Id(String);

impl Deref for Id {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl Id {
    /// Create a new task ID, based on a string.
    pub(crate) const fn new(id: String) -> Self {
        Self(id)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl From<String> for Id {
    fn from(id: String) -> Self {
        Self(id)
    }
}

/// The actions a controller has to implement to bridge between the UI and the
/// model.
pub(crate) trait Actions {
    /// Set a given task as the active task, allowing the UI to show the task
    /// details.
    fn activate_task(
        root: &mut dyn RootRender,
        vdom: VdomWeak,
        id: Id,
    ) -> Box<dyn Future<Item = (), Error = ()>>;

    /// Archives all active jobs of the active task, removes the active flag
    /// from the task, and redirects the UI to the home page.
    fn close_active_task(root: &mut dyn RootRender, vdom: VdomWeak);

    /// Gather the relevant task variables, and ask the server to run the task.
    ///
    /// This function returns as soon as the server signals the task is queued.
    ///
    /// The `JobId` of the job running the task is returned. Use the
    /// `Job::status` function to know when a task has run to completion.
    fn run(
        root: &mut dyn RootRender,
        vdom: VdomWeak,
        id: Id,
        variables: HashMap<String, String>,
    ) -> Box<dyn Future<Item = job::RemoteId, Error = ()>>;

    /// Sets the last job as the active job, if it isn't already.
    fn reactivate_last_job(root: &mut dyn RootRender, vdom: VdomWeak, id: Id);

    /// This is a temporary solution for the fact that the virtual DOM library used
    /// ([Dodrio]) doesn't support injecting raw HTML into the DOM.
    ///
    /// Whenever task details need to be rendered, this method should be called
    /// instead, to make sure the raw escaped HTML is swapped with the correct
    /// rendered HTML output.
    ///
    /// [Dordio]: https://github.com/fitzgen/dodrio
    fn render_task_details(vdom: VdomWeak) -> Box<dyn Future<Item = (), Error = ()>>;

    /// Activate the login field for a given task.
    fn show_task_login(root: &mut dyn RootRender, vdom: VdomWeak, id: Id);

    /// Deactivate the login field for a given task.
    fn hide_task_login(tasks: Rc<RefCell<tasks::Tasks>>, vdom: VdomWeak, id: Id);
}
