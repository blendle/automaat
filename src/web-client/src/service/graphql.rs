//! The GraphQL service is a thin wrapper around a GraphQL-capable HTTP client.

use futures::future::Future;
use graphql_client::{web, GraphQLQuery, Response};

/// The GraphQL service.
#[derive(Clone)]
pub(crate) struct Service {
    /// The endpoint of the GraphQL API.
    endpoint: String,
}

impl Service {
    /// Create a new GraphQL service.
    pub(crate) fn new<T: Into<String>>(endpoint: T) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    /// Perform a request to the GraphQL server.
    pub(crate) fn request<Q: GraphQLQuery + 'static>(
        &self,
        query: Q,
        variables: Q::Variables,
    ) -> impl Future<Item = Response<Q::ResponseData>, Error = web::ClientError> + 'static {
        web::Client::new(self.endpoint.as_str()).call(query, variables)
    }
}
