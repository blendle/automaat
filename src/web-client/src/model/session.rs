//! A session represents the authenticated state of a user.

use crate::graphql::fetch_session_details::FetchSessionDetailsSession;
use dodrio::{RootRender, VdomWeak};
use futures::future::Future;
use std::fmt;

/// This enum can be used to signal if a session has access to some resource,
/// and if not, why that is.
#[derive(Eq, PartialEq)]
pub(crate) enum AccessMode {
    /// The session has the required access to this resource.
    Ok,

    /// The session is authenticated, but lacks the proper authorization.
    ///
    /// The session will have to be granted more privileges for the access mode
    /// to change.
    Unauthorized,

    /// The session is not (yet) authenticated, it might have access once
    /// authenticated, or it might lack sufficient authorization.
    Unauthenticated,
}

impl fmt::Display for AccessMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessMode::Ok => f.write_str("ok"),
            AccessMode::Unauthorized => f.write_str("unauthorized"),
            AccessMode::Unauthenticated => f.write_str("unauthenticated"),
        }
    }
}

/// A session is a known authenticated state at the server, based on a token
/// provided by the client.
///
/// The session stores a cache of all preferences tracked at the server.
pub(crate) struct Session {
    /// A list of privileges tied to the active session.
    ///
    /// These privileges dictate which parts of the application are accessible,
    /// and which are restricted.
    ///
    /// Specifically, it determines which mutation APIs are available to the
    /// application, and which tasks can be run.
    pub(crate) privileges: Vec<String>,
}

impl From<FetchSessionDetailsSession> for Session {
    fn from(details: FetchSessionDetailsSession) -> Self {
        Self {
            privileges: details.privileges,
        }
    }
}

/// The actions a controller has to implement to bridge between the UI and the
/// model.
pub(crate) trait Actions {
    /// Try to authenticate with the server using the provided token.
    ///
    /// If authentication succeeds, the `App#session` details are updated.
    ///
    /// If no token is passed in, the token is fetched from the cookie.
    fn authenticate(
        root: &mut dyn RootRender,
        vdom: VdomWeak,
        token: Option<String>,
    ) -> Box<dyn Future<Item = (), Error = ()>>;
}
