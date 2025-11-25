#!/bin/bash

# Build script pro Linux ARM64 na macOS ARM64
# Vyžaduje Docker

set -e

echo "🐳 Building Linux ARM64 version using Docker..."

# Vytvoř Dockerfile pro build
cat > Dockerfile.linux-arm64 << 'EOF'
FROM --platform=linux/arm64 ubuntu:24.04

# Nastavit non-interactive mode
ENV DEBIAN_FRONTEND=noninteractive

# Instalace závislostí
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf \
    libx11-dev \
    libxcb1-dev \
    libxcb-render0-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libpipewire-0.3-dev \
    libgbm-dev \
    libssl-dev \
    pkg-config \
    libclang-dev \
    tesseract-ocr \
    tesseract-ocr-eng \
    libtesseract-dev \
    libleptonica-dev \
    && rm -rf /var/lib/apt/lists/*

# Instalace Node.js 20
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs

# Instalace Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

# Kopíruj zdrojové kódy
COPY . .

# Build
WORKDIR /app/tracker-agent-app
RUN npm ci
RUN npm run tauri build

# Výsledné soubory budou v /app/tracker-agent-app/src-tauri/target/release/bundle/
EOF

# Build Docker image a zkompiluj aplikaci
docker build --platform linux/arm64 -f Dockerfile.linux-arm64 -t tracker-agent-builder .

# Vytvoř kontejner a zkopíruj výsledky
docker create --name tracker-agent-temp tracker-agent-builder
mkdir -p dist/linux-arm64
docker cp tracker-agent-temp:/app/tracker-agent-app/src-tauri/target/release/bundle/deb/. dist/linux-arm64/
docker cp tracker-agent-temp:/app/tracker-agent-app/src-tauri/target/release/bundle/appimage/. dist/linux-arm64/
docker rm tracker-agent-temp

# Cleanup
rm Dockerfile.linux-arm64

echo "✅ Build dokončen! Soubory jsou v dist/linux-arm64/"
ls -lh dist/linux-arm64/

