//! An [Automaat] processor to execute shell commands.
//!
//! Execute shell commands in an Automaat-based workflow. The return value of
//! the shell command is returned as the output of the processor.
//!
//! An optional `stdin` value can be given to provide as the stdin string to the
//! shell command.
//!
//! If the shell command returns a non-zero exit code, the processor returns the
//! _stderr_ output as its error value.
//!
//! All commands are executed within the [`Context`] workspace.
//!
//! [Automaat]: automaat_core
//! [`Context`]: automaat_core::Context
//!
//! # Examples
//!
//! Execute the `echo "hello world"` command in a shell, and return its output.
//!
//! ```rust
//! # fn main() -> Result<(), Box<std::error::Error>> {
//! use automaat_core::{Context, Processor};
//! use automaat_processor_shell_command::ShellCommand;
//!
//! let context = Context::new()?;
//!
//! let processor = ShellCommand {
//!     command: "grep".to_owned(),
//!     arguments: Some(vec!["hello".to_owned()]),
//!     stdin: Some("hello\nworld".to_owned()),
//!     cwd: None,
//!     paths: None,
//! };
//!
//! let output = processor.run(&context)?;
//!
//! assert_eq!(output, Some("hello".to_owned()));
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
#![doc(html_root_url = "https://docs.rs/automaat-processor-shell-command/0.1.0")]

use automaat_core::{Context, Processor};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};
use std::{env, error, fmt, io, path};

/// The processor configuration.
#[cfg_attr(feature = "juniper", derive(juniper::GraphQLObject))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ShellCommand {
    /// The main shell command to execute.
    pub command: String,

    /// The arguments added to the `main` command.
    pub arguments: Option<Vec<String>>,

    /// An optional string passed into to command as _stdin_.
    pub stdin: Option<String>,

    /// The _current working directory_ in which the command is executed.
    ///
    /// This allows you to move to a child path within the [`Context`]
    /// workspace.
    ///
    /// If set to `None`, the root of the workspace is used as the default.
    ///
    /// [`Context`]: automaat_core::Context
    pub cwd: Option<String>,

    /// Optional paths added to the `PATH` environment variable.
    ///
    /// If you have a single script inside the `bin/` directory you want to
    /// execute, you can also use the `cwd` option, but if your scripts call
    /// other custom scripts, and expect them to be directly accessible, you can
    /// add `bin` to `paths` to make that work.
    pub paths: Option<Vec<String>>,
}

/// The GraphQL [Input Object][io] used to initialize the processor via an API.
///
/// [`ShellCommand`] implements `From<Input>`, so you can directly initialize
/// the processor using this type.
///
/// _requires the `juniper` package feature to be enabled_
///
/// [io]: https://graphql.github.io/graphql-spec/June2018/#sec-Input-Objects
#[cfg(feature = "juniper")]
#[graphql(name = "ShellCommandInput")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, juniper::GraphQLInputObject)]
pub struct Input {
    command: String,
    arguments: Option<Vec<String>>,
    stdin: Option<String>,
    cwd: Option<String>,
    paths: Option<Vec<String>>,
}

#[cfg(feature = "juniper")]
impl From<Input> for ShellCommand {
    fn from(input: Input) -> Self {
        Self {
            command: input.command,
            arguments: input.arguments,
            stdin: input.stdin,
            cwd: input.cwd,
            paths: input.paths,
        }
    }
}

impl ShellCommand {
    /// Validate the `ShellCommand` configuration.
    ///
    /// # Errors
    ///
    /// This method returns the [`Error::Path`] error if either the [`cwd`] or
    /// the [`paths`] fields contain anything other than a simple relative path,
    /// such as `my/path`. Anything such as `../`, or `/etc` is not allowed.
    ///
    /// [`cwd`]: ShellCommand::cwd
    /// [`paths`]: ShellCommand::paths
    fn validate(&self) -> Result<(), Error> {
        fn check_path(path: &str) -> Result<(), Error> {
            let path = path::Path::new(path);

            path.components().try_for_each(|c| match c {
                path::Component::Normal(_) => Ok(()),
                _ => Err(Error::Path(
                    "only sibling or child paths are accessible".into(),
                )),
            })
        }

        if let Some(cwd) = &self.cwd {
            check_path(cwd)?;
        };

        if let Some(paths) = &self.paths {
            paths.iter().map(String::as_str).try_for_each(check_path)?;
        }

        Ok(())
    }
}

impl<'a> Processor<'a> for ShellCommand {
    const NAME: &'static str = "Shell Command";

    type Error = Error;
    type Output = String;

    /// Run the shell command as defined by the provided configuration.
    ///
    /// The command will be executed in the [`automaat_core::Context`]
    /// workspace, optionally in a child path using the [`cwd`] option.
    ///
    /// [`cwd`]: ShellCommand::cwd
    ///
    /// # Output
    ///
    /// `None` is returned if the processor runs successfully but no value was
    /// returned by the command on _stdout_.
    ///
    /// `Some` is returned if the command did return a value and exited with
    /// status code `0`.
    ///
    /// If a value is returned, any ANSI escape codes are stripped, and the
    /// return value is transformed lossy transformed into a valid UTF-8 string,
    /// with any invalid bytes transformed to the [replacement character]. Any
    /// whitespace to the right of the output (including newlines) is also
    /// stripped.
    ///
    /// [replacement character]: std::char::REPLACEMENT_CHARACTER
    ///
    /// # Errors
    ///
    /// If the run fails, an [`Error`] result value is returned. The variant can
    /// differ, depending on if the command itself failed, some IO error
    /// happened, or the configuration is invalid.
    fn run(&self, context: &Context) -> Result<Option<Self::Output>, Self::Error> {
        self.validate()?;

        let arguments = match &self.arguments {
            None => vec![],
            Some(v) => v.iter().map(String::as_str).collect(),
        };

        let workspace = context.workspace_path();
        let cwd = workspace.join(path::Path::new(
            self.cwd.as_ref().unwrap_or(&"".to_owned()).as_str(),
        ));

        let new_paths = match self.paths {
            None => vec![],
            Some(ref paths) => paths.iter().map(|p| workspace.join(p)).collect(),
        };

        // Optionally add custom paths to the PATH environment variable.
        let path = match env::var_os("PATH") {
            Some(ref p) => env::split_paths(p).chain(new_paths.into_iter()).collect(),
            None => new_paths,
        };

        let mut command = Command::new(&self.command);
        let command = command
            .current_dir(cwd)
            .env("PATH", env::join_paths(path)?)
            .args(arguments);

        let output = if let Some(input) = &self.stdin {
            let mut spawn = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            spawn.stdin.as_mut().unwrap().write_all(input.as_bytes())?;
            spawn.wait_with_output()
        } else {
            command.output()
        }?;

        if !output.status.success() {
            if output.stderr.is_empty() {
                return Err(Error::Command(
                    "unknown error during command execution".into(),
                ));
            };

            return Err(Error::Command(
                String::from_utf8_lossy(&strip_ansi_escapes::strip(output.stderr)?)
                    .trim_end()
                    .to_owned(),
            ));
        }

        if output.stdout.is_empty() {
            return Ok(None);
        };

        Ok(Some(
            String::from_utf8_lossy(&strip_ansi_escapes::strip(output.stdout)?)
                .trim_end()
                .to_owned(),
        ))
    }
}

/// Represents all the ways that [`ShellCommand`] can fail.
///
/// This type is not intended to be exhaustively matched, and new variants may
/// be added in the future without a major version bump.
#[derive(Debug)]
pub enum Error {
    /// The command execution failed.
    ///
    /// This happens if the command returns with a non-zero exit code.
    ///
    /// The string value represents the _stderr_ output of the command.
    Command(String),

    /// An I/O operation failed.
    ///
    /// This is a wrapper around [`std::io::Error`].
    Io(io::Error),

    /// The provided [`ShellCommand::paths`] or [`ShellCommand::cwd`]
    /// configuration is invalid.
    Path(String),

    #[doc(hidden)]
    __Unknown, // Match against _ instead, more variants may be added in the future.
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Command(ref err) => write!(f, "Command error: {}", err),
            Error::Io(ref err) => write!(f, "IO error: {}", err),
            Error::Path(ref err) => write!(f, "Path error: {}", err),
            Error::__Unknown => unreachable!(),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::Command(_) | Error::Path(_) => None,
            Error::Io(ref err) => Some(err),
            Error::__Unknown => unreachable!(),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<env::JoinPathsError> for Error {
    fn from(err: env::JoinPathsError) -> Self {
        Error::Path(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn processor_stub() -> ShellCommand {
        ShellCommand {
            command: "echo".to_owned(),
            arguments: None,
            stdin: None,
            cwd: None,
            paths: None,
        }
    }

    mod run {
        use super::*;

        #[test]
        fn test_command_without_output() {
            let mut processor = processor_stub();
            processor.command = "true".to_owned();

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap();

            assert!(output.is_none())
        }

        #[test]
        fn test_command_with_output() {
            let mut processor = processor_stub();
            processor.command = "ps".to_owned();

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert!(output.contains("PID"))
        }

        #[test]
        fn test_command_with_input() {
            let mut processor = processor_stub();
            processor.command = "cat".to_owned();
            processor.stdin = Some("hello world".to_owned());

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert!(output.contains("hello world"))
        }

        #[test]
        fn test_command_with_arguments() {
            let mut processor = processor_stub();
            processor.command = "echo".to_owned();
            processor.arguments = Some(vec!["hello world".to_owned()]);

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert_eq!(output, "hello world".to_owned())
        }

        #[test]
        #[should_panic]
        fn test_command_non_zero_exit_code() {
            let mut processor = processor_stub();
            processor.command = "false".to_owned();

            let context = Context::new().unwrap();
            let _ = processor.run(&context).unwrap();
        }

        #[test]
        fn test_command_stderr_output() {
            let mut processor = processor_stub();
            processor.command = "ls".to_owned();
            processor.arguments = Some(vec!["invalid-file".to_owned()]);

            let context = Context::new().unwrap();
            let error = processor.run(&context).unwrap_err();

            assert!(error.to_string().contains("Command error"))
        }

        #[test]
        fn test_invalid_command() {
            let mut processor = processor_stub();
            processor.command = "doesnotexist".to_owned();

            let context = Context::new().unwrap();
            let error = processor.run(&context).unwrap_err();

            assert_eq!(
                error.to_string(),
                "IO error: No such file or directory (os error 2)".to_owned()
            )
        }

        #[test]
        fn test_appending_paths() {
            let mut processor = processor_stub();
            processor.command = "printenv".to_owned();
            processor.arguments = Some(vec!["PATH".to_owned()]);
            processor.paths = Some(vec!["hello/world".to_owned()]);

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert!(output.contains(&format!(
                ":{}",
                context
                    .workspace_path()
                    .join("hello/world")
                    .to_string_lossy()
            )));
        }
    }

    mod validate {
        use super::*;

        #[test]
        fn test_no_cwd() {
            let mut processor = processor_stub();
            processor.cwd = None;

            processor.validate().unwrap()
        }

        #[test]
        fn test_relative_cwd() {
            let mut processor = processor_stub();
            processor.cwd = Some("hello/world".to_owned());

            processor.validate().unwrap()
        }

        #[test]
        #[should_panic]
        fn test_prefix_cwd() {
            let mut processor = processor_stub();
            processor.cwd = Some("../parent".to_owned());

            processor.validate().unwrap()
        }

        #[test]
        #[should_panic]
        fn test_absolute_cwd() {
            let mut processor = processor_stub();
            processor.cwd = Some("/etc".to_owned());

            processor.validate().unwrap()
        }

        #[test]
        fn test_no_paths() {
            let mut processor = processor_stub();
            processor.paths = None;

            processor.validate().unwrap()
        }

        #[test]
        fn test_relative_paths() {
            let mut processor = processor_stub();
            processor.paths = Some(vec!["hello/world".to_owned()]);

            processor.validate().unwrap()
        }

        #[test]
        fn test_multiple_valid_paths() {
            let mut processor = processor_stub();
            processor.paths = Some(vec!["valid/path".to_owned(), "another/path".to_owned()]);

            processor.validate().unwrap()
        }

        #[test]
        #[should_panic]
        fn test_prefix_paths() {
            let mut processor = processor_stub();
            processor.paths = Some(vec!["../parent".to_owned()]);

            processor.validate().unwrap()
        }

        #[test]
        #[should_panic]
        fn test_absolute_paths() {
            let mut processor = processor_stub();
            processor.paths = Some(vec!["/etc".to_owned()]);

            processor.validate().unwrap()
        }

        #[test]
        #[should_panic]
        fn test_multiple_paths_one_bad() {
            let mut processor = processor_stub();
            processor.paths = Some(vec!["valid/path".to_owned(), "/etc".to_owned()]);

            processor.validate().unwrap()
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
