#!/bin/sh
set -e

BIN_NAME="{{bin}}"
VERSION="{{version}}"
REPO="{{repository}}"
INSTALL_PATH="{{path}}"

echo "Installing $BIN_NAME v$VERSION..."

# Expand tilde if present
_PATH="${INSTALL_PATH/#\~/$HOME}"
mkdir -p "$_PATH"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

TARGET=""
if [ "$OS" = "linux" ]; then
    [IF target.x86_64-unknown-linux-gnu]
    if [ "$ARCH" = "x86_64" ]; then TARGET="x86_64-unknown-linux-gnu"; fi
    [/IF]
    [IF target.aarch64-unknown-linux-gnu]
    if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then TARGET="aarch64-unknown-linux-gnu"; fi
    [/IF]
    [IF target.i686-unknown-linux-gnu]
    if [ "$ARCH" = "i686" ]; then TARGET="i686-unknown-linux-gnu"; fi
    [/IF]
    [IF target.s390x-unknown-linux-gnu]
    if [ "$ARCH" = "s390x" ]; then TARGET="s390x-unknown-linux-gnu"; fi
    [/IF]
elif [ "$OS" = "darwin" ]; then
    [IF target.x86_64-apple-darwin]
    if [ "$ARCH" = "x86_64" ]; then TARGET="x86_64-apple-darwin"; fi
    [/IF]
    [IF target.aarch64-apple-darwin]
    if [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then TARGET="aarch64-apple-darwin"; fi
    [/IF]
else
    echo "Unsupported OS: $OS"
    exit 1
fi

if [ -z "$TARGET" ]; then
    echo "Unsupported architecture: $ARCH"
    exit 1
fi

ASSET="${BIN_NAME}-${VERSION}-${TARGET}.tar.gz"
DOWNLOAD_URL="${REPO}/releases/download/v${VERSION}/${ASSET}"
TMP_DIR=$(mktemp -d)

echo "Downloading $DOWNLOAD_URL..."
curl -sL -f -o "$TMP_DIR/$ASSET" "$DOWNLOAD_URL"

echo "Extracting..."
tar -xzf "$TMP_DIR/$ASSET" -C "$TMP_DIR"

EXTRACTED_DIR="$TMP_DIR/${BIN_NAME}-${VERSION}-${TARGET}"
if [ -d "$EXTRACTED_DIR" ]; then
    cp "$EXTRACTED_DIR/$BIN_NAME" "$_PATH/"
else
    cp "$TMP_DIR/$BIN_NAME" "$_PATH/"
fi

chmod +x "$_PATH/$BIN_NAME"
rm -rf "$TMP_DIR"

echo "$BIN_NAME installed successfully to $_PATH/"
echo "Please make sure $_PATH is in your PATH."
