# Rust SEP - URL Shortener

## What's this about?
This repository contains an implementation of a URL shortener web service using `axum` for backend and a `React` SPA for frontend as part of software engineering lab at LMU in winter semester.

At this point one can:
1. Shorten links (with generated or user-provided alias, optionally protected by a password)
2. Create a user account to be able to view the shortened links
3. Convert individual links into collections
4. Create collections from several links at once, similarly with generated or user-provided alias and optional password
5. Look up ownership/password status and metrics of individual links

A full outline of our API is provided in `docs/api.md`.

## Quick Setup

0. Install [Docker](https://docs.docker.com/get-started/get-docker/)

1. Check out the repository, `cd` to project root and setup `.env` (you can just copy `.env.default`):
```
git clone https://github.com/vierse/rust-sep.git
cd rust-sep
cp .env.default .env
```

2. Build Docker image and start the containers (app + DB, for optional services see below)
```
docker compose up --build app
```

3. Done! By default, the app will be available at `localhost:3000`.

4. To bring it all down run:
```
docker compose down -v
```
Omit `-v` to keep Docker volumes.

## Other Services

### Metrics

Docker compose includes a bunch of other services too. Mainly, we use `metrics` to emit Prometheus metrics through a separate port, by default `9000`. If the app is running in Docker, it will only be accessible to Docker's network.

To gather these metrics, Prometheus server must be started too:
```
docker compose up prometheus
```
Then Prometheus will be available at `localhost:9090` port.

Grafana can then be used to create dashboards. Start it like so:
```
docker compose up grafana
```
Granted, this is not something we've had a lot of time to experiment with.

### Load Testing

We used `locust` to generate synthetic traffic to our server. `locust/locustfile.py` describes various scenarios of how our API might be interacted with. We don't consider it to be exhaustive, but a decent general measure of the server's capability to handle many concurrent requests.

Tests were performed from a separate host over local network. First, a dataset needs to be created using the provided Python script:
```
cd locust
python create_datasets.py
```
This will generate 10_000 url entries, shorten them and store the resulting aliases. Then to run locust:
```
docker compose up locust
```
Its UI will be available at `localhost:8089`, where it will be possible to set the target amount of users, run time and the target's address.


## Development

You can also run the backend's binary directly:
```
cargo run --bin server
```
With `RUST_LOG=debug` to print debug traces:
```
RUST_LOG=debug cargo run --bin server
```

Note that in this case, you need to compile and bundle the frontend manually. This can be done with `deno` (provided you have `deno` installed) or Docker (in case you don't):
```
# with Docker
docker compose run --rm web-build

# with deno
deno task web:build
```

### SQLx

We use `sqlx` with compile-time checked queries. To compile we can use cached queries in `.sqlx/`. For that `SQLX_OFFLINE` must be set to `true` in `.env`. In this case the app can also run SQL migrations on its own.

For development we need a live DB and `SQLX_OFFLINE` must be set to `false`, as we can't generate `sqlx` cache otherwise. In this case, `sqlx-cli` is required to run migrations:
```
# start PostgreSQL container
docker compose up postgres

cargo install --locked sqlx-cli
sqlx migrate run
```

To generate `.sqlx` cache:
```
cargo sqlx prepare
```

To create a new migration (following our scheme):
```
sqlx migrate add
```

To run tests (requires live DB with migrations applied):
```
cargo test
```

### Web UI

For frontend development we use `Vite` to serve our web UI. 

`Vite` then routes the API requests to backend. This behavior can be configured in `web/vite.config.ts`. To run `Vite` we used `deno`, see `deno.json` for possible tasks.

To just run `Vite`'s dev server:
```
deno task web:dev
```

### Making requests with `curl`

`POST` Request:
```
curl -i localhost:3000/api/shorten \
  -H 'Content-Type: application/json' \
  -d '{"url":"https://example.com"}'
```
`GET` Request:
```
curl -i localhost:3000/r/abcxyz
```
