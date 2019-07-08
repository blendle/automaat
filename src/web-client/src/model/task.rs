//! A task that can be run by starting a job.

use crate::graphql::fetch_task_details::{
    FetchTaskDetailsPipeline, FetchTaskDetailsPipelineVariables,
};
use crate::graphql::search_tasks::SearchTasksPipelines;
use crate::model::{job, variable};
use dodrio::{RootRender, VdomWeak};
use futures::future::Future;
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;

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
    details: SearchTasksPipelines,

    /// The variable objects returned by the GraphQL server.
    ///
    /// The values of the objects are used internally to expose the relevant
    /// details via the designated methods.
    ///
    /// This is an optional value, because the variables aren't fetched when
    /// searching for tasks, but are only returned when the task is opened in
    /// the UI. From that point on, the variables are cached and won't have to
    /// be fetched again during the active application session.
    variables: Option<Vec<FetchTaskDetailsPipelineVariables>>,
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

    /// Unset any active job as inactive, but keep the job around in the cache.
    pub(crate) fn deactivate_job(&mut self) {
        self.active_job_idx = None
    }

    /// Get the job that is currently marked as "active", if any.
    pub(crate) fn active_job(&self) -> Option<&job::Job> {
        match self.active_job_idx {
            None => None,
            Some(idx) => self.jobs.get(idx),
        }
    }
}

impl From<SearchTasksPipelines> for Task {
    fn from(details: SearchTasksPipelines) -> Self {
        Self {
            details,
            active_job_idx: None,
            variables: None,
            jobs: vec![],
        }
    }
}

impl From<FetchTaskDetailsPipeline> for Task {
    fn from(input: FetchTaskDetailsPipeline) -> Self {
        let details = SearchTasksPipelines {
            id: input.id.clone(),
            name: input.name.clone(),
            description: input.description.clone(),
        };

        Self {
            details,
            active_job_idx: None,
            variables: input.variables,
            jobs: vec![],
        }
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
}
