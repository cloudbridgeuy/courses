# Build stage: compile the server. Course content and static assets are
# embedded into the binary at compile time, so nothing else ships.
# aws-lc-sys (TLS for the AWS SDK) needs a C toolchain and cmake.
FROM rust:1.95-slim-bookworm@sha256:d7482085ff5b415f84dba5647ae71606650bdef00db7aeb69f4b3d170c3e4082 AS builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential=12.9 cmake=3.25.1-1 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo build --release -p courses_server

# Runtime stage: just the binary plus CA certificates for outbound TLS.
FROM debian:bookworm-slim@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates=20230311+deb12u1 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/courses_server /usr/local/bin/courses_server
ENV PORT=8080
EXPOSE 8080
CMD ["courses_server"]
