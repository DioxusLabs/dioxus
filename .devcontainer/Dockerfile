ARG VARIANT="nightly-bookworm-slim"
FROM rustlang/rust:${VARIANT}

ENV DEBIAN_FRONTEND=noninteractive

# Install required system libraries and NPM
# Reference: https://dioxuslabs.com/learn/0.7/beyond/contributing#before-you-contribute
RUN apt-get update -qq \
    && apt-get install -y -qq \
    libgdk3.0-cil \
    libatk1.0-dev \
    libcairo2-dev \
    libpango1.0-dev \
    libgdk-pixbuf2.0-dev \
    libsoup-3.0-dev \
    libjavascriptcoregtk-4.1-dev \
    libwebkit2gtk-4.1-dev \
    npm \
    && rm -rf /var/lib/apt/lists/*

# Set a shared folder for pre-installed browsers
ENV PLAYWRIGHT_BROWSERS_PATH=/ms-playwright

# Temporarily install Playwright globally to install the browsers and their dependencies
RUN npm install -g @playwright/test && \
    npx playwright install --with-deps && \
    npm uninstall -g @playwright/test
