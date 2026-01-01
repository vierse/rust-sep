# Project

Our app consists of server (the Rust/axum part) and web UI (the TypeScript/React part).

Project folders:
- `.github`: CI pipeline
- `.sqlx`: sqlx query cache, to check SQL queries without having the database running
- `docker`: contains a Dockerfile that builds a Docker image with our app (for the final submission)
- `migrations`: SQL queries that are used by `sqlx migrate run` to initialize the database
- `scripts`: Scripts, `quality.sh` runs clippy and format check on your code, `build-docker-image.sh` builds a docker image for the final submission (usually not needed, as it's handled by docker compose)
- `src`: Source code for the server
    - `api/handlers`: Handlers that are used by API to process the request
    - `api/router.rs`: Creates a `Router` that defines the API endpoints
    - `bin/server.rs`: The entry point of the app
    - `app.rs`: Main app functionality
    - `config.rs`: Module to parse application settings from `settings.yml` and from environment variables
- `target`: Cargo build artifacts
- `tests`: Integration tests
- `web`: Web UI
    - `dist`: Generated after building the web UI according to `README.md`, containing the compiled web UI that `axum` serves to the user.
    - `node_modules`: Generated automatically, contains web UI dependencies
    - `src`: Contains the code for web UI
    - `deno.lock`: A lockfile for web UI dependencies
    - `...`: Various config files for JS/TS stuff, were included in template React project
- `.dockerignore`: Lists stuff that Docker needs to ignore when building an image
- `.env`: Needs to be manually created. Contains environment variables with various settings for the project, such as network port, database connection info, etc.
- `.env.example`: An example of a `.env` file with default settings
- `compose.yml`: Defines services for `docker compose`, for example the database.
- `deno.json`: Deno tasks, used by Docker
- `settings.yml`: App settings, have to match `.env` variables for now
