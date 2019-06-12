//! A [`Pipeline`] is a collection of [`Step`]s and [`Variable`]s, wrapped in
//! one package, containing a descriptive name and optional documentation.
//!
//! Each pipeline is pre-configured for usage, after which it can be
//! "triggered".
//!
//! Before a pipeline can be triggered, the person wanting to trigger the
//! pipeline needs to first provide all values for the variables attached to the
//! pipeline. For more details on variables, see the [`variable`] module
//! documentation.
//!
//! Once all variable values are provided, the pipeline can be triggered.
//! Triggering a pipeline results in a [`Task`] being created, which will be
//! picked up by the task runner immediately.
//!
//! [`variable`]: crate::resources::variable

use crate::resources::{NewStep, NewVariable, Step, Variable, VariableValue};
use crate::schema::pipelines;
use crate::Database;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::error;

#[derive(Clone, Debug, Deserialize, Serialize, Identifiable, Queryable)]
#[table_name = "pipelines"]
/// The model representing a pipeline stored in the database.
pub(crate) struct Pipeline {
    pub(crate) id: i32,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
}

impl Pipeline {
    pub(crate) fn steps(&self, conn: &Database) -> QueryResult<Vec<Step>> {
        use crate::schema::steps::dsl::*;

        Step::belonging_to(self)
            .order((position.asc(), id.asc()))
            .load(&**conn)
    }

    pub(crate) fn variables(&self, conn: &Database) -> QueryResult<Vec<Variable>> {
        use crate::schema::variables::dsl::*;

        Variable::belonging_to(self).order(id.desc()).load(&**conn)
    }

    pub(crate) fn get_missing_variable(
        &self,
        conn: &Database,
        variable_values: &[VariableValue],
    ) -> QueryResult<Option<Variable>> {
        let result = self.variables(conn)?.into_iter().find_map(|v| {
            if variable_values.iter().any(|vv| vv.key == v.key) {
                return None;
            }

            Some(v)
        });

        Ok(result)
    }
}

/// Contains all the details needed to store a pipeline in the database.
///
/// Use [`NewPipeline::new`] to initialize this struct.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct NewPipeline<'a> {
    name: &'a str,
    description: Option<&'a str>,
    variables: Vec<NewVariable<'a>>,
    steps: Vec<NewStep<'a>>,
}

impl<'a> NewPipeline<'a> {
    /// Initialize a `NewPipeline` struct, which can be inserted into the
    /// database using the [`NewPipeline#create`] method.
    pub(crate) fn new(name: &'a str, description: Option<&'a str>) -> Self {
        Self {
            name,
            description,
            variables: vec![],
            steps: vec![],
        }
    }

    /// Attach variables to this pipeline.
    ///
    /// `NewPipeline` takes ownership of the variables, but you are required to
    /// call [`NewPipeline#create`] to persist the pipeline and its variables.
    ///
    /// Can be called multiple times to append more variables.
    pub(crate) fn with_variables(&mut self, mut variables: Vec<NewVariable<'a>>) {
        self.variables.append(&mut variables)
    }

    /// Attach steps to this pipeline.
    ///
    /// `NewPipeline` takes ownership of the steps, but you are required to
    /// call [`NewPipeline#create`] to persist the pipeline and its steps.
    ///
    /// Can be called multiple times to append more steps.
    pub(crate) fn with_steps(&mut self, mut steps: Vec<NewStep<'a>>) {
        self.steps.append(&mut steps)
    }

    /// Persist the pipeline and any attached variables and steps into the
    /// database.
    ///
    /// Persisting the data happens within a transaction that is rolled back if
    /// any data fails to persist.
    pub(crate) fn create(self, conn: &Database) -> Result<Pipeline, Box<dyn error::Error>> {
        conn.transaction(|| {
            use crate::schema::pipelines::dsl::*;

            // waiting on https://github.com/diesel-rs/diesel/issues/860
            let values = (name.eq(&self.name), description.eq(&self.description));

            let pipeline = diesel::insert_into(pipelines)
                .values(values)
                .get_result(&**conn)?;

            self.variables
                .into_iter()
                .try_for_each(|variable| variable.add_to_pipeline(conn, &pipeline))?;

            self.steps
                .into_iter()
                .try_for_each(|step| step.add_to_pipeline(conn, &pipeline))?;

            Ok(pipeline)
        })
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
    use crate::resources::{CreateStepInput, CreateVariableInput, Step, Variable};
    use juniper::{object, FieldResult, GraphQLInputObject, ID};

    /// Contains all the data needed to create a new `Pipeline`.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct CreatePipelineInput {
        /// The name of the pipeline.
        ///
        /// This name is required to be unique.
        ///
        /// Take care to give a short, but descriptive name, to provide maximum
        /// flexibility for the UI, _and_ provide maximum understanding for the
        /// user.
        pub(crate) name: String,

        /// An optional description of the pipeline.
        ///
        /// While the description is optional, it is best-practice to provide
        /// relevant information so that the user of the pipeline knows what to
        /// expect when triggering a pipeline.
        pub(crate) description: Option<String>,

        /// An optional list of variables attached to the pipeline.
        ///
        /// Without variables, a pipeline can only be used for one single
        /// purpose. While this might sometimes be desirable, using variables
        /// provides more flexibility for the user that triggers the pipeline.
        pub(crate) variables: Vec<CreateVariableInput>,

        /// A list of steps attached to the pipeline.
        ///
        /// Not providing any steps will result in a pipeline that has no
        /// functionality.
        ///
        /// Not providing any steps will be considered an error in a future
        /// version of this API.
        pub(crate) steps: Vec<CreateStepInput>,
    }

    /// An optional set of input details to filter a set of `Pipeline`s, based
    /// on either their name, or description.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct SearchPipelineInput {
        /// An optional `name` filter.
        ///
        /// Providing this value will do a `%name%` `ILIKE` query.
        ///
        /// This filter can be combined with the `description` filter, which
        /// will result in a combined `OR` filter.
        pub(crate) name: Option<String>,

        /// An optional `description` filter.
        ///
        /// Providing this value will do a `%description%` `ILIKE` query.
        ///
        /// This filter can be combined with the `name` filter, which
        /// will result in a combined `OR` filter.
        pub(crate) description: Option<String>,
    }

    #[object(Context = Database)]
    impl Pipeline {
        /// The unique identifier for a specific pipeline.
        fn id() -> ID {
            ID::new(self.id.to_string())
        }

        /// A unique and descriptive name of the pipeline.
        fn name() -> &str {
            self.name.as_ref()
        }

        /// An (optional) detailed description of the functionality provided by
        /// this pipeline.
        ///
        /// A description _might_ be markdown formatted, and should be parsed
        /// accordingly by the client.
        fn description() -> Option<&str> {
            self.description.as_ref().map(String::as_ref)
        }

        /// The variables belonging to the pipeline.
        ///
        /// This field can return `null`, but _only_ if a database error
        /// prevents the data from being retrieved.
        ///
        /// If no variables are attached to a pipeline, an empty array is
        /// returned instead.
        ///
        /// If a `null` value is returned, it is up to the client to decide the
        /// best course of action. The following actions are advised, sorted by
        /// preference:
        ///
        /// 1. continue execution if the information is not critical to success,
        /// 2. retry the request to try and get the relevant information,
        /// 3. disable parts of the application reliant on the information,
        /// 4. show a global error, and ask the user to retry.
        fn variables(context: &Database) -> FieldResult<Option<Vec<Variable>>> {
            self.variables(context).map(Some).map_err(Into::into)
        }

        /// The steps belonging to the pipeline.
        ///
        /// This field can return `null`, but _only_ if a database error
        /// prevents the data from being retrieved.
        ///
        /// If no steps are attached to a pipeline, an empty array is returned
        /// instead.
        ///
        /// If a `null` value is returned, it is up to the client to decide the
        /// best course of action. The following actions are advised, sorted by
        /// preference:
        ///
        /// 1. continue execution if the information is not critical to success,
        /// 2. retry the request to try and get the relevant information,
        /// 3. disable parts of the application reliant on the information,
        /// 4. show a global error, and ask the user to retry.
        fn steps(context: &Database) -> FieldResult<Option<Vec<Step>>> {
            self.steps(context).map(Some).map_err(Into::into)
        }
    }

}

impl<'a> TryFrom<&'a graphql::CreatePipelineInput> for NewPipeline<'a> {
    type Error = String;

    fn try_from(input: &'a graphql::CreatePipelineInput) -> Result<Self, Self::Error> {
        let mut pipeline = Self::new(&input.name, input.description.as_ref().map(String::as_ref));

        let variables = input.variables.iter().map(Into::into).collect();
        let steps = input
            .steps
            .iter()
            .enumerate()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, Self::Error>>()?;

        pipeline.with_variables(variables);
        pipeline.with_steps(steps);
        Ok(pipeline)
    }
}
