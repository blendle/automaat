//! Small utility functions.

use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{Element, Url};

/// Get the current location hash, if any.
pub(crate) fn hash() -> Option<String> {
    window()
        .location()
        .hash()
        .ok()
        .and_then(|h| if h.is_empty() { None } else { Some(h) })
}

/// Set the current location hash.
pub(crate) fn set_hash(hash: &str) {
    window().location().set_hash(hash).unwrap_throw();
}

/// Get the location query string matching the provided name.
///
/// Returns `None` if no query string matching the name could be found.
pub(crate) fn get_location_query(name: &str) -> Option<String> {
    let href = window().location().href().unwrap_throw();
    let search = Url::new(&href).unwrap_throw().search_params();

    search.get(name)
}

/// Set a query string value of the current location.
///
/// If the passed-in value is `None`, any active query string matching the
/// provided name will be removed.
///
/// If the value is `Some`, it will override any existing query string value
/// matching the provided name.
pub(crate) fn set_location_query(name: &str, value: Option<&str>) {
    let href = window().location().href().unwrap_throw();
    let url = Url::new(href.as_str()).unwrap_throw();
    let search = url.search_params();

    match value {
        None => search.delete(name),
        Some(value) => search.set(name, value),
    };

    let mut string = search.to_string().as_string().unwrap_throw();
    if !string.is_empty() {
        string.insert_str(0, "?");
    }

    url.set_search(string.as_str());

    window()
        .history()
        .unwrap_throw()
        .replace_state_with_url(&"".into(), "", Some(url.href().as_str()))
        .unwrap_throw();
}

/// Get the top-level window.
pub(crate) fn window() -> web_sys::Window {
    web_sys::window().unwrap_throw()
}

/// Get the top-level document.
pub(crate) fn document() -> web_sys::Document {
    window().document().unwrap_throw()
}

/// Find a single element in the document and cast it into the provided type.
///
/// # Panic
///
/// This function panics under the following circumstances:
///
/// * The query selector has an invalid format.
/// * The queried element does not exist.
/// * The element is of the wrong type.
pub(crate) fn element<T>(selector: &str) -> T
where
    T: JsCast,
{
    document()
        .query_selector(selector)
        .unwrap_throw()
        .unwrap_throw()
        .unchecked_into::<T>()
}

/// Similar to `element`, except that it returns an optional value.
///
/// If the element could not be found, or could not be casted to the provided
/// type, this function returns `None`, otherwise the `Some` will contain the
/// requested element type.
pub(crate) fn try_element<T>(selector: &str) -> Option<T>
where
    T: JsCast,
{
    document()
        .query_selector(selector)
        .unwrap_throw()
        .and_then(|e| e.dyn_into::<T>().ok())
}

/// Similar to `child`, except that it returns an optional value.
///
/// If the element could not be found, or could not be casted to the provided
/// type, this function returns `None`, otherwise the `Some` will contain the
/// requested element type.
pub(crate) fn try_child<T>(element: &Element, selector: &str) -> Option<T>
where
    T: JsCast,
{
    element
        .query_selector(selector)
        .unwrap_throw()
        .and_then(|e| e.dyn_into::<T>().ok())
}
