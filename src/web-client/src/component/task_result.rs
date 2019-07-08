//! A single task result shown in the UI when searching for tasks.

use crate::model::task::Task;
use crate::router::Route;
use dodrio::bumpalo::collections::string::String;
use dodrio::bumpalo::format;
use dodrio::{Node, Render, RenderContext};
use std::marker::PhantomData;

/// The `TaskResult` component.
pub(crate) struct TaskResult<'a, C> {
    /// A reference to the task for which the (sparse) details are shown in the
    /// list of tasks.
    task: &'a Task,

    /// Reference to application controller.
    _controller: PhantomData<C>,
}

impl<'a, C> TaskResult<'a, C> {
    /// Create a new `TaskResult` component with the provided task reference.
    pub(crate) const fn new(task: &'a Task) -> Self {
        Self {
            task,
            _controller: PhantomData,
        }
    }
}

/// The trait implemented by this component to render all its views.
trait Views<'b> {
    /// The header part of the result, showing the name of the task.
    fn header(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The description of the task.
    fn description(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The button to open the details view of the task.
    fn open_button(&self, cx: &mut RenderContext<'b>) -> Node<'b>;
}

impl<'a, 'b, C> Views<'b> for TaskResult<'a, C> {
    fn header(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let name = String::from_str_in(self.task.name(), cx.bump).into_bump_str();

        div(&cx)
            .attr("class", "header")
            .child(div(&cx).child(h1(&cx).child(text(name)).finish()).finish())
            .finish()
    }

    fn description(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let description = String::from_str_in(self.task.description(), cx.bump).into_bump_str();

        div(&cx)
            .attr("class", "description")
            .child(
                div(&cx)
                    .child(p(&cx).child(text(description)).finish())
                    .finish(),
            )
            .finish()
    }

    fn open_button(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let route = Route::Task(self.task.id());
        let url = format!(in cx.bump, "{}", route).into_bump_str();
        let label = format!(in cx.bump, "Open task: {}", self.task.name()).into_bump_str();

        a(&cx)
            .attr("href", url)
            .attr("tabindex", "0")
            .attr("aria-label", label)
            .child(div(&cx).child(i(&cx).finish()).finish())
            .finish()
    }
}

impl<'a, C> Render for TaskResult<'a, C> {
    fn render<'b>(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let details = div(&cx)
            .children([self.header(cx), self.description(cx)])
            .finish();

        let content = div(&cx).children([details, self.open_button(cx)]).finish();

        div(&cx)
            .attr("class", "task-result")
            .child(div(&cx).child(content).finish())
            .finish()
    }
}
