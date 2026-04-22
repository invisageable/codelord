#!/bin/sh
# codelord installer — downloads the latest macOS release from GitHub
# and installs codelord.app into /Applications.
#
# Usage:
#   curl -fsSL https://codelord.sh/install.sh | sh
#
# Or pin a specific version / install dir:
#   curl -fsSL https://codelord.sh/install.sh | CODELORD_VERSION=0.0.2 sh
#   curl -fsSL https://codelord.sh/install.sh | CODELORD_PREFIX="$HOME/Applications" sh
#
# Release artifacts produced by `tasks/release.sh` are named
# `codelord-macos-<arch>-<version>.tar.gz` and contain `codelord.app`.

set -eu

REPO="compilords/codelord"
APP_NAME="codelord.app"
PREFIX="${CODELORD_PREFIX:-/Applications}"
VERSION="${CODELORD_VERSION:-latest}"

say() { printf "\033[1m==>\033[0m %s\n" "$*"; }
err() { printf "\033[1;31merror:\033[0m %s\n" "$*" >&2; exit 1; }

# --- OS / arch detection ---

OS=$(uname -s)
ARCH=$(uname -m)

[ "$OS" = "Darwin" ] || err "codelord currently only ships for macOS (got $OS)."

case "$ARCH" in
  arm64|aarch64) ARCH="arm64" ;;
  x86_64|amd64)  ARCH="x86_64" ;;
  *)             err "unsupported architecture: $ARCH" ;;
esac

# --- Resolve release + asset ---

if [ "$VERSION" = "latest" ]; then
  API_URL="https://api.github.com/repos/$REPO/releases/latest"
else
  API_URL="https://api.github.com/repos/$REPO/releases/tags/$VERSION"
fi

say "Fetching release metadata ($VERSION)"
RELEASE_JSON=$(curl -fsSL "$API_URL") || err "failed to fetch $API_URL"

ASSET_URL=$(
  printf "%s" "$RELEASE_JSON" \
    | grep -o '"browser_download_url": *"[^"]*"' \
    | sed 's/^"browser_download_url": *"\(.*\)"$/\1/' \
    | grep "codelord-macos-${ARCH}-" \
    | head -n 1
)

[ -n "$ASSET_URL" ] || err "no macOS-${ARCH} asset on release $VERSION."

# --- Download + extract ---

TMP=$(mktemp -d -t codelord-install)
trap 'rm -rf "$TMP"' EXIT INT TERM

TARBALL="$TMP/$(basename "$ASSET_URL")"
say "Downloading $(basename "$ASSET_URL")"
curl -fSL --progress-bar "$ASSET_URL" -o "$TARBALL"
curl -fsSL "$ASSET_URL.sha256" -o "$TARBALL.sha256" \
  || err "checksum file missing for this release."

say "Verifying SHA-256"
EXPECTED=$(awk '{print $1}' "$TARBALL.sha256")
ACTUAL=$(shasum -a 256 "$TARBALL" | awk '{print $1}')
[ "$EXPECTED" = "$ACTUAL" ] \
  || err "SHA-256 mismatch (expected $EXPECTED, got $ACTUAL)."

say "Extracting"
tar -xzf "$TARBALL" -C "$TMP"
[ -d "$TMP/$APP_NAME" ] || err "archive did not contain $APP_NAME"

# --- Install ---

DEST="$PREFIX/$APP_NAME"

if [ -d "$PREFIX" ] && [ ! -w "$PREFIX" ]; then
  SUDO="sudo"
  say "Install prefix $PREFIX needs sudo"
else
  SUDO=""
fi

mkdir -p "$PREFIX" 2>/dev/null || $SUDO mkdir -p "$PREFIX"

if [ -d "$DEST" ]; then
  say "Removing existing $DEST"
  $SUDO rm -rf "$DEST"
fi

say "Installing to $DEST"
$SUDO mv "$TMP/$APP_NAME" "$DEST"

# The bundle isn't notarized yet, so macOS Gatekeeper would refuse to
# open it. Clearing the quarantine xattr sidesteps that for users who
# curl-installed deliberately.
$SUDO xattr -dr com.apple.quarantine "$DEST" 2>/dev/null || true

say "Installed codelord at $DEST"
say "Launch with: open \"$DEST\""
