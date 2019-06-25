use lazy_static::lazy_static;
use regex::Regex;
use wasm_bindgen::JsCast;
use web_sys::{window as web_window, Document, Element, Event, KeyboardEvent, Window};

pub(crate) fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

pub(crate) fn window() -> Window {
    web_window().expect("global window")
}

pub(crate) fn document() -> Document {
    window().document().expect("document object")
}

pub(crate) fn element(selector: &str) -> Option<Element> {
    let element = document().query_selector(selector).expect("valid selector");

    element_with_console_error(element, selector)
}

pub(crate) fn element_child(element: &Element, selector: &str) -> Option<Element> {
    let element = element.query_selector(selector).expect("valid selector");

    element_with_console_error(element, selector)
}

pub(crate) fn element_is_active(element: &Element) -> bool {
    match document().active_element() {
        None => false,
        Some(el) => el.id() == element.id(),
    }
}

pub(crate) fn keyboard_event(event: &Event) -> Option<u32> {
    JsCast::dyn_ref::<KeyboardEvent>(event).map(KeyboardEvent::key_code)
}

fn element_with_console_error(element: Option<Element>, selector: &str) -> Option<Element> {
    if element.is_none() {
        console_error(&format!("could not find element: {}", selector));
    }

    element
}

fn console_error(message: &str) {
    web_sys::console::error_1(&message.into());
}

pub(crate) fn format_id_from_str(string: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new("[^A-z0-9\\-_]").unwrap();
    }

    RE.replace_all(string, "").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_id_from_str() {
        let invalid = " this is / invalid!";
        let format = format_id_from_str(invalid);

        assert_eq!(format.as_str(), "thisisinvalid")
    }
}
