ARG NODE_VARIANT="24-bookworm-slim"
ARG RUST_VARIANT="1.94.0-bookworm"

FROM docker.io/library/node:${NODE_VARIANT} AS node
FROM docker.io/library/rust:${RUST_VARIANT}

ENV DEBIAN_FRONTEND=noninteractive \
    PLAYWRIGHT_BROWSERS_PATH=/ms-playwright

# The Rust image does not include Node.js. Copy the same Node.js major version used
# by CI instead of installing Debian's older npm/Node.js 18 packages.
COPY --from=node /usr/local/ /usr/local/

# Linux development, desktop, and packaging dependencies. Keep the desktop list
# aligned with the Linux jobs in .github/workflows/main.yml.
RUN apt-get update -qq \
    && apt-get install -y -qq --no-install-recommends \
        ca-certificates \
        curl \
        desktop-file-utils \
        git \
        libasound2-dev \
        libayatana-appindicator3-dev \
        libglib2.0-dev \
        libgtk-3-dev \
        librsvg2-dev \
        libudev-dev \
        libwebkit2gtk-4.1-dev \
        libxdo-dev \
        patchelf \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Stable is used for normal development and CI. The pinned nightly is needed for
# documentation work and the CLI's optional unit-graph support.
RUN rustup component add clippy rustfmt rust-src \
    && rustup target add wasm32-unknown-unknown \
    && rustup toolchain install nightly-2026-05-01 \
        --profile minimal \
        --component rustfmt,rust-src

# Install exactly the browser revision selected by the repository lockfile. Only
# Chromium is needed because it is the sole project in playwright.config.js.
COPY packages/playwright-tests/package.json \
     packages/playwright-tests/package-lock.json \
     /tmp/dioxus-playwright/
RUN cd /tmp/dioxus-playwright \
    && npm ci \
    && npx playwright install --with-deps chromium \
    && chmod -R a+rX "${PLAYWRIGHT_BROWSERS_PATH}" \
    && rm -rf /tmp/dioxus-playwright /var/lib/apt/lists/*
