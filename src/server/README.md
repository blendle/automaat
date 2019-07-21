# Automaat Server

🚧 _Work In Progress_ 🚧

## Server Configuration

You can start the server using `automaat server`.

The following environment variables are used to configure the server.

- `DATABASE_URL`: Postgres server FQDN (e.g. `postgres://postgres@localhost`).
- `ENCRYPTION_SECRET`: Secret key to encrypt global and local variable values at rest.
- `SERVER_ROOT`: Root of the static files you want to serve (if any).
- `SERVER_BIND`: Address and port to bind to (e.g. `0.0.0.0:443`).
- `SERVER_SSL_KEY_PATH`: Path to your (optional) SSL private key.
- `SERVER_SSL_CHAIN_PATH`: Path to your (optional) SSL chained certificate.

## Worker Configuration

You can start the worker using `automaat worker`.

The following environment variables are used to configure the worker.

- `DATABASE_URL`: Postgres server FQDN (e.g. `postgres://postgres@localhost`).
- `ENCRYPTION_SECRET`: Secret key to encrypt global and local variable values at rest.
