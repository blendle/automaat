use crate::graphql::Schema;
use crate::models::Session;
use crate::server::{RequestState, ServerError, State};
use actix_web::web::{block, Data, Json};
use actix_web::{HttpRequest, HttpResponse};
use diesel::pg::PgConnection;
use futures::future::Future;
use juniper::http::{graphiql, playground, GraphQLRequest};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

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
    (graphql, request): (Json<GraphQLRequest>, HttpRequest),
    schema: Data<Arc<Schema>>,
) -> impl Future<Item = HttpResponse, Error = ServerError> {
    let token = auth_token(&request);

    block(move || {
        let conn = state.pool.get()?;
        let session = authenticate(&token?, &conn)?;
        let response = graphql.execute(&schema, &RequestState::new(conn, session));

        serde_json::to_string(&response).map_err(Into::<ServerError>::into)
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

fn authenticate(token: &str, conn: &PgConnection) -> Result<Session, ServerError> {
    Uuid::from_str(token)
        .ok()
        .and_then(|token| Session::find_by_token(token, conn).ok())
        .ok_or(ServerError::Authentication)
}

fn auth_token(request: &HttpRequest) -> Result<String, ServerError> {
    use actix_web::http::header;

    request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().map(str::to_owned).ok())
        .ok_or(ServerError::Authentication)
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;
    use diesel::prelude::*;
    use diesel::result::Error;

    fn connection() -> PgConnection {
        PgConnection::establish("postgres://postgres@localhost").unwrap()
    }

    #[test]
    #[should_panic]
    fn test_authenticate_invalid_uuid() {
        let _ = authenticate("invalid-uuid", &connection()).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_authenticate_unknown_session() {
        let uuid = Uuid::new_v4().to_string();
        let _ = authenticate(&uuid, &connection()).unwrap();
    }

    #[test]
    fn test_authenticate_known_session() {
        let conn = connection();

        conn.test_transaction::<_, Error, _>(|| {
            let session = Session::create(&conn).unwrap();
            let auth = authenticate(&session.token.to_string(), &conn).unwrap();

            assert_eq!(session.token, auth.token);
            Ok(())
        });
    }

    #[test]
    #[should_panic]
    fn test_auth_token_missing() {
        let req = test::TestRequest::default().to_http_request();

        let _ = auth_token(&req).unwrap();
    }

    #[test]
    fn test_auth_token_exists() {
        let req = test::TestRequest::with_header("authorization", "token").to_http_request();

        let _ = auth_token(&req).unwrap();
    }
}
