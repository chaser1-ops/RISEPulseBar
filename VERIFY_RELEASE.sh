#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# VERIFY_RELEASE.sh — Post-release checks for RISE PulseBar
#
# Run after RELEASE_PULSEBAR.sh to verify everything looks correct.
# No credentials required.
# ═══════════════════════════════════════════════════════════════════════════════
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'
PASS=0
TOTAL=0

check() {
    TOTAL=$((TOTAL + 1))
    if eval "$2" >/dev/null 2>&1; then
        echo -e "  ${GREEN}PASS${NC}  $1"
        PASS=$((PASS + 1))
    else
        echo -e "  ${RED}FAIL${NC}  $1"
    fi
}

echo ""
echo "RISE PulseBar — Release Verification"
echo "═════════════════════════════════════"
echo ""

# Locate artifacts
APP_PATH=$(find src-tauri/target/release/bundle/macos -name "*.app" -maxdepth 1 2>/dev/null | head -1)
DMG_PATH=$(find src-tauri/target/release/bundle/dmg -name "*.dmg" -maxdepth 1 2>/dev/null | head -1)

check ".app bundle exists" "[ -d '$APP_PATH' ]"
check ".dmg file exists"   "[ -f '$DMG_PATH' ]"

# Code signature
check "App is code-signed" "codesign --verify --deep --strict '$APP_PATH'"

# spctl (Gatekeeper)
SPCTL_RESULT=$(spctl --assess --verbose "$APP_PATH" 2>&1 || true)
check "spctl accepts app"  "echo '$SPCTL_RESULT' | grep -qi 'accepted'"

# Staple check
check "App has stapled ticket" "xcrun stapler validate '$APP_PATH'"
check "DMG has stapled ticket" "xcrun stapler validate '$DMG_PATH'"

# Version consistency
VERSION=$(grep '"version"' src-tauri/tauri.conf.json | head -1 | sed 's/.*: *"\(.*\)".*/\1/')
check "Cargo.toml version matches"     "grep -q 'version = \"$VERSION\"' src-tauri/Cargo.toml"
check "tauri.conf.json version"        "grep -q '\"version\": \"$VERSION\"' src-tauri/tauri.conf.json"
check "README mentions DMG name"       "grep -q '${VERSION}' README.md"
check "RELEASE_NOTES mentions version" "grep -q 'v${VERSION}' RELEASE_NOTES.md"

# Git tag
check "Git tag v${VERSION} exists"     "git tag -l | grep -q 'v${VERSION}'"

# Binary info
BINARY=$(find src-tauri/target/release -name "rise-pulsebar" -maxdepth 1 2>/dev/null | head -1)
if [ -n "$BINARY" ]; then
    SIZE=$(du -h "$BINARY" | cut -f1)
    ARCH=$(file "$BINARY" | grep -o 'arm64\|x86_64' | head -1)
    echo ""
    echo "  Binary: $SIZE ($ARCH)"
fi
if [ -n "$DMG_PATH" ]; then
    DMG_SIZE=$(du -h "$DMG_PATH" | cut -f1)
    echo "  DMG:    $DMG_SIZE"
fi

echo ""
echo "═════════════════════════════════════"
echo -e "  ${GREEN}${PASS}${NC} / ${TOTAL} checks passed"
echo ""

if [ "$PASS" -lt "$TOTAL" ]; then
    echo -e "  ${YELLOW}Some checks failed. Review output above.${NC}"
    exit 1
fi
