#!/bin/bash
set -e

REPO="theodore-evans/astray"
INSTALL_DIR="/usr/local/bin"
BINARY_NAME="astray"

# Detect platform and architecture.
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
    Darwin)
        case "$ARCH" in
            arm64|aarch64) ARTIFACT="astray-macos-arm64" ;;
            x86_64)        ARTIFACT="astray-macos-x64" ;;
            *)             echo "Unsupported architecture: $ARCH"; exit 1 ;;
        esac
        ;;
    Linux)
        ARTIFACT="astray-linux-x64"
        ;;
    *)
        echo "Unsupported OS: $OS"; exit 1
        ;;
esac

echo "Downloading $ARTIFACT..."

# Get the latest release download URL.
DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/$ARTIFACT"

TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

HTTP_CODE=$(curl -sL -w "%{http_code}" -o "$TMPDIR/$ARTIFACT" "$DOWNLOAD_URL")

if [ "$HTTP_CODE" != "200" ]; then
    echo "Error: Download failed (HTTP $HTTP_CODE)."
    echo "No release found. Install from source instead:"
    echo "  cargo install --git https://github.com/$REPO"
    exit 1
fi

chmod +x "$TMPDIR/$ARTIFACT"

# Remove macOS quarantine.
if [ "$OS" = "Darwin" ]; then
    xattr -d com.apple.quarantine "$TMPDIR/$ARTIFACT" 2>/dev/null || true
fi

# Install.
if [ -w "$INSTALL_DIR" ]; then
    mv "$TMPDIR/$ARTIFACT" "$INSTALL_DIR/$BINARY_NAME"
else
    echo "Installing to $INSTALL_DIR (requires sudo)..."
    sudo mv "$TMPDIR/$ARTIFACT" "$INSTALL_DIR/$BINARY_NAME"
fi

echo "Installed $BINARY_NAME to $INSTALL_DIR/$BINARY_NAME"
echo ""
echo "Run with:"
echo "  astray --shape circle --longest"
