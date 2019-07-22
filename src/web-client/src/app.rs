//! The top-level application container, containing all application-related
//! state.

use crate::component;
use crate::controller::Controller;
use crate::model::{job, session, statistics, task, tasks};
use crate::router::Route;
use crate::service::{CookieService, GraphqlService};
use dodrio::{Node, Render, RenderContext};
use std::cell::{Ref, RefCell, RefMut};
use std::marker::PhantomData;
use std::rc::Rc;
use wasm_bindgen::UnwrapThrowExt;

/// Main application container.
pub(crate) struct App<C = Controller> {
    /// The GraphQL client to fetch and mutate data.
    pub(crate) client: GraphqlService,

    /// The cookie service to modify cookie data.
    pub(crate) cookie: CookieService,

    /// All tasks fetched since the start of the application session.
    ///
    /// This is purely meant for caching purposes, the source of truth lives on
    /// the server, and if needed, a task should be fetched again and replaced
    /// in the cache.
    tasks: Rc<RefCell<tasks::Tasks>>,

    /// Global statistics of the application, such as total tasks available on
    /// the server, or number of actively running jobs.
    stats: Rc<RefCell<statistics::Statistics>>,

    /// Reference to application controller.
    _controller: PhantomData<C>,
}

impl<C> App<C> {
    /// Create a new application instance, with the provided GraphQL service.
    pub(crate) fn new(client: GraphqlService, cookie: CookieService) -> Self {
        Self {
            client,
            cookie,
            tasks: Rc::default(),
            stats: Rc::default(),
            _controller: PhantomData,
        }
    }

    /// Get a reference to the tasks cache.
    pub(crate) fn tasks(&self) -> Result<Ref<'_, tasks::Tasks>, ()> {
        self.tasks.try_borrow().map_err(|_| ())
    }

    /// Get a mutable reference to the tasks cache.
    pub(crate) fn tasks_mut(&self) -> Result<RefMut<'_, tasks::Tasks>, ()> {
        self.tasks.try_borrow_mut().map_err(|_| ())
    }

    /// Get a reference-counted clone of the cached tasks.
    pub(crate) fn cloned_tasks(&self) -> Rc<RefCell<tasks::Tasks>> {
        Rc::clone(&self.tasks)
    }

    /// Get a reference-counted clone of the cached statistics.
    pub(crate) fn cloned_statistics(&self) -> Rc<RefCell<statistics::Statistics>> {
        Rc::clone(&self.stats)
    }
}

impl<C> Render for App<C>
where
    C: tasks::Actions + task::Actions + job::Actions + session::Actions + Clone + 'static,
{
    fn render<'b>(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        // TODO: once we have actual session data to store, we should add an
        // `Option<Session>` to the `App`, and trigger this route if that value
        // is set to `None`, instead of reading the current path.
        if let Some(Route::Login) = Route::active() {
            return component::Login::<C>::new().render(cx);
        }

        let stats = self.stats.try_borrow().unwrap_throw();
        let tasks = self.tasks().unwrap_throw();
        let filtered_tasks = tasks.filtered_tasks();

        let header = component::Header::new(stats);
        let navbar = component::Navbar::<C>::new();
        let tasks_list = component::Tasks::<C>::new(filtered_tasks);

        let mut node = div(&cx)
            .child(header.render(cx))
            .child(navbar.render(cx))
            .child(tasks_list.render(cx));

        let tasks = self.tasks().unwrap_throw();

        if let Some(task) = tasks.active_task() {
            let task_details = component::TaskDetails::<C>::new(&*task);
            node = node.child(task_details.render(cx));
        };

        node.finish()
    }
}
