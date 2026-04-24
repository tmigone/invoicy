#!/bin/sh
set -e

REPO="tmigone/invoicy"
INSTALL_DIR="/usr/local/bin"

# Detect OS
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$OS" in
    darwin) OS="macos" ;;
    linux) OS="linux" ;;
    mingw*|msys*|cygwin*) OS="windows" ;;
    *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64|amd64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="arm64" ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Windows only supports x86_64
if [ "$OS" = "windows" ]; then
    ARCH="x86_64"
fi

# Linux only supports x86_64 for now
if [ "$OS" = "linux" ] && [ "$ARCH" = "arm64" ]; then
    echo "Linux arm64 is not currently supported"
    exit 1
fi

# Build binary name
if [ "$OS" = "windows" ]; then
    BINARY="invoicy-windows-x86_64.exe"
else
    BINARY="invoicy-${OS}-${ARCH}"
fi

# Get latest release tag
echo "Fetching latest release..."
LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)

if [ -z "$LATEST" ]; then
    echo "Failed to fetch latest release"
    exit 1
fi

echo "Latest version: $LATEST"

# Download URL
URL="https://github.com/${REPO}/releases/download/${LATEST}/${BINARY}"

# Download binary
echo "Downloading $BINARY..."
TMPFILE=$(mktemp)
curl -fsSL "$URL" -o "$TMPFILE"

# Install
if [ "$OS" = "windows" ]; then
    echo "Windows detected. Binary downloaded to: $TMPFILE"
    echo "Please move it to your PATH manually."
else
    chmod +x "$TMPFILE"

    if [ -w "$INSTALL_DIR" ]; then
        mv "$TMPFILE" "$INSTALL_DIR/invoicy"
    else
        echo "Installing to $INSTALL_DIR (requires sudo)..."
        sudo mv "$TMPFILE" "$INSTALL_DIR/invoicy"
    fi

    echo "Installed invoicy to $INSTALL_DIR/invoicy"
fi

echo "Done! Run 'invoicy --help' to get started."
