//! The main navigation bar on the home page.
//!
//! This includes the search field and will include the planned filters in the
//! future.

use crate::model::tasks;
use crate::utils;
use dodrio::{Node, Render, RenderContext};
use std::marker::PhantomData;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

/// The Navbar component.
pub(crate) struct Navbar<C> {
    /// The internal reference to the DOM element representing the search bar.
    search_node: Option<HtmlInputElement>,

    /// Reference to application controller.
    _controller: PhantomData<C>,
}

impl<C> Navbar<C> {
    /// Create a new Navbar component.
    pub(crate) fn new() -> Self {
        Self {
            search_node: utils::try_element(".search input"),
            _controller: PhantomData,
        }
    }

    /// Set the input value of the search bar to the provided string.
    pub(crate) fn set_search_value(&self, value: &str) {
        let _ = self.search_node.as_ref().map(|s| s.set_value(value));
    }

    /// Get the input value of the search bar.
    ///
    /// Returns an empty string if the search field DOM node does not exist.
    pub(crate) fn search_value(&self) -> String {
        self.search_node
            .as_ref()
            .map_or("".to_owned(), HtmlInputElement::value)
    }

    /// Set focus to the search field DOM node.
    pub(crate) fn focus_search(&self) {
        let _ = self.search_node.as_ref().map(HtmlInputElement::select);
    }

    /// Remove focus from the search field DOM node.
    pub(crate) fn blur_search(&self) {
        let _ = self.search_node.as_ref().map(|s| s.blur());
    }
}

impl<C> Render for Navbar<C>
where
    C: tasks::Actions,
{
    fn render<'b>(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let field = input(&cx)
            .attr("type", "text")
            .attr("aria-label", "search tasks")
            .attr("placeholder", "Search Tasks...")
            .on("input", move |root, vdom, event| {
                let value = event
                    .target()
                    .unwrap_throw()
                    .unchecked_into::<HtmlInputElement>()
                    .value();

                let query = if value.is_empty() {
                    None
                } else {
                    Some(value.as_str())
                };

                utils::set_location_query("search", query);
                spawn_local(C::search(root, vdom, value));
            });

        let search = div(&cx).attr("class", "search").child(field.finish());

        nav(&cx)
            .attr("class", "navbar")
            .child(div(&cx).child(search.finish()).finish())
            .finish()
    }
}
