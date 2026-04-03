#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# RELEASE_PULSEBAR.sh — Repeatable release, sign, notarize, and verify script
# for RISE PulseBar (macOS Tauri app).
#
# Usage:
#   1. Copy RELEASE_ENV_TEMPLATE.sh → RELEASE_ENV_LOCAL.sh
#   2. Fill in your Apple credentials in RELEASE_ENV_LOCAL.sh
#   3. source ./RELEASE_ENV_LOCAL.sh
#   4. ./RELEASE_PULSEBAR.sh
#
# Requirements:
#   - Rust toolchain (cargo), Tauri CLI (cargo-tauri)
#   - Xcode Command Line Tools
#   - Apple Developer ID Application certificate in Keychain
#   - Environment variables: APPLE_ID, TEAM_ID, APP_PASSWORD
# ═══════════════════════════════════════════════════════════════════════════════
set -euo pipefail

# ─── Colors ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
fail()  { echo -e "${RED}[FAIL]${NC}  $*"; exit 1; }

# ─── Step 0: Verify we are in the correct repo ───────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

[[ -f "src-tauri/tauri.conf.json" ]] || fail "Not in a Tauri project root. Run from the Rise PulseBar directory."

PRODUCT_NAME=$(grep '"productName"' src-tauri/tauri.conf.json | head -1 | sed 's/.*: *"\(.*\)".*/\1/')
[[ "$PRODUCT_NAME" == "Rise PulseBar" ]] || fail "Wrong project: productName is '$PRODUCT_NAME', expected 'Rise PulseBar'"
info "Project confirmed: $PRODUCT_NAME"

VERSION=$(grep '"version"' src-tauri/tauri.conf.json | head -1 | sed 's/.*: *"\(.*\)".*/\1/')
info "Version: $VERSION"

# ─── Step 1: Check required environment variables ────────────────────────────
[[ -n "${APPLE_ID:-}"      ]] || fail "APPLE_ID not set. Source your RELEASE_ENV_LOCAL.sh first."
[[ -n "${TEAM_ID:-}"       ]] || fail "TEAM_ID not set. Source your RELEASE_ENV_LOCAL.sh first."
[[ -n "${APP_PASSWORD:-}"  ]] || fail "APP_PASSWORD not set. Source your RELEASE_ENV_LOCAL.sh first."
info "Credentials loaded (APPLE_ID=$APPLE_ID, TEAM_ID=$TEAM_ID)"

# ─── Step 2: Check required tools ────────────────────────────────────────────
command -v cargo          >/dev/null 2>&1 || fail "cargo not found. Install Rust."
command -v cargo-tauri    >/dev/null 2>&1 || { cargo tauri --version >/dev/null 2>&1 || fail "Tauri CLI not found. Run: cargo install tauri-cli"; }
command -v xcrun          >/dev/null 2>&1 || fail "xcrun not found. Install Xcode Command Line Tools."
info "Tools verified: cargo, tauri-cli, xcrun"

# ─── Step 3: Check signing identity ──────────────────────────────────────────
SIGNING_ID=$(security find-identity -v -p codesigning 2>/dev/null | grep "Developer ID Application" | head -1 | sed 's/.*"\(.*\)"/\1/' || true)
if [[ -z "$SIGNING_ID" ]]; then
    fail "No 'Developer ID Application' certificate found in Keychain. Install your Apple signing certificate first."
fi
info "Signing identity: $SIGNING_ID"

# Update tauri.conf.json with the real signing identity if it's null
CURRENT_SIG=$(grep '"signingIdentity"' src-tauri/tauri.conf.json | head -1)
if echo "$CURRENT_SIG" | grep -q 'null'; then
    warn "signingIdentity is null in tauri.conf.json. Updating to: $SIGNING_ID"
    # Use a temp file for safe replacement
    sed "s/\"signingIdentity\": null/\"signingIdentity\": \"$SIGNING_ID\"/" \
        src-tauri/tauri.conf.json > src-tauri/tauri.conf.json.tmp \
        && mv src-tauri/tauri.conf.json.tmp src-tauri/tauri.conf.json
    info "tauri.conf.json updated with signing identity."
fi

# ─── Step 4: Build in release mode ───────────────────────────────────────────
info "Building release..."
cargo tauri build 2>&1 | tail -10
info "Build complete."

# ─── Step 5: Locate artifacts ────────────────────────────────────────────────
# The .app and .dmg may have version and arch in their names
APP_PATH=$(find src-tauri/target/release/bundle/macos -name "*.app" -maxdepth 1 | head -1)
DMG_PATH=$(find src-tauri/target/release/bundle/dmg -name "*.dmg" -maxdepth 1 | head -1)

[[ -n "$APP_PATH" ]] || fail "Could not find .app bundle in src-tauri/target/release/bundle/macos/"
[[ -n "$DMG_PATH" ]] || fail "Could not find .dmg in src-tauri/target/release/bundle/dmg/"

info "App bundle: $APP_PATH"
info "DMG:        $DMG_PATH"

# ─── Step 6: Submit to Apple notarization ─────────────────────────────────────
info "Submitting DMG to Apple notarization (this may take 2–10 minutes)..."
xcrun notarytool submit "$DMG_PATH" \
    --apple-id "$APPLE_ID" \
    --team-id "$TEAM_ID" \
    --password "$APP_PASSWORD" \
    --wait

NOTARY_EXIT=$?
if [[ $NOTARY_EXIT -ne 0 ]]; then
    fail "Notarization failed (exit $NOTARY_EXIT). Check the output above for details."
fi
info "Notarization succeeded."

# ─── Step 7: Staple the notarization ticket ──────────────────────────────────
info "Stapling .app..."
xcrun stapler staple "$APP_PATH"

info "Stapling .dmg..."
xcrun stapler staple "$DMG_PATH"

info "Stapling complete."

# ─── Step 8: Verify with spctl ───────────────────────────────────────────────
info "Verifying app with spctl..."
SPCTL_OUT=$(spctl --assess --verbose "$APP_PATH" 2>&1 || true)
echo "  $SPCTL_OUT"

if echo "$SPCTL_OUT" | grep -qi "accepted"; then
    info "spctl verification PASSED."
else
    warn "spctl verification did not show 'accepted'. Review output above."
fi

# ─── Summary ──────────────────────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════════════════"
echo -e " ${GREEN}RISE PulseBar v${VERSION} — Release Complete${NC}"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "  App:  $APP_PATH"
echo "  DMG:  $DMG_PATH"
echo ""
echo "  Next steps:"
echo "    1. Upload DMG to GitHub release (gh release create v${VERSION} \"$DMG_PATH\")"
echo "    2. Upload DMG to Gumroad product listing"
echo "    3. Verify install on a clean Mac (or a test user account)"
echo ""
