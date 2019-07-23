//! The GraphQL service is a thin wrapper around a GraphQL-capable HTTP client.

use crate::CookieService;
use failure::{Compat, Fail};
use futures::future::Future;
use graphql_client::{web, GraphQLQuery, Response};
use std::{error, fmt};

/// The GraphQL service.
#[derive(Clone)]
pub(crate) struct Service {
    /// The endpoint of the GraphQL API.
    endpoint: String,

    /// The cookie service used to store and clean up authentication
    /// credentials.
    cookie: CookieService,
}

/// An encapsulation of all possible errors triggered by a GraphQL API request.
#[derive(Debug)]
pub(crate) enum Error {
    /// GraphQL client error.
    Client(Compat<web::ClientError>),

    /// Authentication error.
    Authentication,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Client(err) => write!(f, "{}", err),
            Error::Authentication => f.write_str("authentication"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Client(err) => Some(err),
            Error::Authentication => None,
        }
    }
}

impl Service {
    /// Create a new GraphQL service.
    pub(crate) fn new<T: Into<String>>(endpoint: T, cookie: CookieService) -> Self {
        Self {
            endpoint: endpoint.into(),
            cookie,
        }
    }

    /// Perform a request to the GraphQL server.
    pub(crate) fn request<Q: GraphQLQuery + 'static>(
        &self,
        query: Q,
        variables: Q::Variables,
    ) -> impl Future<Item = Response<Q::ResponseData>, Error = Error> + 'static {
        let mut client = web::Client::new(self.endpoint.as_str());

        // TODO: cache this value in memory.
        if let Some(ref auth) = self.cookie.get("session") {
            client.add_header("authorization", auth);
        }

        let cookie = self.cookie.clone();
        client
            .call(query, variables)
            .map_err(|err| Error::Client(err.compat()))
            .and_then(move |response| {
                if let Some(errors) = &response.errors {
                    if errors.iter().any(|e| e.message == "Unauthorized") {
                        cookie.remove("session");
                        return futures::future::err(Error::Authentication);
                    }
                }

                futures::future::ok(response)
            })
    }
}
