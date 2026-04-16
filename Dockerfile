# Base stage — cargo-chef pre-installed, pinned Rust
FROM lukemathwalker/cargo-chef:latest-rust-alpine AS chef
WORKDIR /esemese

# install the required system dependencies for our linking configuration 
# RUN apt update && apt install lld clang -y

# Stage 1: generate the recipe
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: build deps from recipe, then the app
FROM chef AS builder
RUN apk add --no-cache musl-dev
COPY --from=planner /esemese/recipe.json recipe.json
# This layer is cached unless recipe.json changes (= Cargo.toml/lock changed)
RUN cargo chef cook --release --recipe-path recipe.json
# Now bring in source and build just the binary
COPY . .
RUN cargo build --release --bin esemese-backend-server

# Stage 3: runtime
FROM scratch AS runtime
COPY --from=builder /esemese/target/release/esemese-backend-server /server
EXPOSE 8000
ENTRYPOINT ["/server"]
