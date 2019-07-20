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
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel_migrations::embed_migrations;
use std::sync::Arc;
use std::{env, ops::Deref};

// TODO: rename `Database` to `State` and move this into the state object,
// passing it along when needed.
//
// TODO: when we have proper logging, warn when no secret is provided,
// potentially refuse to start in non-debug mode.
lazy_static::lazy_static! {
    static ref SERVER_SECRET: String = env::var("SERVER_SECRET")
        .unwrap_or_else(|_| "default secret".to_owned());
}

/// The main database connection pool shared across all threads.
pub(crate) struct Database(PooledConnection<ConnectionManager<PgConnection>>);

impl Deref for Database {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// pub(crate) struct Config {
//     pub(crate) encryption_token: String,
// }

// TODO: enable this in a separate commit
// let config = Config {
//     encryption_token: SERVER_SECRET.to_string(),
// };

pub(crate) struct State {
    pub(crate) pool: DatabasePool,
}

pub(crate) type DatabasePool = Pool<ConnectionManager<PgConnection>>;
pub(crate) type GraphQLSchema = Arc<graphql::Schema>;

fn main() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL environment variable");
    let pool = Pool::new(ConnectionManager::new(database_url)).expect("valid database pool");

    let db = Database(pool.get().expect("valid database connection"));
    embedded_migrations::run(&*db).expect("successful database migration");

    let state = State { pool };

    let args: Vec<String> = env::args().collect();
    let result = match args.get(1).map(String::as_str) {
        Some("server") => Server::new(state).run_to_completion(),
        Some("worker") => Worker::new(state).run_to_completion(),
        _ => Err("usage: automaat [server|worker]".into()),
    };

    if let Err(err) = result {
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
