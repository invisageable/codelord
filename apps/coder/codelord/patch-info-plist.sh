#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
BUNDLE_PATH="$WORKSPACE_ROOT/target/release/bundle/osx/codelord.app/Contents/Info.plist"

if [ ! -f "$BUNDLE_PATH" ]; then
  echo "Bundle not found at $BUNDLE_PATH"
  exit 1
fi

if grep -q "NSMicrophoneUsageDescription" "$BUNDLE_PATH"; then
  echo "Microphone permission already exists"
  exit 0
fi

sed -i '' 's|</dict>|  <key>NSMicrophoneUsageDescription</key>\
  <string>codelord needs microphone access for voice control commands. Press Cmd+Shift+Space to use voice commands.</string>\
</dict>|' "$BUNDLE_PATH"

echo "Added microphone permission to Info.plist"
