#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
VERSION="${1:-0.2.0}"
BINARY="${2:-$PROJECT_ROOT/target/release/shiori}"
APP="Shiori.app"

if [[ ! -f "$BINARY" ]]; then
    echo "Error: binary not found at $BINARY"
    echo "Usage: $0 [version] [binary_path]"
    exit 1
fi

echo "Bundling Shiori.app (v${VERSION})..."

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources"

cp "$BINARY" "$APP/Contents/MacOS/shiori"

sed "s/__VERSION__/${VERSION}/g" "$SCRIPT_DIR/Info.plist" > "$APP/Contents/Info.plist"

if [[ -f "$SCRIPT_DIR/shiori.icns" ]]; then
    cp "$SCRIPT_DIR/shiori.icns" "$APP/Contents/Resources/shiori.icns"
else
    echo "Warning: shiori.icns not found, skipping icon"
fi

if [[ -d "$PROJECT_ROOT/assets" ]]; then
    cp -R "$PROJECT_ROOT/assets" "$APP/Contents/Resources/assets"
else
    echo "Warning: assets directory not found"
fi

cat > "$APP/Contents/Resources/shiori-cli" << 'WRAPPER'
#!/bin/bash
exec "/Applications/Shiori.app/Contents/MacOS/shiori" "$@"
WRAPPER
chmod +x "$APP/Contents/Resources/shiori-cli"

echo "Created $APP (v${VERSION})"
echo "  Binary: $(file "$APP/Contents/MacOS/shiori" | cut -d: -f2-)"
echo "  Size:   $(du -sh "$APP" | cut -f1)"
