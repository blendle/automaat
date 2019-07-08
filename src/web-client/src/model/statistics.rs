//! A collection of global application statistics.

use dodrio::{RootRender, VdomWeak};
use futures::future::Future;

/// The statistics model.
#[derive(Clone, Debug, Default)]
pub(crate) struct Statistics {
    /// The total number of tasks available on the server.
    ///
    /// The value is optional to allow for lazy-loading of the value.
    pub(crate) total_tasks: Option<u32>,

    /// The number of actively running jobs on the server.
    ///
    /// The value is optional to allow for lazy-loading of the value.
    pub(crate) running_jobs: Option<u32>,

    /// The number of failed jobs on the server.
    ///
    /// The value is optional to allow for lazy-loading of the value.
    pub(crate) failed_jobs: Option<u32>,
}

impl Statistics {
    /// Update the model to contain up-to-date metrics.
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn update(&mut self, total: usize, running: usize, failed: usize) {
        self.total_tasks = Some(total as u32);
        self.running_jobs = Some(running as u32);
        self.failed_jobs = Some(failed as u32);
    }
}

/// The actions a controller has to implement to bridge between the UI and the
/// model.
pub(crate) trait Actions {
    /// Update the statistics model based on UI events, such as starting a new
    /// job.
    fn update_statistics(
        root: &mut dyn RootRender,
        vdom: VdomWeak,
    ) -> Box<dyn Future<Item = (), Error = ()> + 'static>;
}
