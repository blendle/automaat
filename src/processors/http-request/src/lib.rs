//! An [Automaat] processor to execute HTTP requests.
//!
//! This processor allows you to make simple HTTP requests. It supports request
//! headers, setting a request body, and asserting the response status.
//!
//! If more power or functionality is needed, we can add it as needed. However,
//! you can always use the [Shell Command] processor combined with a utility
//! like [`cURL`] if you need more advanced functionality.
//!
//! [Automaat]: automaat_core
//! [Shell Command]: https://docs.rs/automaat-processor-shell-command
//! [`cURL`]: https://curl.haxx.se/
//!
//! # Examples
//!
//! ## GET
//!
//! A GET request with headers attached.
//!
//! ```rust
//! # fn main() -> Result<(), Box<std::error::Error>> {
//! use automaat_core::{Context, Processor};
//! use automaat_processor_http_request::{HttpRequest, Method, Header};
//! use url::Url;
//!
//! let context = Context::new()?;
//! let headers = vec![
//!     Header::new("accept", "application/json"),
//!     Header::new("content-type", "text/html"),
//! ];
//!
//! let processor = HttpRequest {
//!     url: "https://httpbin.org/headers".to_owned(),
//!     method: Method::GET,
//!     headers: headers,
//!     body: None,
//!     assert_status: vec![],
//! };
//!
//! let output = processor.run(&context)?;
//! # assert!(output.clone().unwrap().contains(r#""Content-Type": "text/html""#));
//! # assert!(output.unwrap().contains(r#""Accept": "application/json""#));
//! #     Ok(())
//! # }
//! ```
//!
//! ## POST
//!
//! Simple POST request with a query parameter and a body.
//!
//! ```rust
//! # fn main() -> Result<(), Box<std::error::Error>> {
//! use automaat_core::{Context, Processor};
//! use automaat_processor_http_request::{Method, HttpRequest };
//! use url::Url;
//!
//! let context = Context::new()?;
//!
//! let processor = HttpRequest {
//!     url: "https://httpbin.org/response-headers?hello=world".to_owned(),
//!     method: Method::POST,
//!     headers: vec![],
//!     body: Some("universe".to_owned()),
//!     assert_status: vec![200],
//! };
//!
//! let output = processor.run(&context)?;
//! # assert!(output.clone().unwrap().contains(r#""hello": "world""#));
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
#![doc(html_root_url = "https://docs.rs/automaat-processor-http-request/0.1.0")]

use automaat_core::{Context, Processor};
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::{error, fmt, str::FromStr};
use url::Url;

/// The processor configuration.
#[cfg_attr(feature = "juniper", derive(juniper::GraphQLObject))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HttpRequest {
    /// The URL to make the request to.
    pub url: String,

    /// The HTTP method (GET, POST, etc.) to use.
    pub method: Method,

    /// An optional set of headers to add to the request.
    pub headers: Vec<Header>,

    /// The optional body of the request.
    pub body: Option<String>,

    /// An assertion to validate the status code of the response matches one of
    /// the provided values.
    pub assert_status: Vec<i32>,
}

/// The processor configuration.
#[cfg_attr(feature = "juniper", derive(juniper::GraphQLEnum))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Method {
    /// The CONNECT request method.
    CONNECT,

    /// The DELETE request method.
    DELETE,

    /// The GET request method.
    GET,

    /// The HEAD request method.
    HEAD,

    /// The OPTIONS request method.
    OPTIONS,

    /// The PATCH request method.
    PATCH,

    /// The POST request method.
    POST,

    /// The PUT request method.
    PUT,

    /// The TRACE request method.
    TRACE,
}

impl From<Method> for reqwest::Method {
    fn from(method: Method) -> Self {
        match method {
            Method::CONNECT => Self::CONNECT,
            Method::DELETE => Self::DELETE,
            Method::GET => Self::GET,
            Method::HEAD => Self::HEAD,
            Method::OPTIONS => Self::OPTIONS,
            Method::PATCH => Self::PATCH,
            Method::POST => Self::POST,
            Method::PUT => Self::PUT,
            Method::TRACE => Self::TRACE,
        }
    }
}

/// A request header.
#[cfg_attr(feature = "juniper", derive(juniper::GraphQLObject))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Header {
    /// The name of the header.
    pub name: String,

    /// The value of the header.
    pub value: String,
}

impl Header {
    /// Create a header, based on a name and value string.
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }
}

#[cfg(feature = "juniper")]
impl From<HeaderInput> for Header {
    fn from(input: HeaderInput) -> Self {
        Self {
            name: input.name,
            value: input.value,
        }
    }
}

/// The GraphQL [Input Object][io] used to initialize the processor via an API.
///
/// [`HttpRequest`] implements `From<Input>`, so you can directly initialize
/// the processor using this type.
///
/// _requires the `juniper` package feature to be enabled_
///
/// [io]: https://graphql.github.io/graphql-spec/June2018/#sec-Input-Objects
#[cfg(feature = "juniper")]
#[graphql(name = "HttpRequestInput")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, juniper::GraphQLInputObject)]
pub struct Input {
    url: String,
    method: Method,
    headers: Option<Vec<HeaderInput>>,
    body: Option<String>,
    assert_status: Option<Vec<i32>>,
}

/// A request header.
#[cfg(feature = "juniper")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, juniper::GraphQLInputObject)]
pub struct HeaderInput {
    /// The name of the header.
    pub name: String,

    /// The value of the header.
    pub value: String,
}

#[cfg(feature = "juniper")]
impl From<Input> for HttpRequest {
    fn from(input: Input) -> Self {
        Self {
            url: input.url,
            method: input.method,
            headers: input
                .headers
                .unwrap_or_else(Default::default)
                .into_iter()
                .map(Into::into)
                .collect(),
            body: input.body,
            assert_status: input.assert_status.unwrap_or_else(Default::default),
        }
    }
}

impl HttpRequest {
    /// Convert the string URL into a URL object.
    fn url(&self) -> Result<Url, Error> {
        Url::from_str(&self.url).map_err(Into::into)
    }

    /// Validate the `HttpRequest` configuration.
    ///
    /// # Errors
    ///
    /// This method returns an error if one of the provided HTTP headers has an
    /// invalid format, or if the URL is invalid.
    fn validate(&self) -> Result<(), Error> {
        let _ = self.url()?;

        for header in &self.headers {
            let _ = header::HeaderName::from_str(header.name.as_str())?;
            let _ = header::HeaderValue::from_str(header.value.as_str())?;
        }

        Ok(())
    }
}

impl<'a> Processor<'a> for HttpRequest {
    const NAME: &'static str = "HTTP Request";

    type Error = Error;
    type Output = String;

    /// Do the configured HTTP request, and return its results.
    ///
    /// # Output
    ///
    /// If the request was successful, and the response status matches the
    /// optional status assertion, the body of the response is returned.
    ///
    /// If the body is an empty string, `None` is returned instead.
    ///
    /// # Errors
    ///
    /// If the provided URL is invalid, the [`Error::Url`] error variant is
    /// returned.
    ///
    /// If the provided HTTP headers are invalid, the [`Error::Header`] error
    /// variant is returned.
    ///
    /// If the request fails, or the response body cannot be read, the
    /// [`Error::Response`] error variant is returned.
    ///
    /// If the response status does not match one of the provided status
    /// assertions, the [`Error::Status`] error variant is returned.
    fn run(&self, _context: &Context) -> Result<Option<Self::Output>, Self::Error> {
        self.validate()?;

        // request builder
        let mut request = Client::new().request(self.method.into(), self.url.as_str());

        // headers
        let mut map = header::HeaderMap::new();
        for header in &self.headers {
            let _ = map.insert(
                header.name.as_str().parse::<header::HeaderName>()?,
                header.value.as_str().parse()?,
            );
        }

        // body
        if let Some(body) = self.body.to_owned() {
            request = request.body(body);
        }

        // response
        let mut response = request.headers(map).send()?;

        // status check
        let status = i32::from(response.status().as_u16());
        if !self.assert_status.is_empty() && !self.assert_status.contains(&status) {
            return Err(Error::Status(status));
        }

        // response body
        let body = response.text()?;
        if body.is_empty() {
            Ok(None)
        } else {
            Ok(Some(body))
        }
    }
}

/// Represents all the ways that [`HttpRequest`] can fail.
///
/// This type is not intended to be exhaustively matched, and new variants may
/// be added in the future without a major version bump.
#[derive(Debug)]
pub enum Error {
    /// The response returned an error
    Response(reqwest::Error),

    /// One of the provided request headers has an invalid format.
    Header(String),

    /// The expected response status did not match the actual status.
    Status(i32),

    /// The URL has an invalid format.
    Url(url::ParseError),

    #[doc(hidden)]
    __Unknown, // Match against _ instead, more variants may be added in the future.
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Response(ref err) => write!(f, "Response error: {}", err),
            Error::Url(ref err) => write!(f, "URL error: {}", err),
            Error::Header(ref err) => write!(f, "Invalid header: {}", err),
            Error::Status(status) => write!(f, "Invalid status code: {}", status),
            Error::__Unknown => unreachable!(),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::Response(ref err) => Some(err),
            Error::Url(ref err) => Some(err),
            Error::Header(_) | Error::Status(_) => None,
            Error::__Unknown => unreachable!(),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Response(err)
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::Url(err)
    }
}

impl From<header::InvalidHeaderName> for Error {
    fn from(err: header::InvalidHeaderName) -> Self {
        Error::Header(err.to_string())
    }
}

impl From<header::InvalidHeaderValue> for Error {
    fn from(err: header::InvalidHeaderValue) -> Self {
        Error::Header(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn processor_stub() -> HttpRequest {
        HttpRequest {
            url: "https://httpbin.org/status/200".to_owned(),
            method: Method::GET,
            headers: vec![],
            body: None,
            assert_status: vec![],
        }
    }

    mod run {
        use super::*;

        #[test]
        fn test_empty_response() {
            let processor = processor_stub();

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap();

            assert!(output.is_none())
        }

        #[test]
        fn test_response_body() {
            let mut processor = processor_stub();
            processor.url = "https://httpbin.org/range/5".to_owned();

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap();

            assert_eq!(output, Some("abcde".to_owned()))
        }

        #[test]
        fn test_request_body() {
            let mut processor = processor_stub();
            processor.url = "https://httpbin.org/anything".to_owned();
            processor.body = Some("hello world".to_owned());

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert!(output.contains("hello world"));
        }

        #[test]
        fn test_request_header() {
            let mut processor = processor_stub();
            processor.url = "https://httpbin.org/headers".to_owned();
            processor.headers = vec![Header {
                name: "test-header".to_owned(),
                value: "value".to_owned(),
            }];

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert!(output.contains("Test-Header"));
        }

        #[test]
        fn test_valid_status() {
            let mut processor = processor_stub();
            processor.url = "https://httpbin.org/status/200".to_owned();
            processor.assert_status = vec![200, 204];

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap();

            assert_eq!(output, None)
        }

        #[test]
        fn test_invalid_status() {
            let mut processor = processor_stub();
            processor.url = "https://httpbin.org/status/404".to_owned();
            processor.assert_status = vec![200, 201];

            let context = Context::new().unwrap();
            let error = processor.run(&context).unwrap_err();

            assert_eq!(error.to_string(), "Invalid status code: 404".to_owned());
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
