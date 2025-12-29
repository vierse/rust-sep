# Rust SEP - URL Shortener

## Usage

To run the server:
```
cargo run --bin server
```
Listens on `localhost:3000`.


To run the tests:
```
cargo test
```

## Docker:

Build the image:
```
./scripts/build-docker-image.sh
```

Create a container:
```
docker run -p 3000:3000 shorten-app
```

Web UI should be accessible at `127.0.0.1:3000`

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