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
use service::{CookieService, GraphqlService, ShortcutService};
use wasm_bindgen::prelude::*;

/// Starting point of the application once loaded in the browser.
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    init_log();

    let cookie = CookieService::new();
    let graphql = GraphqlService::new("/graphql", cookie.clone());
    let app: App = App::new(graphql, cookie);

    let body = utils::document().body().unwrap_throw();
    let vdom = Vdom::new(&body, app);

    let router: Router = Router::default();
    router.listen(&vdom.weak());

    let shortcut: ShortcutService = ShortcutService::default();
    shortcut.listen(vdom.weak());

    vdom.forget();
    Ok(())
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
