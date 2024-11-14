# We use the latest Rust stable release as base image
FROM lukemathwalker/cargo-chef:latest-rust-1.82.0 AS chef
WORKDIR /app
FROM chef AS planner 
COPY . .
RUN cargo chef prepare --recipe-path recipe.json
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json 


WORKDIR /app
# Install the required system dependencies for our linking configuration
RUN apt update && apt install lld clang -y
# Copy all files from our working environment to our Docker image
COPY . .
# Let's build our binary!
# We'll use the release profile to make it faaaast
ENV SQLX_OFFLINE=true 
RUN cargo build --release

# When `docker run` is executed, launch the binary!
FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/zero-to-prod .
COPY configuration configuration  
ENV APP_ENVIRONMENT=configuration/production
ENTRYPOINT ["/app/zero-to-prod"]