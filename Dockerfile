FROM rust:1.59.0
WORKDIR /app
# Install the required system dependencies for our linking configuration
RUN apt update && apt install lld clang -y
COPY . .
# To enable sqlx to check query validity at build time against the sqlx-data.json
ENV SQLX_OFFLINE true
RUN cargo build --release
# To enable host port 0.0.0.0
ENV APP_ENVIRONMENT production
# Binary to run
ENTRYPOINT ["./target/release/zero2prod"]
