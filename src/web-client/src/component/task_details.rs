//! The component that displays the details of a specific task, and allows any
//! interaction with that task (such as running it, or viewing previous results)
//! to happen.

use crate::app::App;
use crate::component;
use crate::model::job::{self, Job, Status};
use crate::model::session::{self, AccessMode};
use crate::model::task::{self, Task};
use crate::utils;
use dodrio::bumpalo::collections::string::String as BString;
use dodrio::{Node, Render, RenderContext};
use futures::prelude::*;
use std::marker::PhantomData;
use wasm_bindgen::JsCast;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlFormElement, HtmlInputElement};

/// The `TaskDetails` component.
pub(crate) struct TaskDetails<'a, C> {
    /// A reference to the task for which the details are presented.
    task: &'a Task,

    /// The mode of access to this task, according to the active session
    /// details.
    access_mode: AccessMode,

    /// Reference to application controller.
    _controller: PhantomData<C>,
}

impl<'a, C> TaskDetails<'a, C> {
    /// Create a new TaskDetails component for the provided task.
    pub(crate) const fn new(task: &'a Task, access_mode: AccessMode) -> Self {
        Self {
            task,
            access_mode,
            _controller: PhantomData,
        }
    }

    /// Set focus to the login field DOM node.
    pub(crate) fn focus_login() {
        let _ = utils::element("input.login")
            .as_ref()
            .map(HtmlInputElement::select);
    }
}

/// The trait implemented by this component to render all its views.
trait Views<'b> {
    /// The header section of the details view.
    fn header(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The body of the details view, showing the task description, optionally
    /// its defined variables, and the output result after running a task.
    fn body(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The list of variables belonging to the task.
    fn variables(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The resulting output after running a task.
    fn results(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The footer section of the task details. This contains the navigation
    /// buttons for exiting the details view, or running the task.
    fn footer(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The back button to exit the details view.
    fn btn_back(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The authenticate button to open the login dialog.
    fn btn_authenticate(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The login field to authenticate.
    fn field_login(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The run button to start running a task.
    fn btn_run(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The (disabled) "missing authorization" button.
    fn btn_unauthorized(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The form is the container object that contains the header, body and
    /// footer of the details view.
    fn form(&self, cx: &mut RenderContext<'b>) -> Node<'b>;
}

impl<'a, 'b, C> Views<'b> for TaskDetails<'a, C>
where
    C: task::Actions + job::Actions + session::Actions,
{
    fn header(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let name = BString::from_str_in(self.task.name(), cx.bump).into_bump_str();

        header(&cx)
            .child(p(&cx).child(text(name)).finish())
            .finish()
    }

    fn body(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let description = BString::from_str_in(self.task.description(), cx.bump).into_bump_str();
        let details = div(&cx)
            .child(p(&cx).child(text(description)).finish())
            .child(self.variables(cx));

        let mut body = div(&cx).child(div(&cx).child(details.finish()).finish());

        if let Some(job) = self.task.active_job() {
            if job.is_completed() {
                body = body.child(self.results(cx))
            }
        } else if !self.task.finished_jobs().is_empty() {
            let id = self.task.id();
            let link = a(&cx)
                .child(text("review the results of the last run"))
                .on("click", move |root, vdom, _event| {
                    let id = id.clone();
                    C::reactivate_last_job(root, vdom, id)
                })
                .finish();

            body = body.child(
                div(&cx)
                    .attr("class", "last-result")
                    .children([
                        text("You can "),
                        link,
                        text(", because you ran this task before."),
                    ])
                    .finish(),
            );
        }

        section(&cx).child(body.finish()).finish()
    }

    fn variables(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let variables = self.task.variables();
        let components: Vec<component::Variable<'_>> = variables.as_ref().map_or(vec![], |v| {
            v.iter()
                .map(|variable| {
                    let existing_value = self.task.active_job().and_then(|job| {
                        job.variable_values.get(variable.key()).map(String::as_ref)
                    });

                    (variable, existing_value)
                })
                .map(Into::into)
                .collect()
        });

        fieldset(&cx)
            .children(components.iter().map(|v| v.render(cx)).collect::<Vec<_>>())
            .finish()
    }

    fn results(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let (class, title, body) = match self.task.active_job().map(|j| &j.status).unwrap_throw() {
            Status::Succeeded(string) => ("is-success", "Success!", string),
            Status::Failed(string) => ("is-danger", "Failed!", string),
            _ => unreachable!(),
        };

        let class = BString::from_str_in(class, cx.bump).into_bump_str();
        let header = BString::from_str_in(title, cx.bump).into_bump_str();
        let body = BString::from_str_in(body.as_str(), cx.bump).into_bump_str();

        let staging = div(&cx)
            .attr("class", "message-staging")
            .child(text(body))
            .finish();

        let header = div(&cx)
            .attr("class", "message-header")
            .child(p(&cx).child(text(header)).finish())
            .finish();

        let body = div(&cx).attr("class", "message-body").finish();

        let details = article(&cx)
            .attr("class", class)
            .children([staging, header, body])
            .finish();

        div(&cx)
            .attr("class", "job-response")
            .child(div(&cx).child(div(&cx).child(details).finish()).finish())
            .finish()
    }

    fn footer(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let action = if self.task.show_login {
            self.field_login(cx)
        } else {
            match self.access_mode {
                AccessMode::Ok => self.btn_run(cx),
                AccessMode::Unauthorized => self.btn_unauthorized(cx),
                AccessMode::Unauthenticated => self.btn_authenticate(cx),
            }
        };

        footer(&cx).children([self.btn_back(cx), action]).finish()
    }

    fn btn_back(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        button(&cx)
            .attr("class", "back")
            .attr("type", "button")
            .bool_attr(
                "disabled",
                self.task.active_job().map_or(false, Job::is_running),
            )
            .child(span(&cx).child(i(&cx).finish()).finish())
            .child(span(&cx).child(text(" Back")).finish())
            .on("click", move |root, vdom, _event| {
                C::close_active_task(root, vdom)
            })
            .finish()
    }

    fn btn_authenticate(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let id = self.task.id();
        let class = BString::from_str_in(&self.access_mode.to_string(), cx.bump);

        button(&cx)
            .attr("type", "button")
            .attr("class", class.into_bump_str())
            .child(span(&cx).child(text("Authentication Required ")).finish())
            .child(span(&cx).child(i(&cx).finish()).finish())
            .on("click", move |root, vdom, _event| {
                C::show_task_login(root, vdom, id.clone());
                Self::focus_login()
            })
            .finish()
    }

    fn field_login(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let id = self.task.id();

        input(&cx)
            .attr("class", "login")
            .attr("placeholder", "Please provide your login token...")
            .on("input", move |root, vdom, event| {
                let target = event.target().unwrap_throw();
                let value = target.unchecked_ref::<HtmlInputElement>().value();

                let app = root.unwrap_mut::<App>();
                let tasks = app.cloned_tasks();
                let id = id.clone();

                spawn_local(
                    C::authenticate(root, vdom.clone(), Some(value))
                        .map(move |_| C::hide_task_login(tasks, vdom, id)),
                );
            })
            .finish()
    }

    fn btn_run(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let mut disabled = false;
        let mut class = BString::from_str_in(&self.access_mode.to_string(), cx.bump);
        if self.task.active_job().map_or(false, Job::is_running) {
            class.push_str(" is-loading");
            disabled = true;
        };

        button(&cx)
            .attr("type", "submit")
            .attr("class", class.into_bump_str())
            .bool_attr("disabled", disabled)
            .child(span(&cx).child(text("Run Task ")).finish())
            .child(span(&cx).child(i(&cx).finish()).finish())
            .finish()
    }

    fn btn_unauthorized(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let class = BString::from_str_in(&self.access_mode.to_string(), cx.bump);

        button(&cx)
            .attr("type", "button")
            .attr("class", class.into_bump_str())
            .bool_attr("disabled", true)
            .child(span(&cx).child(text("Insufficient Privileges ")).finish())
            .child(span(&cx).child(i(&cx).finish()).finish())
            .finish()
    }

    fn form(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let mut form = form(&cx);

        if let Some(status) = self.task.active_job().map(|j| &j.status) {
            let class = BString::from_str_in(status.to_string().as_str(), cx.bump).into_bump_str();
            form = form.attr("class", class);
        };

        let id = self.task.id();
        form.children([self.header(cx), self.body(cx), self.footer(cx)])
            .on("submit", move |root, vdom, event| {
                let form = event
                    .target()
                    .unwrap_throw()
                    .unchecked_into::<HtmlFormElement>();

                let data = web_sys::FormData::new_with_form(&form).unwrap_throw();
                let object = js_sys::Object::from_entries(&data).unwrap_throw();
                let map = object.into_serde().unwrap_throw();

                let app = root.unwrap_mut::<App>();
                let tasks = app.cloned_tasks();
                let client = app.client.to_owned();

                let id = id.clone();
                let vdom2 = vdom.clone();
                spawn_local({
                    C::run(root, vdom.clone(), id.clone(), map)
                        .and_then(move |job_id| C::poll_result(tasks, vdom, job_id, id, client))
                        .and_then(move |_| C::render_task_details(vdom2))
                });

                event.prevent_default()
            })
            .finish()
    }
}

impl<'a, C> Render for TaskDetails<'a, C>
where
    C: task::Actions + job::Actions + session::Actions,
{
    fn render<'b>(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        div(&cx)
            .attr("class", "task-details")
            .child(div(&cx).finish())
            .child(self.form(cx))
            .finish()
    }
}
