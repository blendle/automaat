//! An [Automaat] processor to run a `jq` program against a JSON string.
//!
//! You can use this processor to manipulate JSON strings provided by another
//! processor.
//!
//! This is a very powerful and versatile processor, courtesy of the [`jq`]
//! library. It allows you do filter data, manipulate data, or return boolean
//! values that can be used in the next processor to decide its output.
//!
//! [Automaat]: automaat_core
//! [`jq`]: https://stedolan.github.io/jq/manual/v1.6/
//!
//! # Example
//!
//! Take the value of the `hello` key, and uppercase the ASCII characters.
//!
//! ```rust
//! # fn main() -> Result<(), Box<std::error::Error>> {
//! use automaat_core::{Context, Processor};
//! use automaat_processor_json_edit::JsonEdit;
//!
//! let context = Context::new()?;
//!
//! let processor = JsonEdit {
//!     json: r#"{"hello":"world"}"#.to_owned(),
//!     program: ".hello | ascii_upcase".to_owned(),
//!     pretty_output: false,
//! };
//!
//! let output = processor.run(&context)?;
//!
//! assert_eq!(output, Some("WORLD".to_owned()));
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
#![doc(html_root_url = "https://docs.rs/automaat-processor-json-edit/0.1.0")]

use automaat_core::{Context, Processor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error, fmt};

/// The processor configuration.
#[cfg_attr(feature = "juniper", derive(juniper::GraphQLObject))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JsonEdit {
    /// The JSON string that will be parsed by the `program` value.
    pub json: String,

    /// The program to run against the provided `json` string.
    ///
    /// A program can either filter the JSON down to a subset of data, or can
    /// mutate the data before returning a value.
    ///
    /// To learn about the supported syntax, see the `jq` documentation:
    ///
    /// https://stedolan.github.io/jq/manual/v1.6/
    pub program: String,

    /// "Pretty print" the JSON output.
    ///
    /// If set to false, the JSON will be printed in a compact format, without
    /// any indentation, spacing or newlines.
    pub pretty_output: bool,
}

/// The GraphQL [Input Object][io] used to initialize the processor via an API.
///
/// [`JsonEdit`] implements `From<Input>`, so you can directly initialize the
/// processor using this type.
///
/// _requires the `juniper` package feature to be enabled_
///
/// [io]: https://graphql.github.io/graphql-spec/June2018/#sec-Input-Objects
#[cfg(feature = "juniper")]
#[graphql(name = "JsonEditInput")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, juniper::GraphQLInputObject)]
pub struct Input {
    json: String,
    program: String,
    pretty_output: Option<bool>,
}

#[cfg(feature = "juniper")]
impl From<Input> for JsonEdit {
    fn from(input: Input) -> Self {
        Self {
            json: input.json,
            program: input.program,
            pretty_output: input.pretty_output.unwrap_or(false),
        }
    }
}

impl JsonEdit {
    fn to_string(&self, value: &Value) -> Result<String, serde_json::Error> {
        if value.is_string() {
            return Ok(value.as_str().unwrap().to_owned());
        };

        if self.pretty_output {
            serde_json::to_string_pretty(&value)
        } else {
            serde_json::to_string(&value)
        }
    }
}

impl<'a> Processor<'a> for JsonEdit {
    const NAME: &'static str = "JSON Edit";

    type Error = Error;
    type Output = String;

    /// Run the provided `program` against the `json` data.
    ///
    /// # Output
    ///
    /// If the final output is a string, the surrounding JSON quotes are
    /// removed. This makes it easier to show raw strings in the UI, without
    /// having to use the regex processor to remove extra quotes.
    ///
    /// This output:
    ///
    /// ```json
    /// "world"
    /// ```
    ///
    /// Becomes this:
    ///
    /// ```json
    /// world
    /// ```
    ///
    /// If `pretty_output` is set, any JSON object or array is pretty printed,
    /// by including newlines, indentation and spacing around the key/value
    /// pairs.
    ///
    /// This output:
    ///
    /// ```json
    /// {"hello":"world"}
    /// ```
    ///
    /// Becomes this:
    ///
    /// ```json
    /// {
    ///   "hello": "world"
    /// }
    /// ```
    ///
    /// When unwrapping arrays in the program, each line is processed according
    /// to the above rules.
    ///
    /// So this output:
    ///
    /// ```json
    /// [{"hello":"world"},"hello",2]
    /// ```
    ///
    /// Becomes this:
    ///
    /// ```json
    /// {"hello":"world"}
    /// hello
    /// 2
    /// ```
    ///
    /// When using the program `.[]`.
    ///
    /// # Errors
    ///
    /// This method returns the [`Error::Json`] error variant if the provided
    /// `json` input or the `program` has invalid syntax.
    ///
    /// The [`Error::Serde`] error variant is returned if the processor failed
    /// to serialize or deserialize the input/output JSON.
    fn run(&self, _context: &Context) -> Result<Option<Self::Output>, Self::Error> {
        let mut output = vec![];
        let json = json_query::run(self.program.as_str(), self.json.as_str())?;

        // The jq program can return multiple lines of JSON if an array is
        // unpacked.
        for line in json.lines() {
            let value: Value = serde_json::from_str(line)?;

            if !value.is_null() {
                output.push(self.to_string(&value)?)
            }
        }

        let string = output.join("\n").trim().to_owned();

        if string.is_empty() {
            Ok(None)
        } else {
            Ok(Some(string))
        }
    }
}

/// Represents all the ways that [`JsonEdit`] can fail.
///
/// This type is not intended to be exhaustively matched, and new variants may
/// be added in the future without a major version bump.
#[derive(Debug)]
pub enum Error {
    /// A syntax error.
    Json(String),

    /// An error during serialization or deserialization.
    Serde(serde_json::Error),

    #[doc(hidden)]
    __Unknown, // Match against _ instead, more variants may be added in the future.
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Json(ref err) => write!(f, "JSON error: {}", err),
            Error::Serde(ref err) => write!(f, "Serde error: {}", err),
            Error::__Unknown => unreachable!(),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::Json(_) => None,
            Error::Serde(ref err) => Some(err),
            Error::__Unknown => unreachable!(),
        }
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::Json(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serde(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn processor_stub() -> JsonEdit {
        JsonEdit {
            json: r#"{"hello":"world"}"#.to_owned(),
            program: ".hello".to_owned(),
            pretty_output: false,
        }
    }

    mod run {
        use super::*;

        #[test]
        fn test_mismatch_program() {
            let mut processor = processor_stub();
            processor.json = r#"{"hello":"world"}"#.to_owned();
            processor.program = ".hi".to_owned();

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap();

            assert!(output.is_none())
        }

        #[test]
        fn test_match_program() {
            let mut processor = processor_stub();
            processor.json = r#"{"hello":"world"}"#.to_owned();
            processor.program = ".hello".to_owned();

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert_eq!(output, "world".to_owned())
        }

        #[test]
        fn test_unwrapped_array() {
            let mut processor = processor_stub();
            processor.json = r#"[{"hello":"world"},{"hello":2}]"#.to_owned();
            processor.program = ".[] | .hello".to_owned();

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert_eq!(output, "world\n2".to_owned())
        }

        #[test]
        fn test_empty_output() {
            let mut processor = processor_stub();
            processor.json = r#"[{"hello":"world"},{"hello":2}]"#.to_owned();
            processor.program = ".[0]".to_owned();
            processor.pretty_output = true;

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert_eq!(output, "{\n  \"hello\": \"world\"\n}".to_owned())
        }

        #[test]
        fn test_combination_of_empty_and_non_empty_lines() {
            let mut processor = processor_stub();
            processor.json = r#"["","hello","","world"]"#.to_owned();
            processor.program = ".[]".to_owned();
            processor.pretty_output = true;

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            // the double newline is as expected, since we trim the start and
            // the end of the output, but keep any newlines you need in the
            // middle of the output. We do still remove `null` values in the
            // middle, to allow for different behaviors depending on the need.
            assert_eq!(output, "hello\n\nworld".to_owned())
        }

        #[test]
        fn test_combination_of_null_and_non_null_lines() {
            let mut processor = processor_stub();
            processor.json = r#"[null,"hello",null,"world"]"#.to_owned();
            processor.program = ".[]".to_owned();
            processor.pretty_output = true;

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert_eq!(output, "hello\nworld".to_owned())
        }

        #[test]
        fn test_empty_string_output() {
            let mut processor = processor_stub();
            processor.json = r#"["",""]"#.to_owned();
            processor.program = ".[]".to_owned();
            processor.pretty_output = true;

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap();

            assert!(output.is_none())
        }

        #[test]
        fn test_null_output() {
            let mut processor = processor_stub();
            processor.json = r#"{"hello":"world"}"#.to_owned();
            processor.program = ".hi".to_owned();
            processor.pretty_output = true;

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap();

            assert!(output.is_none())
        }

        #[test]
        fn test_pretty_output_multi_line() {
            let mut processor = processor_stub();
            processor.json = r#"[{"hello":"world"},{"hello":2}]"#.to_owned();
            processor.program = ".[]".to_owned();
            processor.pretty_output = true;

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert_eq!(
                output,
                "{\n  \"hello\": \"world\"\n}\n{\n  \"hello\": 2\n}".to_owned()
            )
        }

        #[test]
        fn test_complex_program_output() {
            let mut processor = processor_stub();
            processor.json = r#"["a", 2, true, null, {"hello":"world"},{"hello":2}]"#.to_owned();
            processor.pretty_output = true;
            processor.program = r#"tostream
                                    | select(length > 1)
                                    | (
                                    .[0] | map(
                                        if type == "number"
                                        then "[" + tostring + "]"
                                        else "." + .
                                        end
                                    ) | join("")
                                    ) + " = " + (.[1] | @json)"#
                .to_owned();

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");
            let expected = "[0] = \"a\"\n\
                            [1] = 2\n\
                            [2] = true\n\
                            [3] = null\n\
                            [4].hello = \"world\"\n\
                            [5].hello = 2"
                .to_owned();

            assert_eq!(output, expected)
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
