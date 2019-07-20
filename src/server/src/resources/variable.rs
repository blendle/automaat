//! Each [`Task`] contains zero or more variables.
//!
//! A `Variable` is a "runtime" (or "deferred") value that is substituted for
//! any templated [`Processor`] configuration values.
//!
//! The person building the task is required to provide all the
//! configuration values needed to run the steps added to the task, but can
//! choose to use a template value, such as `{country code}` instead of an
//! actual value, and attach a `country code` variable to the task.
//!
//! Now, whenever a task is triggered, the person triggering the task is
//! required to provide the actual value for the `country code` variable.
//!
//! This way, task creators can create tasks that are as easy to use as
//! possible, while still allowing a task to be used for multiple purposes
//! (in this example, the task could be configured to print the weather
//! forecast for the specified country).
//!
//! An optional description can be provided to give some extra context for the
//! person triggering the task. For example:
//!
//! > A `ISO 3166-1 alpha-2` formatted country code.
//!
//! [`Processor`]: crate::Processor

use crate::resources::Task;
use crate::schema::variables;
use crate::State;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::convert::{AsRef, TryFrom};

/// The model representing a variable definition (without an actual value)
/// stored in the database.
#[derive(Clone, Debug, Deserialize, Serialize, Associations, Identifiable, Queryable)]
#[belongs_to(Task)]
#[table_name = "variables"]
pub(crate) struct Variable {
    pub(crate) id: i32,
    pub(crate) key: String,
    pub(crate) description: Option<String>,
    // TODO: figure how to use Diesel's `embed` feature to move this into a
    // `VariableConstraint` struct, which can also hold other constraints (such
    // as `optional: bool`) in the future.
    pub(crate) selection_constraint: Option<Vec<String>>,
    pub(crate) default_value: Option<String>,
    pub(crate) example_value: Option<String>,
    pub(crate) task_id: i32,
}

/// Contains all the details needed to store a variable in the database.
///
/// Use [`NewVariable::new`] to initialize this struct.
#[derive(Clone, Debug, Deserialize, Serialize, Insertable)]
#[table_name = "variables"]
pub(crate) struct NewVariable<'a> {
    key: &'a str,
    description: Option<&'a str>,
    selection_constraint: Option<Vec<&'a str>>,
    default_value: Option<&'a str>,
    example_value: Option<&'a str>,
    task_id: Option<i32>,
}

impl<'a> NewVariable<'a> {
    /// Initialize a `NewVariable` struct, which can be inserted into the
    /// database using the [`NewVariable#add_to_task`] method.
    ///
    /// Returns an error if the `default_value` value is provided, but is not a
    /// subset of the values provided in `selection_constraint`.
    pub(crate) fn new(
        key: &'a str,
        selection_constraint: Option<Vec<&'a str>>,
        default_value: Option<&'a str>,
        example_value: Option<&'a str>,
        description: Option<&'a str>,
    ) -> Result<Self, String> {
        if let Some(selection) = &selection_constraint {
            if let Some(default) = &default_value {
                if !selection.contains(default) {
                    return Err(
                        "default value must be included in the selection constraint".to_owned()
                    );
                }
            }
        };

        Ok(Self {
            key,
            description,
            selection_constraint,
            default_value,
            example_value,
            task_id: None,
        })
    }

    /// Add a variable to a [`Task`], by storing it in the database as an
    /// association.
    ///
    /// Requires a reference to a Task, in order to create the correct data
    /// reference.
    pub(crate) fn add_to_task(mut self, conn: &PgConnection, task: &Task) -> QueryResult<()> {
        use crate::schema::variables::dsl::*;
        self.task_id = Some(task.id);

        diesel::insert_into(variables)
            .values(&self)
            .execute(conn)
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
    use crate::resources::Task;
    use juniper::{object, FieldResult, GraphQLInputObject, GraphQLObject, ID};

    /// Contains all the data needed to create a new `Variable`.
    #[derive(Debug, Clone, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct CreateVariableInput {
        /// The key used to match against templated step configurations.
        ///
        /// If a step's string value contains `{server url}`, then setting the
        /// variable's key to `server url` will allow the step value to be
        /// replaced by the eventually provided variable value when triggering a
        /// task.
        pub(crate) key: String,

        /// An optional description that can be used to explain to a person
        /// about to run a task what the intent is of the required variable.
        pub(crate) description: Option<String>,

        /// An optional default value that can be used by clients to pre-fill
        /// the variable value before running a task.
        pub(crate) default_value: Option<String>,

        /// An optional example value that can be used by the clients to show
        /// one possible way to use this variable.
        ///
        /// This is different from the `default_value` in that it should _not_
        /// be pre-filled as a value by the clients, but should optionally be
        /// shown next to the input field as an extra visual aid.
        pub(crate) example_value: Option<String>,

        /// A set of constraints applied to future values attached to this
        /// variable.
        ///
        /// This object is required, even though all existing constraints are
        /// optional. This is to keep our options open for whenever we _do_ want
        /// to add non-optional constraints.
        pub(crate) constraints: VariableConstraintsInput,
    }

    #[derive(Debug, Clone, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct VariableConstraintsInput {
        /// An optional selection constraint.
        ///
        /// A variable value has to match one of the provided selections in
        /// order to be considered a valid variable.
        pub(crate) selection: Option<Vec<String>>,
    }

    /// The set of constraints that apply to a variable value.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLObject)]
    pub(crate) struct VariableConstraints {
        /// An (optional) set of value selection constraints for this variable.
        ///
        /// If this field returns an array, any variable value matching the key
        /// of this variable will need to have its value match one of the
        /// strings inside this array.
        ///
        /// Clients are encouraged to enforce this invariant, for example by
        /// changing the input field into a select box.
        pub(crate) selection: Option<Vec<String>>,
    }

    #[object(Context = State)]
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

        /// An (optional) default value defined for the variable.
        ///
        /// Clients can use this to pre-fill a value, or select the correct
        /// value if a selection constrained is defined for the variable.
        fn default_value() -> Option<&str> {
            self.default_value.as_ref().map(String::as_ref)
        }

        /// An (optional) example value to use as a visual aid in the clients.
        ///
        /// This is different from the `default_value` in that it should _not_
        /// be pre-filled as a value by the clients, but should optionally be
        /// shown next to the input field as an extra visual aid.
        fn example_value() -> Option<&str> {
            self.example_value.as_ref().map(String::as_ref)
        }

        /// A set of value constraints for this variable.
        ///
        /// This object will always be defined, but it might be empty, if no
        /// constraints are actually set for this variable.
        fn constraints() -> VariableConstraints {
            VariableConstraints {
                selection: self
                    .selection_constraint
                    .as_ref()
                    .map(|v| v.iter().map(ToOwned::to_owned).collect()),
            }
        }

        /// The task to which the variable belongs.
        ///
        /// This field can return `null`, but _only_ if a database error
        /// prevents the data from being retrieved.
        ///
        /// Every variable is _always_ attached to a task, so in normal
        /// circumstances, this field will always return the relevant task
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
        fn task(context: &State) -> FieldResult<Option<Task>> {
            use crate::schema::tasks::dsl::*;
            let conn = context.pool.get()?;

            tasks
                .filter(id.eq(self.task_id))
                .first(&conn)
                .map(Some)
                .map_err(Into::into)
        }

        /// This returns a list of tasks that can provide the value for this
        /// variable.
        ///
        /// For example, if this variable's key is `Customer UUID`, then any
        /// task that has a step that advertises that it can provide the value
        /// for a variable with that key will be returned in this query.
        ///
        /// Clients can use this list to help someone using a task that needs
        /// this variable by guiding them to another task that can provide the
        /// value for this variable.
        fn value_advertisers(context: &State) -> FieldResult<Vec<Task>> {
            use crate::models::VariableAdvertisement;
            use crate::schema::{steps, tasks, variable_advertisements};
            use diesel::dsl::any;
            let conn = context.pool.get()?;

            let adverts = VariableAdvertisement::by_key(self.key.as_ref())
                .select(variable_advertisements::step_id);

            let steps = steps::table
                .filter(steps::id.eq(any(adverts)))
                .select(steps::task_id);

            tasks::table
                .filter(tasks::id.eq(any(steps)))
                .get_results(&conn)
                .map_err(Into::into)
        }
    }
}

impl<'a> TryFrom<&'a graphql::CreateVariableInput> for NewVariable<'a> {
    type Error = String;

    fn try_from(input: &'a graphql::CreateVariableInput) -> Result<Self, Self::Error> {
        Self::new(
            &input.key,
            input
                .constraints
                .selection
                .as_ref()
                .map(|v| v.iter().map(String::as_str).collect()),
            input.default_value.as_ref().map(String::as_ref),
            input.example_value.as_ref().map(String::as_ref),
            input.description.as_ref().map(String::as_ref),
        )
    }
}
