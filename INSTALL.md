# Installation Guide

## Standard Install (unsigned build)

1. Open `Rise PulseBar_1.0.1_aarch64.dmg`
2. Drag **Rise PulseBar.app** into the **Applications** folder
3. Open **Applications** and right-click **Rise PulseBar → Open**
4. Click **Open** on the Gatekeeper dialog
5. The app appears in your menu bar — no dock icon

## Signed + Notarized Build (distribution)

### Prerequisites

- Active **Apple Developer Program** membership ($99/year)
- Xcode Command Line Tools: `xcode-select --install`
- Your **Developer ID Application** certificate installed in Keychain

### Step 1 — Configure signing identity

Edit `src-tauri/tauri.conf.json`:

```json
"macOS": {
  "signingIdentity": "Developer ID Application: Your Name (TEAMID)",
  "entitlements": "entitlements.plist"
}
```

Find your identity:
```bash
security find-identity -v -p codesigning | grep "Developer ID Application"
```

### Step 2 — Build signed app

```bash
cargo tauri build
```

Tauri automatically signs the `.app` and `.dmg` using the identity above.

### Step 3 — Notarize

```bash
# Store credentials once
xcrun notarytool store-credentials "rise-pulsebar-profile" \
  --apple-id "your@email.com" \
  --team-id "YOUR_TEAM_ID" \
  --password "app-specific-password"

# Submit for notarization
xcrun notarytool submit \
  "src-tauri/target/release/bundle/dmg/Rise PulseBar_1.0.1_aarch64.dmg" \
  --keychain-profile "rise-pulsebar-profile" \
  --wait
```

### Step 4 — Staple

```bash
xcrun stapler staple \
  "src-tauri/target/release/bundle/macos/Rise PulseBar.app"

xcrun stapler staple \
  "src-tauri/target/release/bundle/dmg/Rise PulseBar_1.0.1_aarch64.dmg"
```

### Step 5 — Verify

```bash
spctl --assess --verbose \
  "src-tauri/target/release/bundle/macos/Rise PulseBar.app"
# Expected: "accepted" source=Notarized Developer ID
```

## App-Specific Password

Generate at [appleid.apple.com](https://appleid.apple.com) → Sign-In and Security → App-Specific Passwords.

## Troubleshooting

| Error | Fix |
|---|---|
| "Rise PulseBar can't be opened" | Right-click → Open, or `xattr -cr /Applications/Rise\ PulseBar.app` |
| Gatekeeper blocks | Ensure signing identity is Developer ID (not Mac App Store) |
| Notarization fails | Check entitlements.plist — `com.apple.security.app-sandbox` must be `false` |
