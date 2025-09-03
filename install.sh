#!/usr/bin/env bash
set -euo pipefail

REPO="DillonKyle/lazylogger"
BIN_NAME="lazylogger"
INSTALL_DIR="$HOME/.local/bin"
OS="$(uname -s)"
ARCH="$(uname -m)"
VERSION=$(curl -s https://api.github.com/repos/$REPO/releases/latest |
  grep -Po '"tag_name": "\K.*?(?=")')

case "$OS" in
Linux*) PLATFORM="unknown-linux-musl" ;;
Darwin*) PLATFORM="apple-darwin" ;;
*)
  echo "Unsupported OS: $OS"
  exit 1
  ;;
esac

case "$ARCH" in
x86_64) ARCH="x86_64" ;;
*)
  echo "Unsupported architecture: $ARCH"
  exit 1
  ;;
esac

URL="https://github.com/$REPO/releases/latest/download/${BIN_NAME}-${VERSION}-${ARCH}-${PLATFORM}.tar.gz"

echo "Downloading $URL..."
mkdir -p "$INSTALL_DIR"
curl -L "$URL" | tar -xz --strip-components=1 -C "$INSTALL_DIR"
chmod +x "$INSTALL_DIR/$BIN_NAME"

if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
  echo "Add this to your shell rc file (e.g. ~/.bashrc or ~/.zshrc):"
  echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
fi

echo "✅ Installed $BIN_NAME to $INSTALL_DIR"
echo "Run with: $BIN_NAME"
