//! An [Automaat] web client implementation.
//!
//! This is a web client implementation that can be used to interact with the
//! Automaat system.
//!
//! It requires the [Automaat Server] component to be running on the same domain
//! as the web client (it uses relative API endpoints).
//!
//! You can view existing Automaat pipelines, filter the list of pipelines, and
//! run pipelines by providing the required variables.
//!
//! Currently this client does not support _creating_ new pipelines. For this,
//! you can use the [GraphQL Playground] interface exposed by the server via
//! `/graphql/playground`.
//!
//! [Automaat]: https://docs.rs/automaat-core
//! [Automaat Server]: https://docs.rs/automaat-server
//! [GraphQL Playground]: https://git.io/fj2bx
#![deny(
    clippy::all,
    clippy::cargo,
    clippy::clone_on_ref_ptr,
    clippy::indexing_slicing,
    clippy::mem_forget,
    clippy::missing_docs_in_private_items,
    clippy::multiple_inherent_impl,
    clippy::nursery,
    clippy::option_unwrap_used,
    clippy::pedantic,
    clippy::print_stdout,
    clippy::result_unwrap_used,
    clippy::wildcard_enum_match_arm,
    clippy::wrong_pub_self_convention,
    deprecated_in_future,
    future_incompatible,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    rustdoc,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    unused_results,
    variant_size_differences,
    warnings
)]
#![warn(clippy::dbg_macro, clippy::unimplemented, clippy::use_debug)]
#![doc(html_root_url = "https://docs.rs/automaat-web-client/0.1.0")]

/// The Wasm-Enabled, Elfin Allocator trades allocation performance for small
/// code size.
///
/// See: <https://docs.rs/wee_alloc>
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc<'_> = wee_alloc::WeeAlloc::INIT;

pub(crate) mod app;
pub(crate) mod component;
pub(crate) mod controller;
pub(crate) mod graphql;
pub(crate) mod model;
pub(crate) mod router;
pub(crate) mod service;
pub(crate) mod utils;

use app::App;
use dodrio::Vdom;
use router::Router;
use service::{GraphqlService, ShortcutService};
use wasm_bindgen::{prelude::*, JsCast};

/// Starting point of the application once loaded in the browser.
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    init_log();

    let graphql = GraphqlService::new("/graphql");
    let app: App = App::new(graphql);

    let body = utils::document().body().unwrap_throw();
    let vdom = Vdom::new(&body, app);

    let router: Router = Router::default();
    router.listen(vdom.weak());

    let shortcut: ShortcutService = ShortcutService::default();
    shortcut.listen(vdom.weak());

    raw_text_to_html();

    vdom.forget();
    Ok(())
}

/// This is a temporary solution for the fact that the virtual DOM library used
/// ([Dodrio]) doesn't support injecting raw HTML into the DOM.
///
/// Instead, this function starts a [`MutationObserver`] to listen for raw
/// HTML to be inserted into the DOM from tasks, it then takes that HTML as raw
/// text, and inserts it as actual HTML elements into the DOM.
///
/// It happens fast enough (probably within the same render call) that there is
/// no visual glitch, but if that does turn out to be the case, we can migrate
/// the raw output into a hidden element, and take the content from there.
///
/// [Dordio]: https://github.com/fitzgen/dodrio
/// [`MutationObserver`]: https://developer.mozilla.org/en-US/docs/Web/API/MutationObserver
fn raw_text_to_html() {
    use web_sys::{HtmlElement, MutationObserver, MutationRecord};

    let cb: Closure<dyn FnMut(js_sys::Array, MutationObserver)> =
        Closure::wrap(Box::new(|records, _| {
            let mut records = match js_sys::try_iter(&records).unwrap_throw() {
                Some(records) => records,
                None => return,
            };

            let record = match records.nth(0) {
                Some(record) => record.unwrap_throw().unchecked_into::<MutationRecord>(),
                None => return,
            };

            let node = js_sys::try_iter(&record.added_nodes())
                .unwrap_throw()
                .unwrap_throw()
                .nth(0);

            let el = match node.and_then(|n| n.unwrap_throw().dyn_into::<HtmlElement>().ok()) {
                Some(el) => el,
                None => return,
            };

            let body = match utils::try_child::<HtmlElement>(&el, "article > div:nth-child(2)") {
                None => return,
                Some(body) => body,
            };

            let raw_html = body.text_content().unwrap_throw();
            body.set_inner_html(&raw_html);
        }));

    let mut options = web_sys::MutationObserverInit::new();
    let _ = options.child_list(true).subtree(true);

    let observer = web_sys::MutationObserver::new(cb.as_ref().unchecked_ref()).unwrap_throw();
    cb.forget();
    observer
        .observe_with_options(&utils::element("body"), &options)
        .unwrap_throw();
}

/// If the `console` feature is enabled, we enable functionality to log to the
/// browser console using `log::{debug,info,...}`.
///
/// If the application panics, a stack-trace is printed to the console as well.
///
/// This requires "panic" and "fmt" infrastructure in the final binary, and so
/// this is disabled for production builds.
#[cfg(feature = "console")]
fn init_log() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Trace).unwrap_throw();
}

#[cfg(not(feature = "console"))]
fn init_log() {}
