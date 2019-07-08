//! A list of tasks shown in the UI after searching for a task.

use crate::component::TaskResult;
use crate::model::task::Task;
use dodrio::{Node, Render, RenderContext};
use std::marker::PhantomData;

/// The `Tasks` component.
pub(crate) struct Tasks<'a, C> {
    /// The vector of references to the tasks shown in the UI.
    tasks: Vec<&'a Task>,

    /// Reference to application controller.
    _controller: PhantomData<C>,
}

impl<'a, C> Tasks<'a, C> {
    /// Create a new component of a list of tasks.
    pub(crate) const fn new(tasks: Vec<&'a Task>) -> Self {
        Self {
            tasks,
            _controller: PhantomData,
        }
    }
}

impl<'a, C> Render for Tasks<'a, C> {
    fn render<'b>(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let task_results = self
            .tasks
            .iter()
            .map(|task| TaskResult::new(task))
            .map(|t: TaskResult<'_, C>| t.render(cx))
            .collect::<Vec<_>>();

        div(&cx)
            .attr("class", "tasks")
            .children(task_results)
            .finish()
    }
}
