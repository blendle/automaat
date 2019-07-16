//! The router acts on route changes in the application, and maps those changes
//! to modifications on the models.

use crate::app::App;
use crate::component::Navbar;
use crate::controller::Controller;
use crate::model::{statistics, task, tasks};
use crate::utils;
use dodrio::{RootRender, VdomWeak};
use futures::prelude::*;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::PopStateEvent;

/// The router of the application.
pub(crate) struct Router<C = Controller>(PhantomData<C>);

impl<C: Clone> Default for Router<C> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<C> Router<C>
where
    C: tasks::Actions + task::Actions + statistics::Actions + Clone + 'static,
{
    /// Listen for route changes.
    pub(crate) fn listen(&self, vdom: VdomWeak) {
        use Route::*;

        // Callback fired whenever the URL changes.
        //
        // Opens task detail views if needed, or performs search queries.
        let on_popstate_event = move |_: PopStateEvent| {
            let route = match Route::active() {
                None => return utils::set_hash(&Home.to_string()),
                Some(route) => route,
            };

            spawn_local(
                vdom.with_component({
                    let vdom = vdom.clone();
                    move |root| {
                        let vdom = vdom.clone();
                        Self::handle_route_change(route, root, vdom)
                    }
                })
                .map_err(|_| ())
                .and_then(|fut| fut),
            );
        };

        // Handle initial page load.
        on_popstate_event(PopStateEvent::new("popstate").unwrap_throw());

        // popstate
        let closure: Closure<dyn FnMut(_)> = Closure::wrap(Box::new(on_popstate_event));
        utils::window()
            .add_event_listener_with_callback("popstate", closure.as_ref().unchecked_ref())
            .unwrap_throw();
        closure.forget();
    }

    /// Handle route changes.
    ///
    /// This function is called whenever the "popstate" event is triggered,
    /// which happens when the URL of the browser is changed, either by clicking
    /// on a URL, or by using the browser UI, such as the "back" button.
    fn handle_route_change(
        route: Route,
        root: &mut dyn RootRender,
        vdom: VdomWeak,
    ) -> impl Future<Item = (), Error = ()> {
        use Route::*;

        match route {
            Home => {
                let app = root.unwrap_mut::<App>();
                let nav = Navbar::<C>::new();

                // Set the search bar value based on the active query string,
                // unless it is already set to a non-empty string.
                if nav.search_value().is_empty() {
                    if let Some(value) = utils::get_location_query("search") {
                        nav.set_search_value(value.as_str())
                    }
                }

                // Remove any unwanted query parameters from the current
                // location URL.
                //
                // This prevents query parameters added while editing a task
                // form from preserving when returning to the home screen.
                for (key, _) in utils::location_query_params() {
                    match key.as_str() {
                        "search" => continue,
                        other => utils::set_location_query(other, None),
                    }
                }

                // Auto-focus the search bar.
                nav.focus_search();

                // Unset all active tasks when visiting the home page.
                //
                // It is possible to hit this path when you use the browser's
                // "back" button to go back from the task details view to the
                // homepage.
                //
                // The "regular" ways of dismissing the details view (by using
                // the controller's `close_active_task` method) already unloads
                // the active task. In that case, this is a no-op.
                app.tasks_mut().unwrap_throw().disable_all_active_tasks();

                // Update the `navbar` statistics.
                //
                // We do not want to block the application on these statistics,
                // so we spawn a separate future and ignore its output.
                spawn_local(C::update_statistics(root, vdom.clone()));

                // It might be tempting to only trigger the search when the page
                // is first loaded, instead of every time this route is
                // activated.
                //
                // Unfortunately, that won't work, because coming in via a
                // direct task link will only fetch that task, and so without
                // doing an explicit search when going back to the home page,
                // you'd only see that single task in the search results.
                C::search(root, vdom, nav.search_value())
            }

            Task(id) => C::activate_task(root, vdom, id),
        }
    }
}

/// The set of known routes this router can act on.
#[derive(Debug)]
pub(crate) enum Route {
    /// The home page of the application.
    ///
    /// This page shows a list of (optionally filtered) set of tasks that can be
    /// activated.
    Home,

    /// The task details view.
    ///
    /// This view shows the details of a single task, and allows the task to be
    /// converted into a job by providing the required variables.
    Task(task::Id),
}

impl Route {
    /// Returns the current active route, if the path can be matched to one of
    /// the known routes. Returns `None` if the path cannot be parsed.
    pub(crate) fn active() -> Option<Self> {
        Self::from_str(utils::hash().unwrap_or_else(|| "".to_owned()).as_str()).ok()
    }

    /// Changes the path of the browser to the route on which this method is
    /// called.
    pub(crate) fn set_path(&self) {
        utils::set_hash(self.to_string().as_ref())
    }
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Route::*;

        match self {
            Home => f.write_str("#/"),
            Task(id) => write!(f, "#/task/{}", id),
        }
    }
}

/// The error value returned when a string-based path cannot be converted into a
/// `Route`.
pub(crate) struct UnknownRoute;

impl FromStr for Route {
    type Err = UnknownRoute;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Route::*;

        match s {
            "#/" => Ok(Home),
            p if p.starts_with("#/task/") => {
                let id = p.rsplitn(2, '/').next().unwrap_throw();
                if id.is_empty() {
                    Err(UnknownRoute)
                } else {
                    Ok(Task(task::Id::new(id.to_owned())))
                }
            }
            _ => Err(UnknownRoute),
        }
    }
}
