#!/bin/sh
# Ad-hoc signatures pin macOS permissions (mic, accessibility) to the exact binary hash, so every
# rebuild loses them. Re-sign with an identifier-based requirement so grants survive rebuilds.
# Usage: resign.sh [path-to-app]   (default: native release build)
APP="${1:-src-tauri/target/release/bundle/macos/Kōe.app}"
codesign --force --deep -s - -o runtime \
  --entitlements src-tauri/Entitlements.plist \
  -r='designated => identifier "app.koe.desktop"' "$APP"
