FROM rust:1.47 as builder
COPY . .
RUN echo "stable" > rust-toolchain
RUN cargo build -p gcs-server --release

FROM rust:1.47-slim
ENV HOST 0.0.0.0
ENV PORT 80
WORKDIR /app
COPY --from=builder /target/release/gcs-server .
EXPOSE 80

ENTRYPOINT ["/app/gcs-server"]
