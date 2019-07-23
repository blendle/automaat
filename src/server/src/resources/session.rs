use crate::models::{NewSession, Session};
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
    use crate::server::RequestState;
    use juniper::{object, GraphQLInputObject, ID};

    /// Contains all the data needed to create a new `Session`.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct CreateSessionInput {
        /// An optional set of labels attached to a session.
        ///
        /// Labels can be used to restrict who can run what session.
        pub(crate) privileges: Option<Vec<String>>,
    }

    /// Contains all the data needed to update session privileges.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct UpdatePrivilegesInput {
        #[serde(with = "juniper_serde")]
        pub(crate) id: ID,
        pub(crate) privileges: Vec<String>,
    }

    #[object(Context = RequestState)]
    impl Session {
        /// The unique identifier for a specific session.
        fn id() -> ID {
            ID::new(self.id.to_string())
        }

        /// Privileges attached to a session.
        ///
        /// Privileges determine which tasks the session is allowed to run,
        /// based on if that task has a label attached that matches the
        /// privilege name.
        fn privileges() -> Vec<&str> {
            self.privileges.iter().map(String::as_str).collect()
        }
    }
}

impl<'a> From<&'a graphql::CreateSessionInput> for NewSession<'a> {
    fn from(input: &'a graphql::CreateSessionInput) -> Self {
        Self::new(
            input
                .privileges
                .as_ref()
                .map_or(vec![], |l| l.iter().map(String::as_str).collect()),
        )
    }
}
