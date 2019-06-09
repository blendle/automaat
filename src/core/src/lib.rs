//! # Automaat
//!
//! Automaat can help you automate mundane and repeated tasks in a flexible way.
//!
//! Its goal is to provide a simplified, user-friendly, and highly-customisable
//! interface that combines "customer support" software, job schedulers and
//! ad-hoc shell scripts you might currently be using at your organisation.
//!
//! Automaat consists of several core crates:
//!
//! * [`automaat-core`][c] (this one) – Provides the basic building blocks for
//!   the functionality of the other crates.
//! * [`automaat-server`][s] – A server application, with an API to run
//!   processors, and persistent storage.
//! * [`automaat-web-client`][w] – A WebAssembly-based application to interact
//!   with the server, and run processors.
//!
//! [c]: https://docs.rs/automaat-core
//! [s]: https://docs.rs/automaat-server
//! [w]: https://docs.rs/automaat-web-client
//!
//! There are also serveral existing processor implementations, each in their
//! own crate:
//!
//! * [`automaat-processor-git-clone`][pg] – Clone any Git repository to the
//!   processor workspace.
//! * [`automaat-processor-shell-command`][ps] – Execute a shell command.
//! * [`automaat-processor-redis-command`][pr] – Execute a Redis command.
//! * [`automaat-processor-string-regex`][px] – Match (and replace) a string.
//! * [`automaat-processor-print-output`][po] – Return a pre-defined string.
//!
//! Using the `automaat-server` crate, you can combine multiple processors into
//! a single `Pipeline`, combined with a set of runtime `Variable`s, to create
//! easy-to-use workflows to perform a specific task.
//!
//! [pg]: https://docs.rs/automaat-processor-git-clone
//! [ps]: https://docs.rs/automaat-processor-shell-command
//! [pr]: https://docs.rs/automaat-processor-redis-command
//! [px]: https://docs.rs/automaat-processor-string-regex
//! [po]: https://docs.rs/automaat-processor-print-output
//!
//! # Core
//!
//! This crate, `automaat-core`, provides the main [`Processor`] trait to create
//! new processors, and run them.
//!
//! It also provides access to the [`Context`] object, to share state between
//! multiple processors in a single run.
//!
//! If you want to write your own processor, be sure to check out the
//! documentation of the [`Processor`] trait.

#![deny(
    clippy::all,
    clippy::cargo,
    clippy::nursery,
    clippy::pedantic,
    rust_2018_idioms,
    warnings
)]
#![allow(clippy::multiple_crate_versions)]

use serde::{Deserialize, Serialize};
use std::{error, fmt, io, path};
use tempfile::{tempdir, TempDir};

/// The main trait to implement when creating a new Automaat processor.
///
/// Implementing the `Processor` trait makes it possible to use that processor
/// in the `automaat-server` application.
pub trait Processor<'de>: Clone + fmt::Debug + Serialize + Deserialize<'de> {
    /// The human-formatted name of the processor, used to visually identify
    /// this processor amongst others.
    const NAME: &'static str;

    /// If a processor fails its intended purpose, the returned error is turned
    /// into a string, and shown in the `automaat-web-client` application.
    type Error: error::Error;

    /// The processor can return any (successful) output it wants, as long as
    /// that type implements the [`std::fmt::Display`] trait.
    ///
    /// In the `automaat-server` application, the output is turned into a
    /// string, and is processed as markdown.
    ///
    /// While not required, it's best-practice to take advantage of this fact,
    /// to format the output in a pleasant way for users.
    type Output: fmt::Display;

    /// Actually runs the pipeline, performing whatever side-effects are defined
    /// in this specific processor.
    ///
    /// The [`Context`] object can be used to access a temporary workspace
    /// directory that is shared across all processors using the same context
    /// object.
    ///
    /// # Errors
    ///
    /// When a processor has run to completion, it is supposed to return
    /// whatever valuable information could be used via `Self::Output`. If an
    /// unexpected result occurred, `Self::Error` should be returned.
    fn run(&self, context: &Context) -> Result<Option<Self::Output>, Self::Error>;

    /// The `validate` method is used by the `automaat-server` application to do
    /// a runtime check to make sure that the processor is correctly configured
    /// before running it.
    ///
    /// This is an additional validation, on top of whatever invariant is
    /// guaranteed using the type system.
    ///
    /// The default implementation of this method always returns `Ok`.
    ///
    /// # Errors
    ///
    /// If validation fails, an error should be returned. The error message can
    /// be used by clients such as `automaat-web-client` to show an informative
    /// message to the user.
    fn validate(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// The `Context` is an object that can be shared across multiple processor runs
/// for any required shared state.
///
/// At the moment, it is used to provide a shared location on the local
/// file system to store and retrieve data from.
#[derive(Debug)]
pub struct Context {
    workspace: TempDir,
}

impl Context {
    /// Create a new `Context` object.
    ///
    /// # Errors
    ///
    /// If the file system cannot be written to, or something else prevents the
    /// temporary directory from being created, a [`ContextError`] enum is
    /// returned. Specifically the `ContextError::Io` variant.
    pub fn new() -> Result<Self, ContextError> {
        Ok(Self {
            workspace: tempdir()?,
        })
    }

    /// Returns a [`std::path::Path`] reference to the shared workspace.
    pub fn workspace_path(&self) -> &path::Path {
        self.workspace.path()
    }
}

/// Represents all the ways that a [`Context`] can fail.
///
/// This type is not intended to be exhaustively matched, and new variants may
/// be added in the future without a major version bump.
#[derive(Debug)]
pub enum ContextError {
    /// An error occurred during IO activities.
    Io(io::Error),

    #[doc(hidden)]
    __Unknown, // Match against _ instead, more variants may be added in the future.
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ContextError::Io(ref err) => write!(f, "IO error: {}", err),
            ContextError::__Unknown => unreachable!(),
        }
    }
}

impl error::Error for ContextError {
    fn description(&self) -> &str {
        match *self {
            ContextError::Io(ref err) => err.description(),
            ContextError::__Unknown => unreachable!(),
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            ContextError::Io(ref err) => Some(err),
            ContextError::__Unknown => unreachable!(),
        }
    }
}

impl From<io::Error> for ContextError {
    fn from(err: std::io::Error) -> Self {
        ContextError::Io(err)
    }
}
