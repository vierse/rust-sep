FROM rust:1-trixie AS build
COPY --from=denoland/deno:bin-2.6.3 /deno /usr/local/bin/deno

WORKDIR /build

COPY . ./

# web UI
RUN deno task web:install
RUN deno task web:build

# api
RUN cargo build --release

FROM debian:trixie
COPY --from=build /build/web/dist ./web/dist
COPY --from=build /build/target/release/server ./server

CMD ["./server"]