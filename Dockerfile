# Build stage: compile the server. Course content and static assets are
# embedded into the binary at compile time, so nothing else ships.
# aws-lc-sys (TLS for the AWS SDK) needs a C toolchain and cmake.
#
# The build is split with cargo-chef so that dependency compilation lives in its
# own layer, keyed only on Cargo.toml/Cargo.lock. Editing content/ or crates/
# then reuses the cached dependency layer instead of rebuilding the AWS SDK.
# RUSTFLAGS and the toolchain are set here, in the shared base, so that
# `chef cook` and `cargo build` agree. A mismatch invalidates every cooked
# artifact.
FROM rust:1.95-slim-bookworm@sha256:d7482085ff5b415f84dba5647ae71606650bdef00db7aeb69f4b3d170c3e4082 AS chef
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential=12.9 cmake=3.25.1-1 lld=1:14.0-55.7~deb12u1 \
    && rm -rf /var/lib/apt/lists/*
ENV RUSTFLAGS="-C link-arg=-fuse-ld=lld"
WORKDIR /app
# Resolve the pinned toolchain and its components once, here, so the source
# layers below never stop to run rustup.
COPY rust-toolchain.toml .
RUN rustup show \
    && cargo install cargo-chef --locked --version 0.1.77

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release -p courses_server --recipe-path recipe.json

# Sources land only after the dependencies are cooked. Everything below this
# line rebuilds when content/ or crates/ change; everything above does not.
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
