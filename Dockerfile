# backend build 

FROM rust:1.93-bookworm AS backend-builder
WORKDIR /app

COPY ./Cargo.toml ./
COPY ./src/ ./src/

RUN cargo build --release

# migration build

FROM rust:1.93-bookworm AS migration-builder
WORKDIR /app

COPY ./migration/Cargo.toml ./
COPY ./migration/src/ ./src/

RUN cargo build --release

# running 
FROM gcr.io/distroless/cc-debian12
WORKDIR /app

COPY --from=backend-builder /app/target/release/backend .
COPY --from=migration-builder /app/target/release/migration .

# need set environment var: DATABASE_URL
