//! Small utility functions.

use js_sys::Array;
use std::collections::HashMap;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{HtmlInputElement, HtmlSelectElement, Url};

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

/// Given any element T, try to cast it into an input element type, extract the
/// `name` and `value` from the input field, and add it as a key/value pair to
/// the current location query field.
///
/// For example, if an input field has a name attribute of "firstName" and the
/// field contains the value "Bart", the query parameter `firstName=Bart` is
/// added to the current URL location.
///
/// The function returns an error if the passed-in element cannot be casted to
/// an input element type.
pub(crate) fn input_to_location_query<T>(element: T) -> Result<(), ()>
where
    T: JsCast,
{
    let (name, value) = if element.has_type::<HtmlInputElement>() {
        let el = element.unchecked_into::<HtmlInputElement>();
        (el.name(), el.value())
    } else if element.has_type::<HtmlSelectElement>() {
        let el = element.unchecked_into::<HtmlSelectElement>();
        (el.name(), el.value())
    } else {
        return Err(());
    };

    let query = if value.is_empty() {
        None
    } else {
        Some(value.as_str())
    };

    set_location_query(name.as_str(), query);
    Ok(())
}

/// Return the location query params as a hashmap.
///
/// For example, if the location contains `?hello=world&good=bye`, then the
/// returned map contains the keys "hello" and "good", with the values "world"
/// and "bye".
pub(crate) fn location_query_params() -> HashMap<String, String> {
    let href = window().location().href().unwrap_throw();
    let search = Url::new(&href).unwrap_throw().search_params();

    js_sys::try_iter(&search)
        .unwrap_throw()
        .unwrap_throw()
        .map(UnwrapThrowExt::unwrap_throw)
        .map(|v| Array::from(&v))
        .map(|v| (v.pop().as_string(), v.pop().as_string()))
        .map(|(v, k)| (k.unwrap_throw(), v.unwrap_throw()))
        .collect()
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
/// `None` is returned if the element could not be found or is of a different
/// type.
pub(crate) fn element<T>(selector: &str) -> Option<T>
where
    T: JsCast,
{
    document()
        .query_selector(selector)
        .unwrap_throw()
        .and_then(|e| e.dyn_into::<T>().ok())
}
