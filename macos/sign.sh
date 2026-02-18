#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VERSION="${1:?Usage: $0 <version> <arch> [signing_identity]}"
ARCH="${2:?Usage: $0 <version> <arch> [signing_identity]}"
SIGNING_IDENTITY="${3:-${APPLE_SIGNING_IDENTITY:-}}"
APP="Shiori.app"
DMG="Shiori-${VERSION}-${ARCH}.dmg"

if [[ ! -d "$APP" ]]; then
    echo "Error: $APP not found. Run bundle.sh first."
    exit 1
fi

if [[ -z "$SIGNING_IDENTITY" ]]; then
    echo "Error: signing identity required (arg 3 or APPLE_SIGNING_IDENTITY env)"
    exit 1
fi

echo "Signing $APP..."
codesign --deep --force --verify --verbose \
    --sign "$SIGNING_IDENTITY" \
    --entitlements "$SCRIPT_DIR/entitlements.plist" \
    --options runtime \
    "$APP"

echo "Verifying signature..."
codesign --verify --verbose "$APP"

echo "Creating DMG..."
mkdir -p dmg_staging
mv "$APP" dmg_staging/
ln -s /Applications dmg_staging/Applications
hdiutil create \
    -volname "Shiori" \
    -srcfolder dmg_staging \
    -ov \
    -format UDZO \
    "$DMG"
mv dmg_staging/"$APP" .
rm -rf dmg_staging

echo "Signing DMG..."
codesign --sign "$SIGNING_IDENTITY" "$DMG"

APPLE_ID="${APPLE_ID:-}"
APPLE_PASSWORD="${APPLE_PASSWORD:-}"
APPLE_TEAM_ID="${APPLE_TEAM_ID:-}"

if [[ -n "$APPLE_ID" && -n "$APPLE_PASSWORD" && -n "$APPLE_TEAM_ID" ]]; then
    echo "Submitting for notarization..."
    xcrun notarytool submit "$DMG" \
        --apple-id "$APPLE_ID" \
        --password "$APPLE_PASSWORD" \
        --team-id "$APPLE_TEAM_ID" \
        --wait

    echo "Stapling notarization ticket..."
    xcrun stapler staple "$DMG"
else
    echo "Skipping notarization (APPLE_ID, APPLE_PASSWORD, or APPLE_TEAM_ID not set)"
fi

echo "Done: $DMG"
echo "  Size: $(du -sh "$DMG" | cut -f1)"
shasum -a 256 "$DMG"
