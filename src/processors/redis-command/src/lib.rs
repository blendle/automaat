//! An [Automaat] processor to execute Redis commands.
//!
//! Execute Redis commands in an Automaat-based workflow. The return value of
//! the Redis command is returned as the output of the processor.
//!
//! [Automaat]: automaat_core
//!
//! # Examples
//!
//! Execute the Redis `PING` command, with the "hello world" argument, and
//! receive the response back as the output of the run.
//!
//! See the [official documentation on `PING`][ping].
//!
//! [ping]: https://redis.io/commands/ping
//!
//! ```rust
//! # fn main() -> Result<(), Box<std::error::Error>> {
//! use automaat_core::{Context, Processor};
//! use automaat_processor_redis_command::RedisCommand;
//!
//! let context = Context::new()?;
//!
//! let processor = RedisCommand {
//!     command: "PING".to_owned(),
//!     arguments: Some(vec!["hello world".to_owned()]),
//!     url: "redis://127.0.0.1".to_owned(),
//! };
//!
//! let output = processor.run(&context)?;
//!
//! assert_eq!(output, Some("hello world".to_owned()));
//! #     Ok(())
//! # }
//! ```
//!
//! # Package Features
//!
//! * `juniper` – creates a set of objects to be used in GraphQL-based
//!   requests/responses.
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
    missing_debug_implementations,
    missing_copy_implementations
)]
#![warn(variant_size_differences)]
#![allow(clippy::multiple_crate_versions, missing_doc_code_examples)]
#![doc(html_root_url = "https://docs.rs/automaat-processor-redis-command/0.1.0")]

use automaat_core::{Context, Processor};
use redis::RedisError;
use serde::{Deserialize, Serialize};
use std::{error, fmt, str::from_utf8, str::FromStr};
use url::Url;

/// The processor configuration.
#[cfg_attr(feature = "juniper", derive(juniper::GraphQLObject))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RedisCommand {
    /// The main Redis command to execute.
    ///
    /// See the [main Redis documentation] for a list of available commands.
    ///
    /// [main Redis documentation]: https://redis.io/commands
    pub command: String,

    /// The arguments belonging to the main `command`.
    ///
    /// The acceptable value of these arguments depends on the command being
    /// executed.
    pub arguments: Option<Vec<String>>,

    /// The URL of the Redis server.
    ///
    /// See the [redis-rs] "connection parameters" documentation for more
    /// details.
    ///
    /// [redis-rs]: https://docs.rs/redis/latest/redis#connection-parameters
    pub url: String,
}

/// The GraphQL [Input Object][io] used to initialize the processor via an API.
///
/// [`RedisCommand`] implements `From<Input>`, so you can directly initialize
/// the processor using this type.
///
/// _requires the `juniper` package feature to be enabled_
///
/// [io]: https://graphql.github.io/graphql-spec/June2018/#sec-Input-Objects
#[cfg(feature = "juniper")]
#[graphql(name = "RedisCommandInput")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, juniper::GraphQLInputObject)]
pub struct Input {
    command: String,
    arguments: Option<Vec<String>>,
    url: String,
}

#[cfg(feature = "juniper")]
impl From<Input> for RedisCommand {
    fn from(input: Input) -> Self {
        Self {
            command: input.command,
            arguments: input.arguments,
            url: input.url,
        }
    }
}

impl<'a> Processor<'a> for RedisCommand {
    const NAME: &'static str = "Redis Command";

    type Error = Error;
    type Output = String;

    /// Run the configured Redis command, and return its results.
    ///
    /// # Output
    ///
    /// The value returned by the Redis server is fairly untyped, and not always
    /// easily represented in the final output. In general, the most common
    /// values are correctly mapped, such as `Nil` becoming `None`, and all
    /// valid UTF-8 data is returned as `Some`, containing the data as a string.
    ///
    /// Any value that cannot be coerced into a valid UTF-8 string, is
    /// represented in the best possible way as a valid UTF-8 string, but won't
    /// completely match the original output of Redis.
    ///
    /// # Errors
    ///
    /// See the [`Error`] enum for all possible error values that can be
    /// returned. These values wrap the [`redis::ErrorKind`] values.
    fn run(&self, _context: &Context) -> Result<Option<Self::Output>, Self::Error> {
        use redis::Value;

        let url = Url::from_str(&self.url)?;
        let client = redis::Client::open(url.as_str())?;
        let conn = client.get_connection()?;
        let args = self.arguments.clone().unwrap_or_else(Default::default);

        redis::cmd(self.command.as_str())
            .arg(args)
            .query(&conn)
            .map_err(Into::into)
            .map(|v| match v {
                Value::Nil => None,
                Value::Status(string) => Some(string),
                Value::Data(ref val) => match from_utf8(val) {
                    Ok(string) => Some(string.to_owned()),
                    Err(_) => Some(format!("{:?}", val)),
                },
                other => Some(format!("{:?}", other)),
            })
    }
}

/// Represents all the ways that [`RedisCommand`] can fail.
///
/// This type is not intended to be exhaustively matched, and new variants may
/// be added in the future without a major version bump.
#[derive(Debug)]
pub enum Error {
    /// The server generated an invalid response.
    Response(RedisError),

    /// The authentication with the server failed.
    AuthenticationFailed(RedisError),

    /// Operation failed because of a type mismatch.
    Type(RedisError),

    /// A script execution was aborted.
    ExecAbort(RedisError),

    /// The server cannot response because it's loading a dump.
    BusyLoading(RedisError),

    /// A script that was requested does not actually exist.
    NoScript(RedisError),

    /// An error that was caused because the parameter to the client were wrong.
    InvalidClientConfig(RedisError),

    /// This kind is returned if the redis error is one that is not native to
    /// the system. This is usually the case if the cause is another error.
    Io(RedisError),

    /// An extension error. This is an error created by the server that is not
    /// directly understood by the library.
    Extension(RedisError),

    /// The URL has an invalid format.
    Url(url::ParseError),

    #[doc(hidden)]
    __Unknown, // Match against _ instead, more variants may be added in the future.
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Response(ref err)
            | Error::AuthenticationFailed(ref err)
            | Error::Type(ref err)
            | Error::ExecAbort(ref err)
            | Error::BusyLoading(ref err)
            | Error::NoScript(ref err)
            | Error::InvalidClientConfig(ref err)
            | Error::Io(ref err)
            | Error::Extension(ref err) => write!(f, "Redis error: {}", err),
            Error::Url(ref err) => write!(f, "URL error: {}", err),
            Error::__Unknown => unreachable!(),
        }
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::Url(err)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::Response(ref err)
            | Error::AuthenticationFailed(ref err)
            | Error::Type(ref err)
            | Error::ExecAbort(ref err)
            | Error::BusyLoading(ref err)
            | Error::NoScript(ref err)
            | Error::InvalidClientConfig(ref err)
            | Error::Io(ref err)
            | Error::Extension(ref err) => Some(err),
            Error::Url(ref err) => Some(err),
            Error::__Unknown => unreachable!(),
        }
    }
}

impl From<RedisError> for Error {
    fn from(err: RedisError) -> Self {
        use redis::ErrorKind;

        match err.kind() {
            ErrorKind::ResponseError => Error::Response(err),
            ErrorKind::AuthenticationFailed => Error::AuthenticationFailed(err),
            ErrorKind::TypeError => Error::Type(err),
            ErrorKind::ExecAbortError => Error::ExecAbort(err),
            ErrorKind::BusyLoadingError => Error::BusyLoading(err),
            ErrorKind::NoScriptError => Error::NoScript(err),
            ErrorKind::InvalidClientConfig => Error::InvalidClientConfig(err),
            ErrorKind::IoError => Error::Io(err),
            ErrorKind::ExtensionError => Error::Extension(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn processor_stub() -> RedisCommand {
        RedisCommand {
            command: "PING".to_owned(),
            arguments: None,
            url: "redis://127.0.0.1".to_owned(),
        }
    }

    mod run {
        use super::*;

        #[test]
        fn test_command() {
            let mut processor = processor_stub();
            processor.command = "PING".to_owned();

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap();

            assert_eq!(output, Some("PONG".to_owned()))
        }

        #[test]
        fn test_command_and_arguments() {
            let mut processor = processor_stub();
            processor.command = "PING".to_owned();
            processor.arguments = Some(vec!["hello world".to_owned()]);

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap();

            assert_eq!(output, Some("hello world".to_owned()))
        }

        #[test]
        fn test_unknown_command() {
            let mut processor = processor_stub();
            processor.command = "UNKNOWN".to_owned();

            let context = Context::new().unwrap();
            let error = processor.run(&context).unwrap_err();

            assert!(error.to_string().contains("unknown command `UNKNOWN`"));
        }
    }

    #[test]
    fn test_readme_deps() {
        version_sync::assert_markdown_deps_updated!("README.md");
    }

    #[test]
    fn test_html_root_url() {
        version_sync::assert_html_root_url_updated!("src/lib.rs");
    }
}
