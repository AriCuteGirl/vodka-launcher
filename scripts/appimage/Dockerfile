# Build environment for the Linux AppImage. Ubuntu 24.04 has glibc 2.39, so the AppImage runs on any system with
# glibc 2.39 or newer instead of only on the newest distros.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential curl ca-certificates git file pkg-config xz-utils \
      libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev \
      librsvg2-dev libayatana-appindicator3-dev libssl-dev libdbus-1-dev libsecret-1-dev \
      patchelf desktop-file-utils squashfs-tools \
    && rm -rf /var/lib/apt/lists/*

# Rust and Node, at the versions the project builds with.
ARG RUST_VERSION=1.99.0
ARG NODE_VERSION=22.23.5
ARG PNPM_VERSION=12.21.0
ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo PATH=/opt/cargo/bin:/opt/node/bin:$PATH
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain "$RUST_VERSION" \
    && mkdir -p /opt/node \
    && curl -fsSL "https://nodejs.org/dist/v$NODE_VERSION/node-v$NODE_VERSION-linux-x64.tar.xz" \
       | tar -xJ -C /opt/node --strip-components=1 \
    && npm install -g "pnpm@$PNPM_VERSION"

WORKDIR /src
