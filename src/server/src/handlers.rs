#![allow(clippy::single_match_else)]

use crate::graphql::Schema;
use crate::Database;
use juniper_rocket::{graphiql_source, playground_source, GraphQLRequest, GraphQLResponse};
use rocket::response::content::Html;
use rocket::State;
use rocket_contrib::json::Json;
use serde::{Deserialize, Serialize};

/// See: <https://tools.ietf.org/html/draft-inadarei-api-health-check-03>
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Status {
    Pass,
    _Warn,
    _Fail,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct Health {
    status: Status,
    version: &'static str,
    release_id: &'static str,
}

#[get("/graphql/graphiql")]
pub(super) fn graphiql() -> Html<String> {
    graphiql_source("/graphql")
}

#[get("/graphql/playground")]
pub(super) fn playground() -> Html<String> {
    playground_source("/graphql")
}

#[get("/graphql?<request>")]
#[allow(clippy::needless_pass_by_value)]
pub(super) fn query(
    db: Database,
    request: GraphQLRequest,
    schema: State<'_, Schema>,
) -> GraphQLResponse {
    request.execute(&schema, &db)
}

#[post("/graphql", data = "<request>")]
#[allow(clippy::needless_pass_by_value)]
pub(super) fn mutate(
    db: Database,
    request: GraphQLRequest,
    schema: State<'_, Schema>,
) -> GraphQLResponse {
    request.execute(&schema, &db)
}

#[get("/health")]
pub(super) const fn health() -> Json<Health> {
    let health = Health {
        status: Status::Pass,
        version: "TODO",
        release_id: "TODO",
    };

    Json(health)
}
