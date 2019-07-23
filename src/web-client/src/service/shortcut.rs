//! The shortcut service can act on keyboard input to perform specific actions.
//!
//! In a sense, this is closely related to the `Router`, whereas the router acts
//! on path changes and updates the models, this service acts on keystrokes and
//! updates the models.

use crate::component::Navbar;
use crate::controller::Controller;
use crate::model::task;
use crate::router::Route;
use crate::utils;
use dodrio::VdomWeak;
use futures::prelude::*;
use gloo_events::{EventListener, EventListenerOptions};
use std::marker::PhantomData;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlElement, HtmlInputElement, KeyboardEvent};

/// The Enter key code.
pub(crate) const ENTER: u32 = 13;

/// The Escape key code.
pub(crate) const ESCAPE: u32 = 27;

/// The F key code.
pub(crate) const F: u32 = 70;

/// The Shortcut service.
#[derive(Default)]
pub(crate) struct Service<C = Controller>(PhantomData<C>);

impl<C> Service<C>
where
    C: task::Actions,
{
    /// Listen for keyboard input and perform model or DOM updates based on the
    /// input.
    pub(crate) fn listen(&self, vdom: VdomWeak) {
        use Route::*;

        let options = EventListenerOptions::enable_prevent_default();
        EventListener::new_with_options(&utils::document(), "keydown", options, move |event| {
            let event = event.unchecked_ref::<KeyboardEvent>();
            let target = event.target().unwrap_throw();
            let route = match Route::active() {
                None => return,
                Some(route) => route,
            };

            // Set the active keyboard shortcuts based on the currently active
            // route.
            //
            // If the route isn't matched, no shortcuts are enabled.
            match route {
                Home => {
                    let navbar = Navbar::<C>::new();
                    match event.key_code() {
                        F if !target.has_type::<HtmlInputElement>() => navbar.focus_search(),
                        ESCAPE => navbar.blur_search(),
                        _ => return,
                    };
                }
                Task(_) => match event.key_code() {
                    ESCAPE if !target.has_type::<HtmlInputElement>() => spawn_local(
                        vdom.with_component({
                            let vdom = vdom.clone();
                            |root| C::close_active_task(root, vdom)
                        })
                        .map_err(|_| ()),
                    ),
                    ENTER => utils::element::<HtmlElement>(".task-details button[type=submit]")
                        .unwrap_throw()
                        .click(),
                    _ => return,
                },
            }

            event.prevent_default();
        })
        .forget();
    }
}
