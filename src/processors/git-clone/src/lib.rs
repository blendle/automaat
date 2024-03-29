//! An [Automaat] processor to clone a Git repository.
//!
//! Using this crate in your Automaat workflow allows you to clone an external
//! repository into the [`Context`] workspace.
//!
//! Plaintext username/password authentication is supported for private
//! repositories.
//!
//! [Automaat]: https://docs.rs/automaat-core
//! [`Context`]: automaat_core::Context
//!
//! # Examples
//!
//! Clone the Automaat repository into the workspace of the created context, and
//! assert that the repository is in the correct location.
//!
//! Since this repository is open to the public, no credentials are required.
//!
//! The workspace is a temporary directory created on your file system. See the
//! [`Context`] documentation for more details.
//!
//! ```rust
//! # fn main() -> Result<(), Box<std::error::Error>> {
//! use automaat_core::{Context, Processor};
//! use automaat_processor_git_clone::GitClone;
//!
//! let context = Context::new()?;
//!
//! let processor = GitClone {
//!     url: "https://github.com/blendle/automaat".to_owned(),
//!     username: None,
//!     password: None,
//!     path: Some("automaat-repo".to_owned())
//! };
//!
//! processor.run(&context)?;
//!
//! assert!(context.workspace_path().join("automaat-repo/README.md").exists());
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
#![allow(clippy::multiple_crate_versions, missing_doc_code_examples)]
#![doc(html_root_url = "https://docs.rs/automaat-processor-git-clone/0.1.0")]

use automaat_core::{Context, Processor};
use git2::{build::RepoBuilder, Cred, FetchOptions, RemoteCallbacks};
use serde::{Deserialize, Serialize};
use std::{error, fmt, path, str::FromStr};
use url::Url;

/// The processor configuration.
#[cfg_attr(feature = "juniper", derive(juniper::GraphQLObject))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GitClone {
    /// The URL of the remote to fetch the repository from.
    pub url: String,

    /// The optional username used to authenticate with the remote.
    pub username: Option<String>,

    /// The optional password used to authenticate with the remote.
    pub password: Option<String>,

    /// An optional path inside the workspace to clone the repository to. If no
    /// path is given, the root of the workspace is used. If the path does not
    /// exist, it will be created.
    pub path: Option<String>,
}

/// The GraphQL [Input Object][io] used to initialize the processor via an API.
///
/// [`GitClone`] implements `From<Input>`, so you can directly initialize the
/// processor using this type.
///
/// _requires the `juniper` package feature to be enabled_
///
/// [io]: https://graphql.github.io/graphql-spec/June2018/#sec-Input-Objects
#[cfg(feature = "juniper")]
#[graphql(name = "GitCloneInput")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, juniper::GraphQLInputObject)]
pub struct Input {
    url: String,
    username: Option<String>,
    password: Option<String>,
    path: Option<String>,
}

#[cfg(feature = "juniper")]
impl From<Input> for GitClone {
    fn from(input: Input) -> Self {
        Self {
            username: input.username,
            password: input.password,
            url: input.url,
            path: input.path,
        }
    }
}

impl GitClone {
    /// Convert the string URL into a URL object.
    fn url(&self) -> Result<Url, Error> {
        Url::from_str(&self.url).map_err(Into::into)
    }

    /// Validate the `GitClone` configuration.
    ///
    /// # Errors
    ///
    /// This method returns an error under the following circumstances:
    ///
    /// * If the URL is an invalid format, the [`Error::Url`] error variant is
    ///   returned.
    ///
    /// * If a `path` option is provided that contains anything other than a
    ///   simple relative path such as `my/path`. Anything such as `../`, or
    ///   `/etc` is not allowed. The returned error is of type [`Error::Path`].
    ///
    /// In a future update, this will also validate remote connectivity.
    fn validate(&self) -> Result<(), Error> {
        let _ = self.url()?;

        if let Some(path) = &self.path {
            let path = path::Path::new(path);

            path.components().try_for_each(|c| match c {
                path::Component::Normal(_) => Ok(()),
                _ => Err(Error::Path),
            })?;
        };

        Ok(())
    }
}

impl<'a> Processor<'a> for GitClone {
    const NAME: &'static str = "Git Clone";

    type Error = Error;
    type Output = String;

    /// Clone the repository as defined by the provided configuration.
    ///
    /// The repository will be cloned in the [`automaat_core::Context`]
    /// workspace, optionally in a child `path`.
    ///
    /// # Output
    ///
    /// `None` is returned if the processor runs successfully.
    ///
    /// # Errors
    ///
    /// Any errors during cloning will return an [`Error::Git`] result value.
    fn run(&self, context: &Context) -> Result<Option<Self::Output>, Self::Error> {
        self.validate()?;

        let mut callbacks = RemoteCallbacks::new();
        let mut fetch_options = FetchOptions::new();
        let workspace = context.workspace_path();
        let path = self
            .path
            .as_ref()
            .map_or_else(|| workspace.into(), |path| workspace.join(path));

        if let (Some(u), Some(p)) = (&self.username, &self.password) {
            let _ = callbacks.credentials(move |_, _, _| Cred::userpass_plaintext(u, p));
            let _ = fetch_options.remote_callbacks(callbacks);
        };

        RepoBuilder::new()
            .fetch_options(fetch_options)
            .clone(self.url.as_str(), &path)
            .map(|_| None)
            .map_err(Into::into)
    }
}

/// Represents all the ways that [`GitClone`] can fail.
///
/// This type is not intended to be exhaustively matched, and new variants may
/// be added in the future without a major version bump.
#[derive(Debug)]
pub enum Error {
    /// The provided [`GitClone::path`] configuration is invalid.
    Path,

    /// An error occurred while cloning the Git repository.
    Git(git2::Error),

    /// The URL has an invalid format.
    Url(url::ParseError),

    #[doc(hidden)]
    __Unknown, // Match against _ instead, more variants may be added in the future.
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Path => write!(f, "Path error: invalid path location"),
            Error::Git(ref err) => write!(f, "Git error: {}", err),
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
            Error::Path => None,
            Error::Git(ref err) => Some(err),
            Error::Url(ref err) => Some(err),
            Error::__Unknown => unreachable!(),
        }
    }
}

impl From<git2::Error> for Error {
    fn from(err: git2::Error) -> Self {
        Error::Git(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn processor_stub() -> GitClone {
        GitClone {
            username: None,
            password: None,
            url: "http://127.0.0.1".to_owned(),
            path: None,
        }
    }

    mod validate {
        use super::*;

        #[test]
        fn test_no_path() {
            let mut processor = processor_stub();
            processor.path = None;

            processor.validate().unwrap()
        }

        #[test]
        fn test_relative_path() {
            let mut processor = processor_stub();
            processor.path = Some("hello/world".to_owned());

            processor.validate().unwrap()
        }

        #[test]
        #[should_panic]
        fn test_prefix_path() {
            let mut processor = processor_stub();
            processor.path = Some("../parent".to_owned());

            processor.validate().unwrap()
        }

        #[test]
        #[should_panic]
        fn test_absolute_path() {
            let mut processor = processor_stub();
            processor.path = Some("/etc".to_owned());

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
