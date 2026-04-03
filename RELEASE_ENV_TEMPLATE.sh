#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# RELEASE_ENV_TEMPLATE.sh — Credential template for RELEASE_PULSEBAR.sh
#
# INSTRUCTIONS:
#   1. Copy this file:   cp RELEASE_ENV_TEMPLATE.sh RELEASE_ENV_LOCAL.sh
#   2. Fill in your real values in RELEASE_ENV_LOCAL.sh
#   3. Before running the release script:  source ./RELEASE_ENV_LOCAL.sh
#
# NEVER commit RELEASE_ENV_LOCAL.sh — it is .gitignored.
# ═══════════════════════════════════════════════════════════════════════════════

# Your Apple ID email (used for notarization)
export APPLE_ID="your-apple-id@example.com"

# Your Apple Developer Team ID (10-character alphanumeric)
# Find it at: https://developer.apple.com/account → Membership Details
export TEAM_ID="YOUR_TEAM_ID"

# An app-specific password (NOT your regular Apple ID password)
# Generate one at: https://appleid.apple.com → Sign-In and Security → App-Specific Passwords
export APP_PASSWORD="xxxx-xxxx-xxxx-xxxx"
