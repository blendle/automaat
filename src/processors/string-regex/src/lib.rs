//! An [Automaat] processor to match and replace strings using regex patterns.
//!
//! This processor allows you to match against strings in an Automaat workflow,
//! and either show an error if the pattern doesn't match, or replace the
//! pattern with a replacement string.
//!
//! It is great for transforming the output of the previous processor into
//! something that is more readable for the user, before printing it to the
//! screen using the [`PrintOutput`] processor.
//!
//! [Automaat]: automaat_core
//! [`PrintOutput`]: https://docs.rs/automaat-processor-print-output
//!
//! # Examples
//!
//! ## Replace input based on regex pattern
//!
//! One common example of this processor is to use it after another processor
//! ran, which provided some output that needs to be rewritten before it is used
//! by the next processor (or presented to the user).
//!
//! In this example, we get a string `Failure #233 - email does not exist`. We
//! want to rewrite this output to show `error: email does not exist`.
//!
//! ```rust
//! # fn main() -> Result<(), Box<std::error::Error>> {
//! use automaat_core::{Context, Processor};
//! use automaat_processor_string_regex::StringRegex;
//!
//! let context = Context::new()?;
//!
//! let processor = StringRegex {
//!     input: "Failure #233 - email does not exist".to_owned(),
//!     regex: r"\A[^-]+ - (.*)\z".to_owned(),
//!     mismatch_error: None,
//!     replace: Some("error: $1".to_owned())
//! };
//!
//! let output = processor.run(&context)?;
//!
//! assert_eq!(output, Some("error: email does not exist".to_owned()));
//! #     Ok(())
//! # }
//! ```
//!
//! ## Return error on regex mismatch
//!
//! Another common use-case is to match against some input, and return an error
//! if the pattern does not match.
//!
//! In this case, we want the string to be a valid UUIDv4 format, and return an
//! understandable error to the user if it does not match.
//!
//! ```rust
//! # fn main() -> Result<(), Box<std::error::Error>> {
//! use automaat_core::{Context, Processor};
//! use automaat_processor_string_regex::StringRegex;
//!
//! let context = Context::new()?;
//!
//! let processor = StringRegex {
//!     input: "This is not a valid UUID".to_owned(),
//!     regex: r"\A([a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12})\z".to_owned(),
//!     mismatch_error: Some("provided value is not in a valid UUIDv4 format".to_owned()),
//!     replace: None
//! };
//!
//! let error = processor.run(&context).unwrap_err();
//!
//! assert_eq!(error.to_string(), "provided value is not in a valid UUIDv4 format".to_owned());
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
#![doc(html_root_url = "https://docs.rs/automaat-processor-string-regex/0.1.0")]

use automaat_core::{Context, Processor};
use regex::{Error as RegexError, Regex};
use serde::{Deserialize, Serialize};
use std::{error, fmt};

/// The processor configuration.
#[cfg_attr(feature = "juniper", derive(juniper::GraphQLObject))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StringRegex {
    /// The string that will be matched against the provided `regex`, and
    /// optionally replaced by the `replace` pattern.
    pub input: String,

    /// The regular expression used to match the `input`. See the regex crate
    /// [syntax documentation] for more details.
    ///
    /// [syntax documentation]: https://docs.rs/regex/latest/regex/#syntax
    pub regex: String,

    /// If the `regex` pattern does not match the `input` value, an error is
    /// returned. By default, a generic mismatch error is returned.
    ///
    /// You can set this value to have it be returned as the error instead.
    pub mismatch_error: Option<String>,

    /// Optionally use the `regex` pattern and the `input` to construct a
    /// replacement string to return as this processors output.
    ///
    /// You can use variables such as `$1` and `$2` to match against the
    /// patterns in the regex.
    pub replace: Option<String>,
}

/// The GraphQL [Input Object][io] used to initialize the processor via an API.
///
/// [`StringRegex`] implements `From<Input>`, so you can directly initialize the
/// processor using this type.
///
/// _requires the `juniper` package feature to be enabled_
///
/// [io]: https://graphql.github.io/graphql-spec/June2018/#sec-Input-Objects
#[cfg(feature = "juniper")]
#[graphql(name = "StringRegexInput")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, juniper::GraphQLInputObject)]
pub struct Input {
    input: String,
    regex: String,
    mismatch_error: Option<String>,
    replace: Option<String>,
}

#[cfg(feature = "juniper")]
impl From<Input> for StringRegex {
    fn from(input: Input) -> Self {
        Self {
            input: input.input,
            regex: input.regex,
            mismatch_error: input.mismatch_error,
            replace: input.replace,
        }
    }
}

impl<'a> Processor<'a> for StringRegex {
    const NAME: &'static str = "String Regex";

    type Error = Error;
    type Output = String;

    /// Validate that the provided [`regex`] pattern is valid.
    ///
    /// # Errors
    ///
    /// If the regex syntax is invalid, the [`Error::Syntax`] error variant is
    /// returned.
    ///
    /// If the regex pattern is too big (highly unlikely), the
    /// [`Error::CompiledTooBig`] error variant is returned.
    ///
    /// Both variants wrap the original [Regex crate errors].
    ///
    /// [`regex`]: StringRegex::regex
    /// [Regex crate errors]: regex::Error
    fn validate(&self) -> Result<(), Self::Error> {
        Regex::new(self.regex.as_str())
            .map(|_| ())
            .map_err(Into::into)
    }

    /// Do a regex match (and replace), based on the processor configuration.
    ///
    /// # Output
    ///
    /// If [`replace`] is set to `None`, the output of the processor will be
    /// `Ok(None)` if no error occurred.
    ///
    /// If [`replace`] is set to `Some`, then `Some` is returned, matching the
    /// final replaced output value in [`Processor::Output`].
    ///
    /// # Errors
    ///
    /// If the [`regex`] pattern does not match the [`input`] input, the
    /// [`Error::Match`] error variant is returned. If [`mismatch_error`] is
    /// set, the error will contain the provided message. If not, a default
    /// message is provided.
    ///
    /// If the regex pattern is invalid, the same errors are returned as
    /// [`validate`].
    ///
    /// [`replace`]: StringRegex::replace
    /// [`regex`]: StringRegex::regex
    /// [`input`]: StringRegex::input
    /// [`mismatch_error`]: StringRegex::mismatch_error
    /// [`validate`]: #method.validate
    fn run(&self, _context: &Context) -> Result<Option<Self::Output>, Self::Error> {
        let re = Regex::new(self.regex.as_str()).map_err(Into::<Self::Error>::into)?;

        if re.is_match(self.input.as_str()) {
            match &self.replace {
                None => Ok(None),
                Some(replace) => {
                    let out = re
                        .replace_all(self.input.as_str(), replace.as_str())
                        .into_owned();

                    if out.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(out))
                    }
                }
            }
        } else if let Some(msg) = &self.mismatch_error {
            Err(Error::Match(msg.to_owned()))
        } else {
            Err(Error::Match(format!(
                "Match error: \"{}\" does not match pattern: {}",
                self.input, self.regex
            )))
        }
    }
}

/// Represents all the ways that [`StringRegex`] can fail.
///
/// This type is not intended to be exhaustively matched, and new variants may
/// be added in the future without a major version bump.
#[derive(Debug)]
pub enum Error {
    /// A syntax error.
    Syntax(RegexError),

    /// The compiled program exceeded the set size limit. The argument is the
    /// size limit imposed.
    CompiledTooBig(RegexError),

    /// The regex pattern did not match the provided [`StringRegex::input`].
    ///
    /// The contained string value is either a default mismatch error, or a
    /// custom error, based on the [`StringRegex::mismatch_error`] value.
    Match(String),

    #[doc(hidden)]
    __Unknown, // Match against _ instead, more variants may be added in the future.
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Syntax(ref err) | Error::CompiledTooBig(ref err) => {
                write!(f, "Regex error: {}", err)
            }
            Error::Match(ref string) => write!(f, "{}", string),
            Error::__Unknown => unreachable!(),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::Syntax(ref err) | Error::CompiledTooBig(ref err) => Some(err),
            Error::Match(_) => None,
            Error::__Unknown => unreachable!(),
        }
    }
}

impl From<RegexError> for Error {
    fn from(err: RegexError) -> Self {
        match err {
            RegexError::Syntax(_) => Error::Syntax(err),
            RegexError::CompiledTooBig(_) => Error::CompiledTooBig(err),

            // Regex crate has a non-exhaustive error enum, similar to this
            // crates. Should they ever add an error in an upgrade, we will know
            // because compilation failed, and we'll have to add it as well.
            RegexError::__Nonexhaustive => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn processor_stub() -> StringRegex {
        StringRegex {
            input: "hello world".to_owned(),
            regex: r"\Ahello world\z".to_owned(),
            mismatch_error: None,
            replace: None,
        }
    }

    mod run {
        use super::*;

        #[test]
        fn test_match_pattern() {
            let mut processor = processor_stub();
            processor.input = "hello world".to_owned();
            processor.regex = r"hello \w+".to_owned();

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap();

            assert!(output.is_none())
        }

        #[test]
        fn test_mismatch_pattern_default_error() {
            let mut processor = processor_stub();
            processor.input = "hello world".to_owned();
            processor.regex = r"hi \w+".to_owned();

            let context = Context::new().unwrap();
            let error = processor.run(&context).unwrap_err();

            assert_eq!(
                error.to_string(),
                r#"Match error: "hello world" does not match pattern: hi \w+"#.to_owned()
            )
        }

        #[test]
        fn test_mismatch_pattern_custom_error() {
            let mut processor = processor_stub();
            processor.input = "hello world".to_owned();
            processor.regex = r"hi \w+".to_owned();
            processor.mismatch_error = Some("invalid!".to_owned());

            let context = Context::new().unwrap();
            let error = processor.run(&context).unwrap_err();

            assert_eq!(error.to_string(), "invalid!".to_owned())
        }

        #[test]
        fn test_replace_pattern() {
            let mut processor = processor_stub();
            processor.input = "hello world".to_owned();
            processor.regex = r"hello (\w+)".to_owned();
            processor.replace = Some("hi $1!".to_owned());

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert_eq!(output, "hi world!".to_owned())
        }

        #[test]
        fn test_replace_multiline() {
            let mut processor = processor_stub();
            processor.input = "hello world\nhello universe".to_owned();
            processor.regex = r"(?m)^hello (\w+)$".to_owned();
            processor.replace = Some("hi $1!".to_owned());

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert_eq!(output, "hi world!\nhi universe!".to_owned())
        }
    }

    mod validate {
        use super::*;

        #[test]
        fn test_valid_syntax() {
            let mut processor = processor_stub();
            processor.regex = r"hello \w+".to_owned();

            processor.validate().unwrap()
        }

        #[test]
        #[should_panic]
        fn test_invalid_syntax() {
            let mut processor = processor_stub();
            processor.regex = r"hello \NO".to_owned();

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
