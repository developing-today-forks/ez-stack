#!/usr/bin/env bash
set -euo pipefail

# ez-stack installer
# Usage: curl -fsSL https://raw.githubusercontent.com/rohoswagger/ez-stack/main/install.sh | bash
# Or:    curl -fsSL ... | bash -s -- v0.2.0   (specific version)

REPO="rohoswagger/ez-stack"
BINARY="ez"
INSTALL_DIR="${EZ_INSTALL_DIR:-$HOME/.local/bin}"

info() { printf "\033[1;34m::\033[0m %s\n" "$1"; }
success() { printf "\033[1;32m✓\033[0m %s\n" "$1"; }
error() { printf "\033[1;31m✗\033[0m %s\n" "$1" >&2; exit 1; }

# Determine version
VERSION="${1:-latest}"
if [ "$VERSION" = "latest" ]; then
    info "Fetching latest release..."
    VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
    if [ -z "$VERSION" ]; then
        error "Could not determine latest version. Check https://github.com/${REPO}/releases"
    fi
fi
info "Installing ez ${VERSION}"

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
    linux)  TARGET_OS="unknown-linux-gnu" ;;
    darwin) TARGET_OS="apple-darwin" ;;
    *)      error "Unsupported OS: $OS" ;;
esac

case "$ARCH" in
    x86_64|amd64)  TARGET_ARCH="x86_64" ;;
    aarch64|arm64) TARGET_ARCH="aarch64" ;;
    *)             error "Unsupported architecture: $ARCH" ;;
esac

TARGET="${TARGET_ARCH}-${TARGET_OS}"
ARCHIVE="ez-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"

# Download and extract
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

info "Downloading ${URL}..."
if ! curl -fsSL "$URL" -o "${TMPDIR}/${ARCHIVE}"; then
    error "Download failed. Check that ${VERSION} exists at https://github.com/${REPO}/releases"
fi

info "Extracting..."
tar xzf "${TMPDIR}/${ARCHIVE}" -C "$TMPDIR"

# Install
mkdir -p "$INSTALL_DIR"
mv "${TMPDIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
chmod +x "${INSTALL_DIR}/${BINARY}"

success "Installed ez to ${INSTALL_DIR}/${BINARY}"

# Check PATH
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    echo ""
    info "Add ${INSTALL_DIR} to your PATH:"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    echo ""
    echo "  Add this to your ~/.bashrc or ~/.zshrc to make it permanent."
fi

echo ""
success "Run 'ez --help' to get started"
