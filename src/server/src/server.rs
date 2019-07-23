use crate::graphql::{MutationRoot, QueryRoot, Schema};
use crate::handlers;
use crate::middleware::RemoveContentLengthHeader;
use crate::models::Session;
use actix_files::Files;
use actix_web::error::BlockingError;
use actix_web::{
    http::{header, StatusCode},
    middleware::{Compress, DefaultHeaders},
    web, App, HttpResponse, HttpServer, ResponseError,
};
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::sync::Arc;
use std::{env, error::Error, fmt};

pub(crate) struct RequestState {
    pub(crate) conn: PooledConnection<ConnectionManager<PgConnection>>,

    /// The session state defines if a request comes from an authenticated
    /// session, and contains any configuration related to that session.
    ///
    /// If the session is `None`, it means the request is from an
    /// unauthenticated request. This can only happen if no authentication
    /// details were provided. If details _are_ provided, but they do not match
    /// any known session data, an authorization error is returned instead.
    pub(crate) session: Option<Session>,
}

impl RequestState {
    pub(crate) const fn new(
        conn: PooledConnection<ConnectionManager<PgConnection>>,
        session: Option<Session>,
    ) -> Self {
        Self {
            conn,
            session: session,
        }
    }
}

#[derive(Debug)]
pub(crate) enum ServerError {
    Authentication,
    Json(serde_json::Error),
    Internal(String),
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            ServerError::Authentication => "Unauthorized".to_owned(),
            ServerError::Json(err) => err.to_string(),
            ServerError::Internal(string) => string.to_owned(),
        };

        write!(f, r#"{{ "errors": [{{ "message": "{}" }}] }}"#, message)
    }
}

impl ResponseError for ServerError {
    fn error_response(&self) -> HttpResponse {
        let code = match self {
            ServerError::Authentication => StatusCode::UNAUTHORIZED,
            ServerError::Json(_) => StatusCode::BAD_REQUEST,
            ServerError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        HttpResponse::new(code)
    }
}

impl From<serde_json::Error> for ServerError {
    fn from(err: serde_json::Error) -> Self {
        ServerError::Json(err)
    }
}

impl<T> From<BlockingError<T>> for ServerError
where
    T: Into<ServerError> + fmt::Debug,
{
    fn from(err: BlockingError<T>) -> Self {
        match err {
            BlockingError::Error(err) => err.into(),
            BlockingError::Canceled => ServerError::Internal("canceled".to_owned()),
        }
    }
}

pub(crate) trait InternalServerError: fmt::Display {}
impl InternalServerError for r2d2::Error {}

impl<T> From<T> for ServerError
where
    T: InternalServerError,
{
    fn from(err: T) -> Self {
        ServerError::Internal(err.to_string())
    }
}

pub(crate) type DatabasePool = Pool<ConnectionManager<PgConnection>>;

pub(crate) struct State {
    pub(crate) pool: DatabasePool,
}

pub(crate) struct Server {
    state: State,
}

impl Server {
    pub(crate) fn from_environment() -> Result<Self, Box<dyn Error>> {
        let database_url = env::var("DATABASE_URL")?;
        let pool = Pool::new(ConnectionManager::new(database_url))?;

        crate::embedded_migrations::run(&pool.get()?)?;

        Ok(Self {
            state: State { pool },
        })
    }

    pub(crate) fn run_to_completion(self) -> Result<(), Box<dyn Error>> {
        let bind = env::var("SERVER_BIND").unwrap_or_else(|_| "0.0.0.0:8000".to_owned());
        let schema = Arc::new(Schema::new(QueryRoot, MutationRoot));
        let state = Arc::new(self.state);

        let server = HttpServer::new(move || {
            let root = env::var("SERVER_ROOT").unwrap_or_else(|_| "/public".to_owned());

            App::new()
                .wrap(Compress::default())
                .wrap(
                    DefaultHeaders::new()
                        .header(header::CACHE_CONTROL, "max-age=43200, must-revalidate")
                        .header(header::VARY, "Accept-Encoding, Accept, Accept-Language"),
                )
                // TODO: Fix wrong Content-Length header value: https://git.io/fjV2B
                .wrap(RemoveContentLengthHeader)
                .data(state.clone())
                .data(schema.clone())
                .route("/graphql/playground", web::get().to(handlers::playground))
                .route("/graphql/graphiql", web::get().to(handlers::graphiql))
                .route("/graphql", web::get().to_async(handlers::graphql))
                .route("/graphql", web::post().to_async(handlers::graphql))
                .route("/health", web::get().to(handlers::health))
                .service(Files::new("/", root).index_file("index.html"))
        });

        let server = if let Ok(key_path) = env::var("SERVER_SSL_KEY_PATH") {
            let chain_path = env::var("SERVER_SSL_CHAIN_PATH")?;

            let mut builder = SslAcceptor::mozilla_modern(SslMethod::tls())?;

            builder.set_private_key_file(key_path, SslFiletype::PEM)?;
            builder.set_certificate_chain_file(chain_path)?;

            server.bind_ssl(bind, builder)
        } else {
            server.bind(bind)
        }?;

        server.run().map_err(Into::into)
    }
}
