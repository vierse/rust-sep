# Rust SEP - URL Shortener

## Setup


1. Install [Docker](https://docs.docker.com/get-started/get-docker/).

2. You need to provide `.env` file that defines environment variables for the project. You can just copy `.env.default`:
```
cp .env.default .env
```

3. Build web UI:
```
docker compose run --rm web-build
```

4. Start database container:
```
docker compose up postgres -d
```

5. Run the server:
```
cargo run --bin server
```

(Optional) You can also set `RUST_LOG` to print traces:
```
RUST_LOG=debug cargo run --bin server
```

6. By default it should be available at: http://localhost:3000/

7. To stop the database container:
```
docker compose down -v
```
Omit `-v` if you want to keep the data.

## SQL
If you add SQL queries, you'll need to generate sqlx cache for compile-time checking without a database running:
```
cargo sqlx prepare
```
sqlx cache is stored in `.sqlx` and should be included in Git.

## Tests

Run:
```
cargo test
```

## Making requests with `curl`

`POST` Request:
```
curl -X POST http://localhost:3000/api/shorten \
     -H "Content-Type: application/json" \
     -d '{"url": "https://example.com"}' \
     -w "\nStatus: %{http_code}\n"
```
`GET` Request:
```
curl http://localhost:3000/abcxyz \
     -w "\nStatus: %{http_code}\n"
```
