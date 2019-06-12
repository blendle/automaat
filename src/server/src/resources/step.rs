//! A [`Step`] is the grouping of a [`Processor`], some identification details
//! such as a name and description, and the position within a series of steps.
//!
//! A step is one "action" in a series of [`Pipeline`] steps.
//!
//! [`Processor`]: crate::Processor

use crate::resources::Pipeline;
use crate::schema::steps;
use crate::{Database, Processor};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::convert::{AsRef, TryFrom, TryInto};
use std::error::Error;

/// The model representing a step stored in the database.
#[derive(Clone, Debug, Deserialize, Serialize, Associations, Identifiable, Queryable)]
#[belongs_to(Pipeline)]
#[table_name = "steps"]
pub(crate) struct Step {
    pub(crate) id: i32,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) processor: serde_json::Value,
    pub(crate) position: i32,
    pub(crate) pipeline_id: i32,
}

impl Step {
    pub(crate) fn processor(&self) -> Result<Processor, serde_json::Error> {
        serde_json::from_value(self.processor.clone())
    }

    pub(crate) fn pipeline(&self, conn: &Database) -> QueryResult<Pipeline> {
        use crate::schema::pipelines::dsl::*;

        pipelines.filter(id.eq(self.pipeline_id)).first(&**conn)
    }
}

/// Contains all the details needed to store a step in the database.
///
/// Use [`NewStep::new`] to initialize this struct.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct NewStep<'a> {
    name: &'a str,
    description: Option<&'a str>,
    processor: Processor,
    position: i32,
}

impl<'a> NewStep<'a> {
    /// Initialize a `NewStep` struct, which can be inserted into the
    /// database using the [`NewStep#add_to_pipeline`] method.
    pub(crate) const fn new(
        name: &'a str,
        description: Option<&'a str>,
        processor: Processor,
        position: i32,
    ) -> Self {
        Self {
            name,
            description,
            processor,
            position,
        }
    }

    /// Add a step to a [`Pipeline`], by storing it in the database as an
    /// association.
    ///
    /// Requires a reference to a Pipeline, in order to create the correct data
    /// reference.
    ///
    /// This method can return an error if the database insert failed, or if the
    /// step processor cannot be serialized.
    pub(crate) fn add_to_pipeline(
        self,
        conn: &Database,
        pipeline: &Pipeline,
    ) -> Result<(), Box<dyn Error>> {
        use crate::schema::steps::dsl::*;

        let values = (
            name.eq(&self.name),
            description.eq(&self.description),
            processor.eq(serde_json::to_value(self.processor)?),
            position.eq(self.position),
            pipeline_id.eq(pipeline.id),
        );

        diesel::insert_into(steps)
            .values(values)
            .execute(&**conn)
            .map(|_| ())
            .map_err(Into::into)
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
    use crate::ProcessorInput;
    use juniper::{object, FieldResult, GraphQLInputObject, ID};

    /// Contains all the data needed to create a new `Step`.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct CreateStepInput {
        /// The name of the step.
        ///
        /// Take care to give a short, but descriptive name, to provide maximum
        /// flexibility for the UI, _and_ provide maximum understanding for the
        /// user.
        pub(crate) name: String,

        /// An optional description of the step.
        ///
        /// While the description is optional, it is best-practice to provide
        /// relevant information so that the user knows what to expect when
        /// adding a step to a pipeline.
        pub(crate) description: Option<String>,

        /// The processor used by this step to perform the required action.
        ///
        /// **note**
        ///
        /// Due to the fact that the GraphQL spec does not (yet) support
        /// union-based input types, this is a wrapper type with a separate
        /// field for each processor type supported by the server.
        ///
        /// All fields are nullable, but you MUST provide EXACTLY ONE processor
        /// input type. Providing anything else will result in an error from the
        /// API.
        pub(crate) processor: ProcessorInput,
    }

    #[object(Context = Database)]
    impl Step {
        /// The unique identifier for a specific step.
        fn id() -> ID {
            ID::new(self.id.to_string())
        }

        /// A descriptive name of the step.
        fn name() -> &str {
            &self.name
        }

        /// An (optional) detailed description of the functionality provided by
        /// this step.
        ///
        /// A description _might_ be markdown formatted, and should be parsed
        /// accordingly by the client.
        fn description() -> Option<&str> {
            self.description.as_ref().map(String::as_ref)
        }

        /// The processor type used to run the step.
        ///
        /// This query can fail, if the processor failed to be deserialized.
        fn processor() -> FieldResult<Processor> {
            self.processor().map_err(Into::into)
        }

        /// The position of the step in a pipeline, compared to other steps in
        /// the same pipeline. A lower number means the step runs earlier in the
        /// pipeline.
        fn position() -> i32 {
            self.position
        }

        /// The pipeline to which the step belongs.
        ///
        /// This field can return `null`, but _only_ if a database error
        /// prevents the data from being retrieved.
        ///
        /// Every step is _always_ attached to a pipeline, so in normal
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
            self.pipeline(context).map(Some).map_err(Into::into)
        }
    }
}

#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
impl<'a> TryFrom<(usize, &'a graphql::CreateStepInput)> for NewStep<'a> {
    type Error = String;

    fn try_from((index, input): (usize, &'a graphql::CreateStepInput)) -> Result<Self, String> {
        Ok(Self::new(
            &input.name,
            input.description.as_ref().map(String::as_ref),
            input.processor.clone().try_into()?,
            index as i32,
        ))
    }
}
