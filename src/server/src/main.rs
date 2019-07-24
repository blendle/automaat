//! An [Automaat] server implementation.
//!
//! This server performs several critical tasks in the Automaat workflow:
//!
//! * Use a persistent database to store the state of the Automaat instance.
//! * Expose a GraphQL API to fetch and create tasks
//! * Adds some abstractions (such as "tasks") for ease of use.
//!
//! By combining this server with `automaat-web-client`, you can have your own
//! Automaat instance running in your environment.
//!
//! [Automaat]: https://docs.rs/automaat-core
#![deny(
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
    missing_copy_implementations
)]
#![warn(variant_size_differences)]
#![allow(clippy::multiple_crate_versions, missing_doc_code_examples)]
#![doc(html_root_url = "https://docs.rs/automaat-server/0.1.0")]

// This is needed for statically linking.
//
// see: https://git.io/fj2CG
#[allow(unused_extern_crates)]
extern crate openssl;

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

#[macro_use]
extern crate diesel_derive_enum;

mod graphql;
mod handlers;
mod middleware;
mod models;
mod processor;
mod resources;
mod schema;
mod server;
mod worker;

use crate::processor::{Input as ProcessorInput, Processor};
use crate::server::Server;
use crate::worker::Worker;
use diesel_migrations::embed_migrations;
use std::env;

lazy_static::lazy_static! {
    static ref ENCRYPTION_SECRET: String = env::var("ENCRYPTION_SECRET")
        .expect("ENCRYPTION_SECRET environment variable not set");
}

fn main() {
    // Make sure encryption secret is set by loading it once.
    let _ = &ENCRYPTION_SECRET.to_string();

    let args: Vec<String> = env::args().collect();
    let run = || match args.get(1).map(String::as_str) {
        Some("server") => Server::from_environment()?.run_to_completion(),
        Some("worker") => Worker::from_environment()?.run_to_completion(),
        _ => Err("usage: automaat [server|worker]".into()),
    };

    if let Err(err) = run() {
        println!("{}", err)
    }
}

// Embeds all migrations inside the binary, so that they can be run when needed
// on startup.
embed_migrations!();

#[cfg(test)]
mod tests {
    #[test]
    fn test_readme_deps() {
        version_sync::assert_markdown_deps_updated!("README.md");
    }

    #[test]
    fn test_html_root_url() {
        version_sync::assert_html_root_url_updated!("src/main.rs");
    }
}
