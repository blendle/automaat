use crate::models::NewGlobalVariable;
use serde::{Deserialize, Serialize};

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
    use juniper::{GraphQLEnum, GraphQLInputObject};

    /// Define what to do when a conflict occurs on object mutation.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLEnum)]
    pub(crate) enum OnConflict {
        Abort,
        Update,
    }

    /// Create a new global variable.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct GlobalVariableInput {
        /// The key of the global variable.
        ///
        /// This is the name by which the global variable can be referenced in
        /// the task step definitions.
        ///
        /// For example, if the key is named `foo`, then the global variable can
        /// be accessed using the syntax `{{ global['foo'] }}`.
        pub(crate) key: String,

        /// The value of the global variable.
        ///
        /// This can contain sensitive data, as all global variables (similar to
        /// task variables) are encrypted at rest.
        pub(crate) value: String,

        /// Define what to do when the global variable key already exists.
        ///
        /// By default, updating an existing key is disallowed, to prevent
        /// accidentally overriding existing values, potentially breaking
        /// existing tasks.
        ///
        /// You can set this value to `UPDATE` to force the existing key to be
        /// updated to the newly provided value.
        pub(crate) on_conflict: Option<OnConflict>,
    }
}

impl<'a> From<&'a graphql::GlobalVariableInput> for NewGlobalVariable<'a> {
    fn from(input: &'a graphql::GlobalVariableInput) -> Self {
        Self::new(&input.key, &input.value)
    }
}
