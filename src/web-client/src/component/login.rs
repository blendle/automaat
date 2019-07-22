//! The login dialogue shown when authentication is required.

use crate::model::session;
use crate::utils;
use dodrio::{Node, Render, RenderContext};
use futures::prelude::*;
use std::marker::PhantomData;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Element, HtmlInputElement};

/// The Login component.
pub(crate) struct Login<C> {
    /// Reference to application controller.
    _controller: PhantomData<C>,
}

impl<C> Login<C> {
    /// Create a new Login component.
    pub(crate) const fn new() -> Self {
        Self {
            _controller: PhantomData,
        }
    }

    /// Mark the login field as "failed" when the provided input is incorrect.
    pub(crate) fn as_failed() {
        let _ =
            utils::element(".login input").map(|s: Element| s.set_class_name("has-text-danger"));
    }
}

impl<C> Render for Login<C>
where
    C: session::Actions,
{
    fn render<'b>(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let logo = img(&cx)
            .attr("src", "img/logo-white.svg")
            .attr("alt", "Automaat logo")
            .finish();

        let field = input(&cx)
            .attr("type", "text")
            .attr("name", "token")
            .attr("aria-label", "login token")
            .attr("placeholder", "Login Token...")
            .on("input", move |root, vdom, event| {
                let target = event.target().unwrap_throw();
                let value = target.unchecked_ref::<HtmlInputElement>().value();

                spawn_local(C::authenticate(root, vdom, value).map_err(|_| Self::as_failed()));
            })
            .finish();

        let text = div(&cx)
            .attr("class", "description")
            .children([
                p(&cx)
                    .child(text(
                        "This instance of Automaat requires you to \
                         identify yourself.",
                    ))
                    .finish(),
                p(&cx)
                    .child(text(
                        "Please provide your personal token or ask someone \
                         to generate a new token for you.",
                    ))
                    .finish(),
            ])
            .finish();

        let content = div(&cx)
            .children([logo, div(&cx).child(field).finish(), text])
            .finish();

        section(&cx)
            .attr("class", "login")
            .child(div(&cx).child(content).finish())
            .finish()
    }
}
