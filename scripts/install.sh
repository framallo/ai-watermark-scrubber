#!/bin/sh
# One-line installer for the `scrub` binary (ai-watermark-scrubber).
#
#   curl -fsSL https://raw.githubusercontent.com/framallo/ai-watermark-scrubber/main/scripts/install.sh | sh
#
# Downloads a prebuilt binary from the latest GitHub Release for your OS/arch —
# no Rust toolchain required. Override the install dir with SCRUB_INSTALL_DIR,
# or the version with SCRUB_VERSION (e.g. v0.1.0).

set -eu

REPO="framallo/ai-watermark-scrubber"
BIN="scrub"
VERSION="${SCRUB_VERSION:-latest}"

err() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }
info() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }

# --- detect platform -------------------------------------------------------
os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Darwin) os_part="apple-darwin" ;;
  Linux)  os_part="unknown-linux-musl" ;;
  *) err "unsupported OS '$os'. Use: cargo install --git https://github.com/$REPO" ;;
esac

case "$arch" in
  x86_64|amd64)  arch_part="x86_64" ;;
  arm64|aarch64) arch_part="aarch64" ;;
  *) err "unsupported arch '$arch'. Use: cargo install --git https://github.com/$REPO" ;;
esac

target="${arch_part}-${os_part}"
asset="${BIN}-${target}.tar.gz"

if [ "$VERSION" = "latest" ]; then
  url="https://github.com/$REPO/releases/latest/download/$asset"
else
  url="https://github.com/$REPO/releases/download/$VERSION/$asset"
fi

# --- pick an install dir ---------------------------------------------------
if [ -n "${SCRUB_INSTALL_DIR:-}" ]; then
  dir="$SCRUB_INSTALL_DIR"
elif [ -w "/usr/local/bin" ] 2>/dev/null; then
  dir="/usr/local/bin"
else
  dir="$HOME/.local/bin"
fi
mkdir -p "$dir"

# --- download + extract ----------------------------------------------------
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

info "Downloading $asset ($VERSION)…"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$url" -o "$tmp/$asset" || err "download failed: $url"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$tmp/$asset" "$url" || err "download failed: $url"
else
  err "need curl or wget"
fi

tar -xzf "$tmp/$asset" -C "$tmp" || err "extract failed"
install -m 0755 "$tmp/$BIN" "$dir/$BIN" 2>/dev/null || { cp "$tmp/$BIN" "$dir/$BIN"; chmod 0755 "$dir/$BIN"; }

info "Installed $BIN -> $dir/$BIN"
"$dir/$BIN" --version || true

case ":$PATH:" in
  *":$dir:"*) : ;;
  *) printf '\033[1;33mnote:\033[0m %s is not on your PATH. Add:\n  export PATH="%s:$PATH"\n' "$dir" "$dir" ;;
esac
