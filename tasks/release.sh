#!/bin/bash
# Builds codelord, packages it, and creates a GitHub Release.
# Usage: ./tasks/release.sh <version>
# Example: ./tasks/release.sh 0.0.0
#
# Prerequisites:
#   - gh CLI (brew install gh)
#   - cargo-bundle (cargo install cargo-bundle)

set -e

VERSION="${1:?Usage: ./tasks/release.sh <version>}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DIST_DIR="$PROJECT_DIR/apps/coder/codelord-release"

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# --- Build ---

echo "=== Building codelord $VERSION ==="

"$SCRIPT_DIR/bundle-codelord.sh"

# --- Package ---

echo "=== Packaging ==="

BUNDLE_PATH="$PROJECT_DIR/target/release/bundle/osx/codelord.app"
ARTIFACT="codelord-macos-$(uname -m)-$VERSION.tar.gz"

if [ ! -d "$BUNDLE_PATH" ]; then
  echo "Error: Bundle not found at $BUNDLE_PATH"
  exit 1
fi

cd "$PROJECT_DIR/target/release/bundle/osx"
tar czvf "$DIST_DIR/$ARTIFACT" codelord.app
cd "$PROJECT_DIR"

(cd "$DIST_DIR" && shasum -a 256 "$ARTIFACT" > "$ARTIFACT.sha256")

echo "Packaged: apps/coder/codelord-release/$ARTIFACT"
echo "Checksum: apps/coder/codelord-release/$ARTIFACT.sha256"

# --- Tag and Release ---

echo "=== Creating GitHub Release ==="

if git rev-parse "$VERSION" >/dev/null 2>&1; then
  echo "Tag $VERSION already exists, skipping tag creation"
else
  git tag -a "$VERSION" -m "codelord $VERSION"
  git push origin "$VERSION"
  echo "Tag $VERSION pushed"
fi

gh release create "$VERSION" \
  "$DIST_DIR/$ARTIFACT" \
  "$DIST_DIR/$ARTIFACT.sha256" \
  --title "codelord $VERSION" \
  --generate-notes

echo "=== Done ==="
echo "Release: https://github.com/compilords/codelord/releases/tag/$VERSION"
