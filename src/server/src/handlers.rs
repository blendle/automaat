use crate::{GraphQLSchema, State};
use actix_web::web::{block, Data, Json};
use actix_web::{Error, HttpResponse};
use futures::future::Future;
use juniper::http::{graphiql, playground, GraphQLRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

pub(super) fn graphiql() -> HttpResponse {
    let html = graphiql::graphiql_source("/graphql");
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

pub(super) fn playground() -> HttpResponse {
    let html = playground::playground_source("/graphql");
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

pub(super) fn graphql(
    state: Data<Arc<State>>,
    request: Json<GraphQLRequest>,
    schema: Data<GraphQLSchema>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    block(move || {
        let response = request.execute(&schema, &state);
        serde_json::to_string(&response)
    })
    .map_err(Into::into)
    .and_then(|response| {
        Ok(HttpResponse::Ok()
            .content_type("application/json")
            .header("Cache-Control", "no-cache")
            .body(response))
    })
}

pub(super) fn health() -> HttpResponse {
    let health = Health {
        status: Status::Pass,
        version: "TODO",
        release_id: "TODO",
    };

    HttpResponse::Ok()
        .header("Cache-Control", "no-cache")
        .json(health)
}
