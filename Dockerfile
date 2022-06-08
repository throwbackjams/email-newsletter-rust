FROM lukemathwalker/cargo-chef:latest-rust-1.59.0 as chef
WORKDIR /app
RUN apt update && apt install lld clang -y

FROM chef as planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, to be cached via Docker layer for faster builds
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
# To enable sqlx to check query validity at build time against the sqlx-data.json
ENV SQLX_OFFLINE true
RUN cargo build --release --bin zero2prod

# Runtime stage - use bare operating system as base image (without rustc, cargo, etc)
FROM debian:bullseye-slim AS runtime

WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
# Copy the compiled binary from the builder environment
COPY --from=builder /app/target/release/zero2prod zero2prod
# Copy the configuration file for the runtime
COPY configuration configuration
# Set APP_ENVIRONMENT to enable host port 0.0.0.0
ENV APP_ENVIRONMENT production
# Binary to run
ENTRYPOINT ["./zero2prod"]
