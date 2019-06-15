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
#![warn(
    clippy::all,
    clippy::cargo,
    clippy::nursery,
    clippy::pedantic,
    deprecated_in_future,
    future_incompatible,
    missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    rustdoc,
    warnings,
    unused_results,
    unused_qualifications,
    unused_lifetimes,
    unused_import_braces,
    unsafe_code,
    unreachable_pub,
    trivial_casts,
    trivial_numeric_casts,
    missing_debug_implementations,
    missing_copy_implementations
)]
#![warn(variant_size_differences)]
#![allow(clippy::multiple_crate_versions, missing_doc_code_examples)]
#![doc(html_root_url = "https://docs.rs/automaat-web-client/0.1.0")]
#![recursion_limit = "1024"]

use crate::resources::{Pipelines, TaskStatuses};
use crate::views::{
    PipelinesView, SearchBarView,
    StatisticType::{FailedTasks, RunningTasks, TotalPipelines},
    StatisticsView,
};
use futures::future::Future;
use std::convert::TryInto;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

mod resources;
mod utils;
mod views;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc<'_> = wee_alloc::WeeAlloc::INIT;

/// Start the application.
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    // Set up the panic hook for debugging when things go wrong.
    utils::set_panic_hook();

    SearchBarView::init();
    let query = SearchBarView::search_query().try_into().ok();

    spawn_local(Pipelines::fetch(query).and_then(|pipelines| {
        PipelinesView::update(&pipelines);
        futures::future::ok(())
    }));

    // TODO: we're doing an extra call just to show an accurate "total
    // pipelines" statistic. Seems a bit wasteful.
    spawn_local(Pipelines::fetch(None).and_then(|pipelines| {
        StatisticsView::update(&TotalPipelines(pipelines.len()));
        futures::future::ok(())
    }));

    spawn_local(TaskStatuses::fetch().and_then(|tasks| {
        let running_count = tasks.iter().filter(|task| task.is_running()).count();
        let failed_count = tasks.iter().filter(|task| task.is_failed()).count();

        StatisticsView::update(&RunningTasks(running_count));
        StatisticsView::update(&FailedTasks(failed_count));
        futures::future::ok(())
    }));

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_readme_deps() {
        version_sync::assert_markdown_deps_updated!("README.md");
    }

    #[test]
    fn test_html_root_url() {
        version_sync::assert_html_root_url_updated!("src/lib.rs");
    }
}
