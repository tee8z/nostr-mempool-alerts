
#Dependency stage
FROM lukemathwalker/cargo-chef:latest-rust-1.63.0 as Chef

WORKDIR /app
RUN apt update && apt install lld clang -y

FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

#Builder stage
FROM chef as Builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release --bin nostr-vault

#Runtime stage
FROM debian:bullseye-slim AS Runtime

WORKDIR /app

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=Builder /app/target/release/nostr-vault nostr-vault
COPY configuration configuration
ENV APP_ENVIRONMENT production
ENTRYPOINT [ "./nostr-mempool-alerts" ]