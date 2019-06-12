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

mod graphql;
mod handlers;
mod processor;
mod resources;
mod schema;

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate rocket_contrib;

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

#[macro_use]
extern crate diesel_derive_enum;

use crate::graphql::{MutationRoot, QueryRoot, Schema};
use diesel_migrations::embed_migrations;
use processor::{Input as ProcessorInput, Processor};
use rocket::{fairing::AdHoc, Rocket};
use rocket_contrib::databases::diesel::PgConnection;
use std::thread;

/// The main database connection pool shared across all threads.
#[database("db")]
pub(crate) struct Database(PgConnection);

fn main() {
    let _ = server().launch();
}

/// Creates a Rocket server instance including all the CORS configuration, and
/// the required "attachments":
///
/// * Running the database migrations before the server starts.
/// * Running the "task runner" in a separate thread.
///
/// This does not boot the server, so that this function can be used in
/// integration test scenarios that boot an internal server when a test runs.
fn server() -> Rocket {
    let cors = rocket_cors::CorsOptions::default()
        .to_cors()
        .expect("invalid CORS");

    rocket::ignite()
        .attach(Database::fairing())
        .manage(Schema::new(QueryRoot, MutationRoot))
        .attach(AdHoc::on_attach("Database Migrations", run_db_migrations))
        .attach(AdHoc::on_attach("Starting Task Runner...", run_task_runner))
        .attach(cors)
        .mount(
            "/",
            routes![
                handlers::graphiql,
                handlers::playground,
                handlers::health,
                handlers::query,
                handlers::mutate
            ],
        )
}

// Embeds all migrations inside the binary, so that they can be run when needed
// on startup.
embed_migrations!();

/// Takes a connection from the database connection pool, and runs any pending
/// migrations before starting the web server.
///
/// If any of the migrations fail to run, the server is not started.
fn run_db_migrations(rocket: Rocket) -> Result<Rocket, Rocket> {
    let conn = Database::get_one(&rocket).expect("database connection");
    match embedded_migrations::run(&*conn) {
        Ok(_) => Ok(rocket),
        Err(e) => {
            eprintln!("{}", e);
            Err(rocket)
        }
    }
}

// Takes a permanent database connection from the connection pool and starts a
// new thread to continuously poll the database for new tasks that need to run.
//
// Currently there is no way for the thread to signal a panic situation to the
// main thread, so if this thread dies because of a bug, new tasks won't run
// anymore, but the server will keep running.
//
// TODO: split this off into its own crate. Possibly look into using Faktory to
// schedule jobs.
fn run_task_runner(rocket: Rocket) -> Result<Rocket, Rocket> {
    let conn = Database::get_one(&rocket).expect("database connection");
    let _ = thread::spawn(move || crate::resources::poll_tasks(&conn));

    Ok(rocket)
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
