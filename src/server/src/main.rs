//! An [Automaat] server implementation.
//!
//! This server performs several critical tasks in the Automaat workflow:
//!
//! * Use a persistent database to store the state of the Automaat instance.
//! * Expose a GraphQL API to fetch and create tasks
//! * Adds some abstractions (such as "pipelines") for ease of use.
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
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

#[macro_use]
extern crate diesel_derive_enum;

mod graphql;
mod handlers;
mod processor;
mod resources;
mod schema;

use crate::graphql::{MutationRoot, QueryRoot, Schema};
use crate::processor::{Input as ProcessorInput, Processor};
use actix_files::Files;
use actix_web::{
    middleware::{Compress, DefaultHeaders},
    web, App, HttpServer,
};
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel_migrations::embed_migrations;
use std::{env, io, ops::Deref, sync::Arc, thread};

/// The main database connection pool shared across all threads.
pub(crate) struct Database(PooledConnection<ConnectionManager<PgConnection>>);

impl Deref for Database {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(crate) type DatabasePool = Pool<ConnectionManager<PgConnection>>;
pub(crate) type GraphQLSchema = Arc<Schema>;

fn main() -> io::Result<()> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL environment variable");
    let pool = Pool::new(ConnectionManager::new(database_url)).expect("valid database pool");

    let conn = Database(pool.get().expect("valid database connection"));
    embedded_migrations::run(&*conn).expect("successful database migration");
    run_task_runner(conn);

    server(pool)
}

fn server(pool: DatabasePool) -> io::Result<()> {
    let bind = env::var("SERVER_BIND").unwrap_or_else(|_| "0.0.0.0:8000".to_owned());
    let schema = Arc::new(Schema::new(QueryRoot, MutationRoot));

    HttpServer::new(move || {
        let root = env::var("SERVER_ROOT").unwrap_or_else(|_| "/public".to_owned());

        App::new()
            .wrap(Compress::default())
            .wrap(DefaultHeaders::new().header("cache-control", "max-age=900"))
            .data(pool.clone())
            .data(schema.clone())
            .route("/graphql/playground", web::get().to(handlers::playground))
            .route("/graphql/graphiql", web::get().to(handlers::graphiql))
            .route("/graphql", web::get().to_async(handlers::graphql))
            .route("/graphql", web::post().to_async(handlers::graphql))
            .route("/health", web::get().to(handlers::health))
            .service(Files::new("/", root).index_file("index.html"))
    })
    .bind(bind)
    .unwrap()
    .run()
}

// Embeds all migrations inside the binary, so that they can be run when needed
// on startup.
embed_migrations!();

// Takes a permanent database connection from the connection pool and starts a
// new thread to continuously poll the database for new tasks that need to run.
//
// Currently there is no way for the thread to signal a panic situation to the
// main thread, so if this thread dies because of a bug, new tasks won't run
// anymore, but the server will keep running.
//
// TODO: split this off into its own crate. Possibly look into using Faktory to
// schedule jobs.
fn run_task_runner(conn: Database) {
    let _ = thread::spawn(move || crate::resources::poll_tasks(&conn));
}

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
