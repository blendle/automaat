use crate::graphql::{MutationRoot, QueryRoot, Schema};
use crate::handlers;
use crate::middleware::RemoveContentLengthHeader;
use actix_files::Files;
use actix_web::{
    http::header,
    middleware::{Compress, DefaultHeaders},
    web, App, HttpServer,
};
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::sync::Arc;
use std::{env, error::Error};

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
