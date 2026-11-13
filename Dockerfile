# ─── Étape dev : hot-reload avec cargo-watch ─────────────────────────────────
FROM rust:1-slim-bookworm AS dev
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev libpq-dev \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install cargo-watch --locked \
    && cargo install diesel_cli --no-default-features --features postgres --locked
WORKDIR /app

# ─── Étape 1 : Build de l'application ────────────────────────────────────────
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

# Mise en cache des dépendances : on compile d'abord avec des sources factices
COPY Cargo.toml Cargo.lock* ./
COPY migrations ./migrations
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs \
    && cargo build --release && rm -rf src

# Compilation réelle
COPY build.rs ./
COPY src ./src
RUN touch src/main.rs src/lib.rs && cargo build --release

# ─── Étape 2 : Runtime d'exécution minimal ───────────────────────────────────
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Utilisateur non-root pour la sécurité
RUN groupadd -r tsg && useradd -r -g tsg tsg
USER tsg:tsg

COPY --from=builder /app/target/release/server /app/server

EXPOSE 8080
ENTRYPOINT ["/app/server"]
