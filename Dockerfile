# Build stage: compile the server. Course content and static assets are
# embedded into the binary at compile time, so nothing else ships.
# aws-lc-sys (TLS for the AWS SDK) needs a C toolchain and cmake.
FROM rust:1.95-slim-bookworm AS builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential cmake \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo build --release -p courses_server

# Runtime stage: just the binary plus CA certificates for outbound TLS.
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/courses_server /usr/local/bin/courses_server
ENV PORT=8080
EXPOSE 8080
CMD ["courses_server"]
