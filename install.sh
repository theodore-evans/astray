#!/bin/bash
set -e

REPO="theodore-evans/astray"
INSTALL_DIR="/usr/local/bin"
BINARY_NAME="astray"

# Detect architecture.
ARCH=$(uname -m)
case "$ARCH" in
    arm64|aarch64) ARTIFACT="astray-macos-arm64" ;;
    x86_64)        ARTIFACT="astray-macos-x64" ;;
    *)             echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

OS=$(uname -s)
if [ "$OS" != "Darwin" ]; then
    ARTIFACT="astray-linux-x64"
fi

echo "Downloading $ARTIFACT..."

# Get the latest successful workflow run.
RUN_URL=$(curl -sL "https://api.github.com/repos/$REPO/actions/runs?status=success&per_page=1" \
    | grep -o '"artifacts_url":"[^"]*"' | head -1 | cut -d'"' -f4)

if [ -z "$RUN_URL" ]; then
    echo "Error: Could not find a successful build."
    echo "Install from source instead: cargo install --git https://github.com/$REPO"
    exit 1
fi

# Find the matching artifact download URL.
DOWNLOAD_URL=$(curl -sL "$RUN_URL" \
    | grep -B2 "\"$ARTIFACT\"" | grep -o '"archive_download_url":"[^"]*"' | cut -d'"' -f4)

if [ -z "$DOWNLOAD_URL" ]; then
    echo "Error: Could not find artifact $ARTIFACT."
    echo "Install from source instead: cargo install --git https://github.com/$REPO"
    exit 1
fi

# GitHub artifact downloads require authentication.
# Try gh token first, then prompt.
TOKEN=""
if command -v gh &>/dev/null; then
    TOKEN=$(gh auth token 2>/dev/null || true)
fi

if [ -z "$TOKEN" ]; then
    echo ""
    echo "GitHub requires authentication to download workflow artifacts."
    echo "Either install 'gh' (https://cli.github.com) and run 'gh auth login',"
    echo "or paste a personal access token with 'actions' scope:"
    read -rsp "Token (input hidden): " TOKEN
    echo ""
fi

if [ -z "$TOKEN" ]; then
    echo "No token provided. Install from source instead:"
    echo "  cargo install --git https://github.com/$REPO"
    exit 1
fi

TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

curl -sL -H "Authorization: token $TOKEN" "$DOWNLOAD_URL" -o "$TMPDIR/artifact.zip"
unzip -qo "$TMPDIR/artifact.zip" -d "$TMPDIR"

BINARY=$(find "$TMPDIR" -type f -name "astray-*" | head -1)
if [ -z "$BINARY" ]; then
    echo "Error: Binary not found in artifact."
    exit 1
fi

chmod +x "$BINARY"

# Remove macOS quarantine.
if [ "$OS" = "Darwin" ]; then
    xattr -d com.apple.quarantine "$BINARY" 2>/dev/null || true
fi

# Install.
if [ -w "$INSTALL_DIR" ]; then
    mv "$BINARY" "$INSTALL_DIR/$BINARY_NAME"
else
    echo "Installing to $INSTALL_DIR (requires sudo)..."
    sudo mv "$BINARY" "$INSTALL_DIR/$BINARY_NAME"
fi

echo "Installed $BINARY_NAME to $INSTALL_DIR/$BINARY_NAME"
echo ""
echo "Run with:"
echo "  astray --shape circle --longest"
