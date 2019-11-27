//! An [Automaat] processor to run SQL queries.
//!
//! This processor allows you to run SQL queries and return the output of the
//! query as a JSON object.
//!
//! The returned JSON object contains the returned column names as keys, and the
//! row values as the JSON value.
//!
//! Right now, the processor only supports Postgres connections, and only
//! supports SELECT statements. Not all SQL types are supported, but the most
//! common ones are, and new ones can be added if there is a need for them.
//!
//! Combining this processor with the "[JSON Editor]" processor allows you to
//! transform the returned data before presenting it to the user.
//!
//! [Automaat]: automaat_core
//! [JSON Editor]: https://docs.rs/automaat-processor-json-editor
//!
//! # Example
//!
//! Query the database for multiple records with multiple columns of different
//! types.
//!
//! ```rust
//! # fn main() -> Result<(), Box<std::error::Error>> {
//! #     use postgres::{Client, NoTls};
//! #
//! #     struct Guard(Client);
//! #
//! #     impl Drop for Guard {
//! #        fn drop(&mut self) {
//! #            self.0.execute("DROP TABLE users", &[]).unwrap();
//! #        }
//! #     }
//! #
//! #     let mut client = Guard(Client::connect("postgres://postgres@127.0.0.1", NoTls).unwrap());
//! #     let q1 = "DROP TABLE IF EXISTS users;";
//! #     let q2 = "CREATE UNLOGGED TABLE users (id INT, name VARCHAR);";
//! #     let q3 = "INSERT INTO users (id, name) VALUES (1, 'Bart'), (2, 'Lisa'), (3, 'Homer')";
//! #     let _tnx = client.0.simple_query(format!("{}{}{}", q1, q2, q3).as_str()).unwrap();
//! #
//! use automaat_core::{Context, Processor};
//! use automaat_processor_sql_query::{SqlQuery, Type};
//! use url::Url;
//! use serde_json::{from_str, json, Value};
//!
//! let context = Context::new()?;
//!
//! let processor = SqlQuery {
//!     statement: "SELECT id, name FROM users WHERE name = $1 OR id = $2".to_owned(),
//!     parameters: vec![Type::text("Bart"), Type::int(2)],
//!     url: "postgres://postgres@127.0.0.1".to_owned(),
//! };
//!
//! let output = processor.run(&context)?.expect("Some");
//! let expect = json!([{ "id": 1, "name": "Bart" }, { "id": 2, "name": "Lisa" }]);
//!
//! assert_eq!(from_str::<Value>(&output)?, expect);
//! #
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
#![doc(html_root_url = "https://docs.rs/automaat-processor-sql-query/0.1.0")]

pub mod types;
pub use types::Type;

use automaat_core::{Context, Processor};
use postgres::types::ToSql;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlparser::ast::Statement;
use sqlparser::dialect::{Dialect, GenericDialect};
use sqlparser::parser::{Parser, ParserError};
use std::collections::HashMap;
use std::{error, fmt, str::FromStr};
use url::Url;

/// The processor configuration.
#[cfg_attr(feature = "juniper", derive(juniper::GraphQLObject))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SqlQuery {
    /// The SQL statement to execute. This MUST start with "SELECT ...".
    pub statement: String,

    /// The URL of the database server.
    ///
    /// Use URLs such as `postgres://postgres:mypassword@127.0.0.1/my_database`
    pub url: String,

    /// An optional set of parameters to safely inject into the query.
    ///
    /// Parameters are specified in the statement by $n, where n is the index of
    /// the parameter of the list provided, 1-indexed.
    ///
    /// So a statement `SELECT name FROM table WHERE id = $1 AND address = $2`
    /// can be executed with the parameters:
    ///
    /// `[Type::int(2), Type::text("home")].`
    ///
    /// Because of the inability to return union scalar values in GraphQL, the
    /// response type of parameters is a bit awkward. It returns a `Type` object
    /// with all possible parameter types as their own optional fields.
    ///
    /// The server will guarantee that **exactly one** of those fields will have
    /// it value set, representing the type and value of the parameter.
    ///
    /// Currently, only `text`, `int` and `bool` parameter types are supported.
    pub parameters: Vec<Type>,
}

/// The GraphQL [Input Object][io] used to initialize the processor via an API.
///
/// [`SqlQuery`] implements `From<Input>`, so you can directly initialize the
/// processor using this type.
///
/// _requires the `juniper` package feature to be enabled_
///
/// [io]: https://graphql.github.io/graphql-spec/June2018/#sec-Input-Objects
#[cfg(feature = "juniper")]
#[graphql(name = "SqlQueryInput")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, juniper::GraphQLInputObject)]
pub struct Input {
    statement: String,

    url: String,

    parameters: Option<Vec<types::TypeInput>>,
}

#[cfg(feature = "juniper")]
impl From<Input> for SqlQuery {
    fn from(input: Input) -> Self {
        let parameters = input
            .parameters
            .unwrap_or_default()
            .into_iter()
            .map(std::convert::TryInto::try_into)
            .collect::<Result<Vec<_>, ()>>()
            .unwrap_or_default();

        Self {
            statement: input.statement,
            parameters,
            url: input.url,
        }
    }
}

impl SqlQuery {
    /// Convert the string URL into a URL object.
    fn url(&self) -> Result<Url, Error> {
        Url::from_str(&self.url).map_err(Into::into)
    }

    /// Validate that the provided `statement` is valid, and the `url` has a
    /// supported scheme.
    ///
    /// # Errors
    ///
    /// If the URL contains invalid syntax, the [`Error::Url`] error variant is
    /// returned.
    ///
    /// If the statement contains invalid syntax, the [`Error::Syntax`] error
    /// variant is returned.
    ///
    /// If multiple statements are detected, the [`Error::MultipleStatements`]
    /// error variant is returned.
    ///
    /// If the statement does not start with "SELECT", the
    /// [`Error::StatementType`] error variant is returned.
    ///
    /// If the `url` scheme does not match `postgres`, the [`Error::Scheme`]
    /// error variant is returned.
    fn validate(&self) -> Result<(), Error> {
        // Validate URL syntax.
        let url = self.url()?;

        // Set the SQL syntax dialect.
        let dialect: &dyn Dialect = match url.scheme() {
            "postgres" => &PostgreSqlDialect {},
            _ => &GenericDialect {},
        };

        // Validate SQL syntax.
        let ast = Parser::parse_sql(dialect, self.statement.to_owned()).map_err(Error::from)?;

        // Only one statement per query is supported.
        if ast.len() != 1 {
            return Err(Error::MultipleStatements);
        };

        // Only the SELECT statement is supported.
        match ast[0] {
            Statement::Query(_) => (),
            _ => return Err(Error::StatementType),
        };

        match url.scheme() {
            "postgres" => Ok(()),
            scheme => Err(Error::Scheme(scheme.to_owned())),
        }
    }

    fn run_postgres_statement(&self, parameters: &[&dyn ToSql]) -> Result<Option<String>, Error> {
        use postgres::{types::Type as T, Client, NoTls};
        use serde_json::{to_string, to_value};

        let mut conn = Client::connect(self.url.as_str(), NoTls).map_err(Error::from)?;
        let rows = conn
            .query(self.statement.as_str(), parameters)
            .map_err(Error::from)?;

        let mut results = vec![];
        for row in rows {
            let mut map = HashMap::new();
            for column in row.columns() {
                let n = column.name();

                // Hey o/
                //
                // If you wonder why your favorite Postgres value type is not
                // supported, then wonder no more, because there is no reason to
                // this madness.
                //
                // Feel free to open a Pull Request to add the type you require.
                // We'd be glad to accept your contribution!
                //
                // To get some guidance, see the following URL:
                //
                // https://docs.rs/postgres/0.16.0-rc.2/postgres/types/trait.FromSql.html
                #[allow(indirect_structural_match)] // see: http://git.io/JeXc3
                let value: Value = match column.type_() {
                    &T::BOOL => to_value::<Option<bool>>(row.get(n))?,
                    &T::INT4 => to_value::<Option<i32>>(row.get(n))?,
                    &T::JSON | &T::JSONB => to_value::<Option<Value>>(row.get(n))?,
                    &T::TEXT | &T::VARCHAR => to_value::<Option<String>>(row.get(n))?,
                    ty => return Err(Error::ReturnType(ty.to_string())),
                };

                let _ = map.insert(n.to_string(), value);
            }

            if !map.is_empty() {
                results.push(map);
            }
        }

        if results.is_empty() {
            return Ok(None);
        };

        Ok(Some(to_string(&results)?))
    }

    fn run_sqlite_statement(&self) -> Result<Option<String>, Error> {
        unimplemented!()
    }
    fn run_mysql_statement(&self) -> Result<Option<String>, Error> {
        unimplemented!()
    }
}

impl<'a> Processor<'a> for SqlQuery {
    const NAME: &'static str = "SQL Query";

    type Error = Error;
    type Output = String;

    /// Run the provided `statement` against a database and return the values as
    /// an array of JSON objects.
    ///
    /// # Output
    ///
    /// If no records are found, `None` is returned.
    ///
    /// If one or more records are found, a JSON array of objects is returned.
    /// Each object contains the column names as its keys, and the record values
    /// as its values.
    ///
    /// # Errors
    ///
    /// If a database error occurs, the relevant database error type (such as
    /// [`Error::Postgres`]) is returned.
    ///
    /// If an unsupported data type is returned, the [`Error::ReturnType`] error
    /// is returned.
    ///
    /// If anything happens during serialization, the [`Error::Serde`] error is
    /// returned.
    fn run(&self, _context: &Context) -> Result<Option<Self::Output>, Self::Error> {
        self.validate()?;

        let mut parameters: Vec<&dyn ToSql> = vec![];
        for ty in &self.parameters {
            if let Some(v) = ty.as_text() {
                parameters.push(v);
                continue;
            }

            if let Some(v) = ty.as_int() {
                parameters.push(v);
                continue;
            }

            if let Some(v) = ty.as_bool() {
                parameters.push(v);
                continue;
            }

            return Err(Error::ParameterType);
        }

        match self.url()?.scheme() {
            "postgres" => self.run_postgres_statement(&parameters),
            "sqlite" => self.run_sqlite_statement(),
            "mysql" => self.run_mysql_statement(),
            _ => unimplemented!(),
        }
    }
}

/// Represents all the ways that [`SqlQuery`] can fail.
///
/// This type is not intended to be exhaustively matched, and new variants may
/// be added in the future without a major version bump.
#[derive(Debug)]
pub enum Error {
    /// Multiple SQL statements were found (separated by `;`). This is
    /// unsupported.
    MultipleStatements,

    /// A provided parameter SQL type is invalid.
    ParameterType,

    /// Postgres returned an error.
    Postgres(postgres::Error),

    /// The return type of the query is not supported.
    ///
    /// If this error happens, it is usually not because we _can't_ support the
    /// return type, but because we haven't built the needed support yet. Feel
    /// free to create an issue so that we can add the type support you need.
    ReturnType(String),

    /// The used URI scheme is unsupported.
    ///
    /// The scheme of the URI is used to dictate which database types are
    /// supported, so even if the scheme looks okay (such as `sqlite://`), it
    /// could still trigger this error.
    Scheme(String),

    /// An error occurred during serialization or deserialization of the data.
    Serde(serde_json::Error),

    /// The statement type is unsupported. Only SELECT statements are supported,
    /// all others return this error.
    StatementType,

    /// The `statement` field contains invalid SQL syntax.
    Syntax(String),

    /// The URL has an invalid format.
    Url(url::ParseError),

    #[doc(hidden)]
    __Unknown, // Match against _ instead, more variants may be added in the future.
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::MultipleStatements => write!(f, "Multiple SQL statements found"),
            Error::ParameterType => write!(f, "Invalid parameter type provided"),
            Error::Postgres(ref err) => write!(f, "Postgres error: {}", err),
            Error::ReturnType(ref string) => write!(f, "Unsupported return type: {}", string),
            Error::Scheme(ref string) => write!(f, "Unsupported URL scheme: {}", string),
            Error::Serde(ref err) => write!(f, "Serde error: {}", err),
            Error::StatementType => write!(f, "Non-SELECT statements are not supported"),
            Error::Syntax(ref string) => write!(f, "Syntax error: {}", string),
            Error::Url(ref err) => write!(f, "URL error: {}", err),
            Error::__Unknown => unimplemented!(),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::MultipleStatements
            | Error::ParameterType
            | Error::ReturnType(_)
            | Error::Scheme(_)
            | Error::StatementType
            | Error::Syntax(_) => None,
            Error::Postgres(ref err) => Some(err),
            Error::Serde(ref err) => Some(err),
            Error::Url(ref err) => Some(err),
            Error::__Unknown => unreachable!(),
        }
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::Url(err)
    }
}

impl From<ParserError> for Error {
    fn from(err: ParserError) -> Self {
        match err {
            ParserError::ParserError(string) | ParserError::TokenizerError(string) => {
                Error::Syntax(string)
            }
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serde(err)
    }
}

impl From<postgres::Error> for Error {
    fn from(err: postgres::Error) -> Self {
        Error::Postgres(err)
    }
}

/// This is a copy/paste of this code: <http://git.io/fjDhS>
///
/// It adds a `ch == '$'` check, to allow numbered arguments.
#[derive(Copy, Clone, Debug)]
pub struct PostgreSqlDialect {}

impl Dialect for PostgreSqlDialect {
    fn is_identifier_start(&self, ch: char) -> bool {
        (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || ch == '_' || ch == '$'
    }

    fn is_identifier_part(&self, ch: char) -> bool {
        (ch >= 'a' && ch <= 'z')
            || (ch >= 'A' && ch <= 'Z')
            || (ch >= '0' && ch <= '9')
            || ch == '$'
            || ch == '_'
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use postgres::{Client, NoTls};
    use rand::Rng;
    use serde_json::json;

    struct PgData {
        client: Client,
        table: String,
    }

    impl Drop for PgData {
        fn drop(&mut self) {
            let query = format!("DROP TABLE {}", self.table);
            let _ = self.client.execute(query.as_str(), &[]).unwrap();
        }
    }

    fn processor_stub() -> SqlQuery {
        SqlQuery {
            statement: "SELECT * FROM table".to_owned(),
            url: "postgres://postgres@127.0.0.1".to_owned(),
            parameters: vec![],
        }
    }

    fn prepare_pg_data(columns: &str, insert: &str) -> PgData {
        let table = format!("foo_{}", rand::thread_rng().gen::<u16>());
        let mut client = Client::connect("postgres://postgres@127.0.0.1", NoTls).unwrap();
        let query = format!(
            "DROP TABLE IF EXISTS {}; CREATE UNLOGGED TABLE {} {}; INSERT INTO {} {};",
            table, table, columns, table, insert
        );
        let _ = client.simple_query(query.as_str()).unwrap();

        PgData { client, table }
    }

    mod run {
        use super::*;

        #[test]
        fn test_empty_output() {
            let mut processor = processor_stub();
            processor.statement = "SELECT null WHERE true = false".to_owned();

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap();

            assert!(output.is_none())
        }

        #[test]
        fn test_single_value_output() {
            let pg = prepare_pg_data("(id INT)", "(id) VALUES (1)");

            let mut processor = processor_stub();
            processor.statement = format!("SELECT * FROM {}", pg.table);

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert_eq!(json!([{ "id": 1 }]).to_string(), output);
        }

        #[test]
        fn test_multi_value_output() {
            let pg = prepare_pg_data("(id INT)", "(id) VALUES (1), (2), (3)");

            let mut processor = processor_stub();
            processor.statement = format!("SELECT * FROM {}", pg.table);

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert_eq!(
                json!([{ "id": 1 }, { "id": 2 }, { "id": 3 }]).to_string(),
                output
            );
        }

        #[test]
        fn test_null_value_output() {
            let pg = prepare_pg_data("(col INT)", "(col) VALUES (NULL)");

            let mut processor = processor_stub();
            processor.statement = format!("SELECT * FROM {}", pg.table);

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert_eq!(json!([{ "col": null }]).to_string(), output);
        }

        #[test]
        fn test_bool_value_output() {
            let pg = prepare_pg_data("(col BOOL)", "(col) VALUES (true)");

            let mut processor = processor_stub();
            processor.statement = format!("SELECT * FROM {}", pg.table);

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert_eq!(json!([{ "col": true }]).to_string(), output);
        }

        #[test]
        fn test_string_value_output() {
            let pg = prepare_pg_data("(col TEXT)", "(col) VALUES ('hello')");

            let mut processor = processor_stub();
            processor.statement = format!("SELECT * FROM {}", pg.table);

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert_eq!(json!([{ "col": "hello" }]).to_string(), output);
        }

        #[test]
        fn test_json_value_output() {
            let pg = prepare_pg_data("(col JSON)", "(col) VALUES ('[1,2,{\"1\":true}]')");

            let mut processor = processor_stub();
            processor.statement = format!("SELECT * FROM {}", pg.table);

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert_eq!(
                json!([{ "col": [1, 2, { "1": true }] }]).to_string(),
                output
            );
        }

        #[test]
        fn test_parameters() {
            let pg = prepare_pg_data("(col TEXT)", "(col) VALUES ('hello'), ('world')");

            let mut processor = processor_stub();
            processor.statement = format!("SELECT * FROM {} WHERE col = $1", pg.table);
            processor.parameters = vec![Type::text("world")];

            let context = Context::new().unwrap();
            let output = processor.run(&context).unwrap().expect("Some");

            assert_eq!(json!([{ "col": "world" }]).to_string(), output);
        }

        #[test]
        #[should_panic]
        fn test_invalid_parameter() {
            let pg = prepare_pg_data("(col TEXT)", "(col) VALUES ('hello'), ('world')");

            let mut processor = processor_stub();
            processor.statement = format!("SELECT * FROM {} WHERE col = $1", pg.table);
            processor.parameters = vec![Type::default()];

            let context = Context::new().unwrap();
            let _ = processor.run(&context).unwrap().expect("Some");
        }

        #[test]
        fn test_invalid_table() {
            let mut processor = processor_stub();
            processor.statement = "SELECT * FROM does_not_exist".to_owned();

            let context = Context::new().unwrap();
            let error = processor.run(&context).unwrap_err();

            assert!(error
                .to_string()
                .contains("relation \"does_not_exist\" does not exist"));
        }
    }

    mod validate {
        use super::*;

        #[test]
        fn test_select_statement() {
            let mut processor = processor_stub();
            processor.statement = "SELECT * FROM table".to_owned();

            processor.validate().unwrap()
        }

        #[test]
        #[should_panic]
        fn test_update_statement() {
            let mut processor = processor_stub();
            processor.statement = "UPDATE table SET field1 = 1 WHERE field1 = 0".to_owned();

            processor.validate().unwrap()
        }

        #[test]
        #[should_panic]
        fn test_delete_statement() {
            let mut processor = processor_stub();
            processor.statement = "DELETE FROM table WHERE field1 = 0".to_owned();

            processor.validate().unwrap()
        }

        #[test]
        #[should_panic]
        fn test_invalid_statement() {
            let mut processor = processor_stub();
            processor.statement = "HELLO WORLD".to_owned();

            processor.validate().unwrap()
        }

        #[test]
        fn test_postgres_scheme() {
            let mut processor = processor_stub();
            processor.url = "postgres://127.0.0.1".to_owned();

            processor.validate().unwrap()
        }

        #[test]
        #[should_panic]
        fn test_sqlite_scheme() {
            let mut processor = processor_stub();
            processor.url = "sqlite://127.0.0.1".to_owned();

            processor.validate().unwrap()
        }

        #[test]
        #[should_panic]
        fn test_mysql_scheme() {
            let mut processor = processor_stub();
            processor.url = "mysql://127.0.0.1".to_owned();

            processor.validate().unwrap()
        }

        #[test]
        #[should_panic]
        fn test_invalid_scheme() {
            let mut processor = processor_stub();
            processor.url = "invalid://127.0.0.1".to_owned();

            processor.validate().unwrap()
        }

        #[test]
        #[should_panic]
        fn test_invalid_url() {
            let mut processor = processor_stub();
            processor.url = "invalid".to_owned();

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
