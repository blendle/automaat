//! Small utility functions.
use wasm_bindgen::JsCast;
use web_sys::Element;

use wasm_bindgen::UnwrapThrowExt;

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
