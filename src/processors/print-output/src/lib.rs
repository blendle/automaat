//! An [Automaat] processor to print a string output.
//!
//! Using this crate in your Automaat workflow allows you to return the string
//! provided to the processor's configuration.
//!
//! On its own, this is not very useful, but combined with an application like
//! [Automaat Server], you can allow pipelines to configure this processor on
//! runtime, and relay the output to the end-user.
//!
//! [Automaat]: automaat_core
//! [Automaat Server]: https://docs.rs/automaat-server
//!
//! # Examples
//!
//! Configure the processor with a string, and capture that same value as the
//! output of the processor.
//!
//! This processor is infallible (see [`Void`]), so unwrapping the returned
//! value **will never panic**.
//!
//! ```rust
//! # fn main() -> Result<(), Box<std::error::Error>> {
//! use automaat_core::{Context, Processor};
//! use automaat_processor_print_output::PrintOutput;
//!
//! let context = Context::new()?;
//! let hello = "hello world".to_owned();
//!
//! let processor = PrintOutput {
//!   output: hello.clone(),
//! };
//!
//! let output = processor.run(&context).unwrap();
//!
//! assert_eq!(output, Some(hello));
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
#![doc(html_root_url = "https://docs.rs/automaat-processor-print-output/0.1.0")]

use automaat_core::{Context, Processor};
use serde::{Deserialize, Serialize};
use std::{error, fmt};

/// The processor configuration.
#[cfg_attr(feature = "juniper", derive(juniper::GraphQLObject))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PrintOutput {
    /// The string that is returned by the processor when [`PrintOutput#run`] is
    /// called.
    pub output: String,
}

/// The GraphQL [Input Object][io] used to initialize the processor via an API.
///
/// [`PrintOutput`] implements `From<Input>`, so you can directly initialize the
/// processor using this type.
///
/// _requires the `juniper` package feature to be enabled_
///
/// [io]: https://graphql.github.io/graphql-spec/June2018/#sec-Input-Objects
#[cfg(feature = "juniper")]
#[cfg_attr(feature = "juniper", derive(juniper::GraphQLInputObject))]
#[graphql(name = "PrintOutputInput")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Input {
    output: String,
}

#[cfg(feature = "juniper")]
impl From<Input> for PrintOutput {
    fn from(input: Input) -> Self {
        Self {
            output: input.output,
        }
    }
}

impl<'a> Processor<'a> for PrintOutput {
    const NAME: &'static str = "Print Output";

    type Error = Void;
    type Output = String;

    /// Print the output as defined by the processor configuration.
    ///
    /// The repository will be cloned in the [`Context`]
    /// workspace, optionally in a child `path`.
    ///
    /// # Output
    ///
    /// If the input value is an empty string (`""`), this processor returns
    /// `None`. In all other cases, `Some` is returned, containing the
    /// [`PrintOutput::output`] value.
    ///
    /// # Errors
    ///
    /// This processor is infallible, it will never return the error variant of
    /// the result.
    ///
    /// **Calling [`Result::unwrap`] on the returned value will never panic**.
    ///
    /// [`Context`]: automaat_core::Context
    fn run(&self, _context: &Context) -> Result<Option<Self::Output>, Self::Error> {
        let output = match self.output.as_ref() {
            "" => None,
            string => Some(string.to_owned()),
        };

        Ok(output)
    }
}

/// This is an enum without a variant, and can therefor never exist as a value
/// on runtime. This is also known as an _uninhabited type_, it statically
/// proofs that [`Processor::run`] and [`Processor::validate`] are infallible
/// for [`PrintOutput`].
///
/// Read more about this pattern [in this blog post][b].
///
/// [b]: https://smallcultfollowing.com/babysteps/blog/2018/08/13/never-patterns-exhaustive-matching-and-uninhabited-types-oh-my/
#[derive(Clone, Copy, Debug)]
#[allow(clippy::empty_enum)]
pub enum Void {}

impl fmt::Display for Void {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl error::Error for Void {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod run {
        use super::*;

        #[test]
        fn empty_output() {
            let processor = PrintOutput {
                output: "".to_owned(),
            };

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap();

            assert!(output.is_none())
        }

        #[test]
        fn string_output() {
            let processor = PrintOutput {
                output: "hello".to_owned(),
            };

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap();

            assert_eq!(output, Some("hello".to_owned()))
        }
    }

    mod validate {
        use super::*;

        #[test]
        fn empty_output() {
            let processor = PrintOutput {
                output: "".to_owned(),
            };

            processor.validate().unwrap()
        }

        #[test]
        fn string_output() {
            let processor = PrintOutput {
                output: "hello".to_owned(),
            };

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
