//! Each [`Pipeline`] contains zero or more variables.
//!
//! A `Variable` is a "runtime" (or "deferred") value that is substituted for
//! any templated [`Processor`] configuration values.
//!
//! The person building the pipeline is required to provide all the
//! configuration values needed to run the steps added to the pipeline, but can
//! choose to use a template value, such as `{country code}` instead of an
//! actual value, and attach a `country code` variable to the pipeline.
//!
//! Now, whenever a pipeline is triggered, the person triggering the pipeline is
//! required to provide the actual value for the `country code` variable.
//!
//! This way, pipeline creators can create pipelines that are as easy to use as
//! possible, while still allowing a pipeline to be used for multiple purposes
//! (in this example, the pipeline could be configured to print the weather
//! forecast for the specified country).
//!
//! An optional description can be provided to give some extra context for the
//! person triggering the pipeline. For example:
//!
//! > A `ISO 3166-1 alpha-2` formatted country code.
//!
//! [`Processor`]: crate::Processor

use crate::resources::Pipeline;
use crate::schema::variables;
use crate::Database;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::convert::AsRef;

/// The model representing a variable stored in the database.
#[derive(Clone, Debug, Deserialize, Serialize, Associations, Identifiable, Queryable)]
#[belongs_to(Pipeline)]
#[table_name = "variables"]
pub(crate) struct Variable {
    pub(crate) id: i32,
    pub(crate) key: String,
    pub(crate) description: Option<String>,
    pub(crate) pipeline_id: i32,
}

/// The actual runtime variable value belonging to a value (matched by key).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct VariableValue {
    pub(crate) key: String,
    pub(crate) value: String,
}

/// Contains all the details needed to store a variable in the database.
///
/// Use [`NewVariable::new`] to initialize this struct.
#[derive(Clone, Debug, Deserialize, Serialize, Insertable)]
#[table_name = "variables"]
pub(crate) struct NewVariable<'a> {
    key: &'a str,
    description: Option<&'a str>,
    pipeline_id: Option<i32>,
}

impl<'a> NewVariable<'a> {
    /// Initialize a `NewVariable` struct, which can be inserted into the
    /// database using the [`NewVariable#add_to_pipeline`] method.
    pub(crate) const fn new(key: &'a str, description: Option<&'a str>) -> Self {
        Self {
            key,
            description,
            pipeline_id: None,
        }
    }

    /// Add a variable to a [`Pipeline`], by storing it in the database as an
    /// association.
    ///
    /// Requires a reference to a Pipeline, in order to create the correct data
    /// reference.
    pub(crate) fn add_to_pipeline(
        mut self,
        conn: &Database,
        pipeline: &Pipeline,
    ) -> QueryResult<()> {
        use crate::schema::variables::dsl::*;
        self.pipeline_id = Some(pipeline.id);

        diesel::insert_into(variables)
            .values(&self)
            .execute(&**conn)
            .map(|_| ())
    }
}

pub(crate) mod graphql {
    //! All GraphQL related functionality is encapsulated in this module. The
    //! relevant functions and structs are re-exported through
    //! [`crate::graphql`].
    //!
    //! API documentation in this module is also used in the GraphQL API itself
    //! as documentation for the clients.
    //!
    //! You can browse to `/graphql/playground` to see all relevant query,
    //! mutation, and type documentation.

    use super::*;
    use crate::resources::Pipeline;
    use juniper::{object, FieldResult, GraphQLInputObject, ID};

    /// Contains all the data needed to create a new `Variable`.
    #[derive(Debug, Clone, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct CreateVariableInput {
        /// The key used to match against templated step configurations.
        ///
        /// If a step's string value contains `{server url}`, then setting the
        /// variable's key to `server url` will allow the step value to be
        /// replaced by the eventually provided variable value when triggering a
        /// pipeline.
        pub(crate) key: String,

        /// An optional description that can be used to explain to a person
        /// about to run a pipeline what the intent is of the required variable.
        pub(crate) description: Option<String>,
    }

    /// Contains all the data needed to replace templated step configs.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct VariableValueInput {
        pub(crate) key: String,
        pub(crate) value: String,
    }

    #[object(Context = Database)]
    impl Variable {
        /// The unique identifier for a specific variable.
        fn id() -> ID {
            ID::new(self.id.to_string())
        }

        /// The key used to match against templated processor configurations.
        fn key() -> &str {
            self.key.as_ref()
        }

        /// An (optional) detailed description of the intent of the variable.
        ///
        /// A description _might_ be markdown formatted, and should be parsed
        /// accordingly by the client.
        fn description() -> Option<&str> {
            self.description.as_ref().map(String::as_ref)
        }

        /// The pipeline to which the variable belongs.
        ///
        /// This field can return `null`, but _only_ if a database error
        /// prevents the data from being retrieved.
        ///
        /// Every variable is _always_ attached to a pipeline, so in normal
        /// circumstances, this field will always return the relevant pipeline
        /// details.
        ///
        /// If a `null` value is returned, it is up to the client to decide the
        /// best course of action. The following actions are advised, sorted by
        /// preference:
        ///
        /// 1. continue execution if the information is not critical to success,
        /// 2. retry the request to try and get the relevant information,
        /// 3. disable parts of the application reliant on the information,
        /// 4. show a global error, and ask the user to retry.
        fn pipeline(context: &Database) -> FieldResult<Option<Pipeline>> {
            use crate::schema::pipelines::dsl::*;

            pipelines
                .filter(id.eq(self.pipeline_id))
                .first(&**context)
                .map(Some)
                .map_err(Into::into)
        }
    }
}

impl<'a> From<&'a graphql::CreateVariableInput> for NewVariable<'a> {
    fn from(input: &'a graphql::CreateVariableInput) -> Self {
        Self::new(&input.key, input.description.as_ref().map(String::as_ref))
    }
}

impl From<graphql::VariableValueInput> for VariableValue {
    fn from(input: graphql::VariableValueInput) -> Self {
        Self {
            key: input.key,
            value: input.value,
        }
    }
}
