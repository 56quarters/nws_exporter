FROM rust:slim-bookworm AS BUILD
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=BUILD target/release/nws_exporter /usr/local/bin/
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    apt-get clean
CMD ["nws_exporter", "--help"]
