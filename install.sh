#!/usr/bin/env bash
set -euo pipefail

REPO="nvquanghuy/localwebs"
INSTALL_DIR="$HOME/.local/bin"

# Detect OS
case "$(uname -s)" in
  Linux*)  OS="linux" ;;
  Darwin*) OS="darwin" ;;
  *)
    echo "Error: unsupported operating system: $(uname -s)" >&2
    exit 1
    ;;
esac

# Detect architecture
case "$(uname -m)" in
  x86_64)       ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *)
    echo "Error: unsupported architecture: $(uname -m)" >&2
    exit 1
    ;;
esac

TARBALL="localwebs-${ARCH}-unknown-${OS}-musl.tar.gz"
URL="https://github.com/${REPO}/releases/latest/download/${TARBALL}"

echo "📦 Downloading LocalWebs for ${OS}/${ARCH}..."

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

if ! curl -fSL --progress-bar -o "${TMPDIR}/${TARBALL}" "$URL"; then
  echo "Error: failed to download ${URL}" >&2
  echo "Make sure a release exists at https://github.com/${REPO}/releases/latest" >&2
  exit 1
fi

echo "📂 Installing to ${INSTALL_DIR}..."
mkdir -p "$INSTALL_DIR"
tar xzf "${TMPDIR}/${TARBALL}" -C "$INSTALL_DIR"
chmod +x "${INSTALL_DIR}/localwebs"

echo ""
echo "✅ LocalWebs installed successfully!"
echo ""
echo "📍 Location: ${INSTALL_DIR}/localwebs"
echo "🚀 Run: localwebs"
echo "🌐 Open: http://localhost:4444"
echo ""

if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
  echo "⚠️  Add ~/.local/bin to your PATH if it's not already there:"
  echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
  echo ""
fi

echo "Need help? https://github.com/${REPO}#readme"
