FROM rust:1.51 as builder
COPY . .
RUN echo "stable" > rust-toolchain
RUN cargo build -p gcs-server

FROM rust:1.51-slim
WORKDIR /app
COPY --from=builder /target/debug/gcs-server .
EXPOSE 80

ENTRYPOINT ["/app/gcs-server"]
