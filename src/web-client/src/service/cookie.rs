//! The Cookie service allows fetching and storing cookie data.

use crate::utils;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::HtmlDocument;

/// The Cookie service.
#[derive(Clone)]
pub(crate) struct Service;

impl Service {
    /// Create a new Cookie service.
    pub(crate) const fn new() -> Self {
        Service
    }

    /// Set a cookie for a given name, value, and duration
    pub(crate) fn set(&self, name: &str, value: &str) {
        self.set_cookie(name, value, 365)
    }

    /// Retrieve a cookie value for a given name
    pub(crate) fn get(&self, name: &str) -> Option<String> {
        document()
            .cookie()
            .unwrap_throw()
            .split(';')
            .map(|cookie| cookie.splitn(2, '=').collect())
            .find_map(|mut parts: Vec<_>| {
                if let Some(value) = parts.get(0) {
                    if value.trim_start() == name {
                        return parts.pop().map(str::to_owned);
                    }
                }

                None
            })
    }

    /// Remove a cookie for a given name.
    pub(crate) fn remove(&self, name: &str) {
        self.set_cookie(name, "", -1);
    }

    /// Set a cookie for a given name and value. It expires after the set number
    /// of days.
    ///
    /// All cookies are marked as secure, and have strict SameSite policy
    /// enabled.
    fn set_cookie(&self, name: &str, value: &str, days: i32) {
        let cookie = format!(
            "{}={};max-age={};path=/;secure",
            name,
            value,
            days * 24 * 60 * 60
        );

        document().set_cookie(&cookie).unwrap_throw();
    }
}

/// Convert from a `Document` to an `HtmlDocument`.
fn document() -> HtmlDocument {
    JsValue::from(utils::document()).unchecked_into::<HtmlDocument>()
}
