//! A [`Task`] is a collection of [`Step`]s and [`Variable`]s, wrapped in
//! one package, containing a descriptive name and optional documentation.
//!
//! Each task is pre-configured for usage, after which it can be
//! "triggered".
//!
//! Before a task can be triggered, the person wanting to trigger the
//! task needs to first provide all values for the variables attached to the
//! task. For more details on variables, see the [`variable`] module
//! documentation.
//!
//! Once all variable values are provided, the task can be triggered.
//! Triggering a task results in a [`Job`] being created, which will be
//! picked up by the job runner immediately.
//!
//! [`variable`]: crate::resources::variable

use crate::resources::{NewStep, NewVariable, Step, Variable};
use crate::schema::tasks;
use crate::State;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::error;

#[derive(Clone, Debug, Deserialize, Serialize, Identifiable, Queryable)]
#[table_name = "tasks"]
/// The model representing a task stored in the database.
pub(crate) struct Task {
    pub(crate) id: i32,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
}

impl Task {
    pub(crate) fn steps(&self, conn: &PgConnection) -> QueryResult<Vec<Step>> {
        use crate::schema::steps::dsl::*;

        Step::belonging_to(self)
            .order((position.asc(), id.asc()))
            .load(conn)
    }

    pub(crate) fn variables(&self, conn: &PgConnection) -> QueryResult<Vec<Variable>> {
        use crate::schema::variables::dsl::*;

        Variable::belonging_to(self).order(id.desc()).load(conn)
    }

    /// Return the task variable matching the given key, if any.
    pub(crate) fn variable_with_key(
        &self,
        key: &str,
        conn: &PgConnection,
    ) -> QueryResult<Option<Variable>> {
        use crate::schema::variables::dsl::key as vkey;

        Variable::belonging_to(self)
            .filter(vkey.eq(key))
            .first(conn)
            .optional()
    }
}

/// Contains all the details needed to store a task in the database.
///
/// Use [`NewTask::new`] to initialize this struct.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct NewTask<'a> {
    name: &'a str,
    description: Option<&'a str>,
    variables: Vec<NewVariable<'a>>,
    steps: Vec<NewStep<'a>>,
}

impl<'a> NewTask<'a> {
    /// Initialize a `NewTask` struct, which can be inserted into the
    /// database using the [`NewTask#create`] method.
    pub(crate) fn new(name: &'a str, description: Option<&'a str>) -> Self {
        Self {
            name,
            description,
            variables: vec![],
            steps: vec![],
        }
    }

    /// Attach variables to this task.
    ///
    /// `NewTask` takes ownership of the variables, but you are required to
    /// call [`NewTask#create`] to persist the task and its variables.
    ///
    /// Can be called multiple times to append more variables.
    pub(crate) fn with_variables(&mut self, mut variables: Vec<NewVariable<'a>>) {
        self.variables.append(&mut variables)
    }

    /// Attach steps to this task.
    ///
    /// `NewTask` takes ownership of the steps, but you are required to
    /// call [`NewTask#create`] to persist the task and its steps.
    ///
    /// Can be called multiple times to append more steps.
    pub(crate) fn with_steps(&mut self, mut steps: Vec<NewStep<'a>>) {
        self.steps.append(&mut steps)
    }

    /// Persist the task and any attached variables and steps into the
    /// database.
    ///
    /// Persisting the data happens within a transaction that is rolled back if
    /// any data fails to persist.
    pub(crate) fn create(self, conn: &PgConnection) -> Result<Task, Box<dyn error::Error>> {
        conn.transaction(|| {
            use crate::schema::tasks::dsl::*;

            // waiting on https://github.com/diesel-rs/diesel/issues/860
            let values = (name.eq(&self.name), description.eq(&self.description));

            let task = diesel::insert_into(tasks).values(values).get_result(conn)?;

            self.variables
                .into_iter()
                .try_for_each(|variable| variable.add_to_task(conn, &task))?;

            self.steps
                .into_iter()
                .try_for_each(|step| step.add_to_task(conn, &task))?;

            Ok(task)
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

    /// Contains all the data needed to create a new `Task`.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct CreateTaskInput {
        /// The name of the task.
        ///
        /// This name is required to be unique.
        ///
        /// Take care to give a short, but descriptive name, to provide maximum
        /// flexibility for the UI, _and_ provide maximum understanding for the
        /// user.
        pub(crate) name: String,

        /// An optional description of the task.
        ///
        /// While the description is optional, it is best-practice to provide
        /// relevant information so that the user of the task knows what to
        /// expect when triggering a task.
        pub(crate) description: Option<String>,

        /// An optional list of variables attached to the task.
        ///
        /// Without variables, a task can only be used for one single
        /// purpose. While this might sometimes be desirable, using variables
        /// provides more flexibility for the user that triggers the task.
        pub(crate) variables: Vec<CreateVariableInput>,

        /// A list of steps attached to the task.
        ///
        /// Not providing any steps will result in a task that has no
        /// functionality.
        ///
        /// Not providing any steps will be considered an error in a future
        /// version of this API.
        pub(crate) steps: Vec<CreateStepInput>,
    }

    /// An optional set of input details to filter a set of `Task`s, based
    /// on either their name, or description.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct SearchTaskInput {
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

    #[object(Context = State)]
    impl Task {
        /// The unique identifier for a specific task.
        fn id() -> ID {
            ID::new(self.id.to_string())
        }

        /// A unique and descriptive name of the task.
        fn name() -> &str {
            self.name.as_ref()
        }

        /// An (optional) detailed description of the functionality provided by
        /// this task.
        ///
        /// A description _might_ be markdown formatted, and should be parsed
        /// accordingly by the client.
        fn description() -> Option<&str> {
            self.description.as_ref().map(String::as_ref)
        }

        /// The variables belonging to the task.
        ///
        /// This field can return `null`, but _only_ if a database error
        /// prevents the data from being retrieved.
        ///
        /// If no variables are attached to a task, an empty array is
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
        fn variables(context: &State) -> FieldResult<Option<Vec<Variable>>> {
            let conn = context.pool.get()?;

            self.variables(&conn).map(Some).map_err(Into::into)
        }

        /// The steps belonging to the task.
        ///
        /// This field can return `null`, but _only_ if a database error
        /// prevents the data from being retrieved.
        ///
        /// If no steps are attached to a task, an empty array is returned
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
        fn steps(context: &State) -> FieldResult<Option<Vec<Step>>> {
            let conn = context.pool.get()?;

            self.steps(&conn).map(Some).map_err(Into::into)
        }
    }

}

impl<'a> TryFrom<&'a graphql::CreateTaskInput> for NewTask<'a> {
    type Error = String;

    fn try_from(input: &'a graphql::CreateTaskInput) -> Result<Self, Self::Error> {
        let mut task = Self::new(&input.name, input.description.as_ref().map(String::as_ref));

        let variables = input
            .variables
            .iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, String>>()?;

        let steps = input
            .steps
            .iter()
            .enumerate()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, Self::Error>>()?;

        task.with_variables(variables);
        task.with_steps(steps);
        Ok(task)
    }
}
