//! A `Job` is an instance of a `Task` that is either scheduled to run, is
//! actively running on the server, or ran in the past.

use crate::graphql::fetch_job_result::FetchJobResultJobStepsOutput;
use crate::model::{task, tasks};
use crate::service::GraphqlService;
use dodrio::{RootRender, VdomWeak};
use futures::future::Future;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

/// The job model.
#[derive(Clone, Debug, Default)]
pub(crate) struct Job {
    /// The variable values used to run the job.
    ///
    /// The key of the map is equal to the task variable name, the value is the
    /// variable value provided before delivering the job to the server.
    pub(crate) variable_values: HashMap<String, String>,

    /// The last known status of a job.
    pub(crate) status: Status,

    /// The job ID supplied by the server.
    ///
    /// This field is optional, because a job is created by the client _before_
    /// it is delivered to the server. If the server rejects to job's
    /// configuration, not remote ID is assigned, but the job still "ran" as
    /// soon as the job was triggered, and its failure message will match the
    /// message the server gave for rejecting the job.
    pub(crate) remote_id: Option<RemoteId>,
}

impl Job {
    /// Returns `true` if the job is considered to have completed its run.
    pub(crate) fn is_completed(&self) -> bool {
        use Status::*;

        match self.status {
            Created | Delivered => false,
            Succeeded(_) | Failed(_) => true,
        }
    }

    /// Returns `true` if the job is considered to be currently running on the
    /// server.
    pub(crate) fn is_running(&self) -> bool {
        !self.is_completed()
    }
}

/// The job output, containing both the html and text (markdown) output.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct Output {
    /// HTML formatted output.
    pub(crate) html: Option<String>,

    /// Text formatted output.
    pub(crate) text: Option<String>,
}

impl Output {
    /// Create html-only output.
    #[allow(unused)]
    pub(crate) fn html<T>(string: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            html: Some(string.into()),
            text: None,
        }
    }

    /// Create text-only output.
    #[allow(unused)]
    pub(crate) fn text<T>(string: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            html: None,
            text: Some(string.into()),
        }
    }
}

impl<T> From<Option<T>> for Output
where
    T: Into<String> + Clone,
{
    fn from(string: Option<T>) -> Self {
        Self {
            html: string.clone().map(Into::into),
            text: string.map(Into::into),
        }
    }
}

impl From<&FetchJobResultJobStepsOutput> for Output {
    fn from(input: &FetchJobResultJobStepsOutput) -> Self {
        Self {
            html: input.html.clone(),
            text: input.text.clone(),
        }
    }
}

/// The status of the job.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) enum Status {
    /// The job has been created, but nothing was done with it.
    Created,

    /// The job was successfully delivered to the server.
    Delivered,

    /// The server reported a successful run of the job.
    Succeeded(Output),

    /// The server either rejected the job, or the job failed while running.
    Failed(Output),
}

impl Default for Status {
    fn default() -> Self {
        Status::Created
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Status::*;

        match self {
            Created => f.write_str("status-created"),
            Delivered => f.write_str("status-delivered"),
            Succeeded(_) => f.write_str("status-succeeded"),
            Failed(_) => f.write_str("status-failed"),
        }
    }
}

/// The remote job ID provided by the server.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct RemoteId(String);

impl RemoteId {
    /// Create a new `RemoteId`.
    pub(crate) const fn new(id: String) -> Self {
        Self(id)
    }
}

impl fmt::Display for RemoteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// The actions a controller has to implement to bridge between the UI and the
/// model.
pub(crate) trait Actions {
    /// Asks the server for the result of a job.
    ///
    /// If the job is still pending or running, this function should block until
    /// the job either failed, or succeeded.
    ///
    /// Once a final state is reached, the job must be updated with the new
    /// details.
    fn poll_result(
        tasks: Rc<RefCell<tasks::Tasks>>,
        vdom: VdomWeak,
        id: RemoteId,
        task_id: task::Id,
        client: GraphqlService,
    ) -> Box<dyn Future<Item = (), Error = ()>>;

    /// Abort a job that is currently running.
    ///
    /// This function can be used to stop a running job if the results of the
    /// job are no longer relevant.
    fn abort(root: &mut dyn RootRender, vdom: VdomWeak, id: RemoteId);
}
