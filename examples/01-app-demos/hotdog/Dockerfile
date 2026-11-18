FROM rust:1 AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/DioxusLabs/dioxus/refs/heads/main/.github/install.sh | bash
RUN rustup target add wasm32-unknown-unknown
COPY --from=planner /app/recipe.json recipe.json
COPY Dioxus.toml .
RUN cargo chef cook --no-build --recipe-path recipe.json \
    && client="$(dx print client-args --platform web --release --features production)" \
    && eval "$client" \
    && server="$(dx print server-args --platform web --release --features production)" \
    && eval "$server"
COPY . .
RUN dx bundle --platform web --release --features production

FROM chef AS runtime
COPY --from=builder /app/target/dx/hotdog/release/web/ /usr/local/app

ENV PORT=8080
ENV IP=0.0.0.0
EXPOSE 8080

WORKDIR /usr/local/app
ENTRYPOINT [ "/usr/local/app/server" ]
