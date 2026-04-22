#!/bin/bash
# Builds and bundles codelord.app with all required configurations.
# Usage: ./tasks/bundle-codelord.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "Building and bundling codelord"
cargo bundle --release -p codelord

echo "Installing pdfium"
"$SCRIPT_DIR/download-pdfium.sh"

BUNDLE_PATH="$PROJECT_DIR/target/release/bundle/osx/codelord.app"

if [ ! -d "$BUNDLE_PATH" ]; then
  echo "Error: Bundle not created at $BUNDLE_PATH"
  exit 1
fi

echo "Patching Info.plist"
PLIST_PATH="$BUNDLE_PATH/Contents/Info.plist"

# Add microphone permission if not present.
# Use `plutil` so nested dicts (CFBundleURLTypes etc. if ever added)
# don't break the insertion point — sed would land on the first
# `</dict>`, which stops being the root dict once nesting appears.
if ! grep -q "NSMicrophoneUsageDescription" "$PLIST_PATH"; then
  plutil -insert NSMicrophoneUsageDescription \
    -string 'codelord needs microphone access for voice control commands. Press Cmd+Shift+Space to use voice commands.' \
    "$PLIST_PATH"
  echo "Added microphone permission"
else
  echo "Microphone permission already present"
fi

echo "Signing bundle (ad-hoc)"
codesign --force --deep --sign - "$BUNDLE_PATH"

echo "Done"
echo "Bundle ready at: $BUNDLE_PATH"
