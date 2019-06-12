# Automaat Server

🚧 _Work In Progress_ 🚧

## Database Configuration

You need to set the following environment variables:

```shell
# needed for the Rocket webserver to connect
ROCKET_DATABASES='{db={url="postgres://postgres@localhost"}}'

# needed for the Diesel ORM to run migrations
DATABASE_URL="postgres://postgres@localhost"
```

## See Also

* [Rocket environment variables](https://rocket.rs/v0.4/guide/configuration/#environment-variables)
* [Rocket database configuration](https://api.rocket.rs/v0.4/rocket_contrib/databases/index.html#configuration)
* [Deisel getting started](https://diesel.rs/guides/getting-started/)
