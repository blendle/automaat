# Automaat Server

🚧 _Work In Progress_ 🚧

## Server Configuration

You can set the following environment variables:

- `DATABASE_URL`: Postgres server FQDN (e.g. `postgres://postgres@localhost`).
- `SERVER_ROOT`: Root of the static files you want to serve (if any).
- `SERVER_BIND`: Address and port to bind to (e.g. `0.0.0.0:443`).
- `SERVER_SSL_KEY_PATH`: Path to your (optional) SSL private key.
- `SERVER_SSL_CHAIN_PATH`: Path to your (optional) SSL chained certificate.
- `SERVER_SECRET`: Optional secret key to encrypt global and local variable values at rest.

## See Also

- [Diesel getting started](https://diesel.rs/guides/getting-started/)
