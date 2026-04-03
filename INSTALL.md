# RISE PulseBar — Install & Release Operations Guide

## Quick Install (end users)

1. Download `Rise PulseBar_1.0.2_aarch64.dmg`
2. Open the DMG and drag **Rise PulseBar** to **Applications**
3. Launch from **Applications** (first time: right-click > Open to bypass Gatekeeper if unsigned)
4. The app appears in your menu bar — no dock icon

## Build from Source

```bash
# Prerequisites: Rust 1.77+, Tauri CLI 2.x, Xcode Command Line Tools
cd "/Volumes/AI_SSD/Rise PulseBar"
cargo tauri build
```

Artifacts appear at:
- `src-tauri/target/release/bundle/macos/Rise PulseBar.app`
- `src-tauri/target/release/bundle/dmg/Rise PulseBar_<version>_aarch64.dmg`

---

## Signed + Notarized Release (for distribution)

### Prerequisites

- Active [Apple Developer Program](https://developer.apple.com/programs/) membership
- Xcode Command Line Tools: `xcode-select --install`
- **Developer ID Application** certificate installed in Keychain
- An [app-specific password](https://appleid.apple.com) for notarization

### Step-by-step

```bash
# 1. Prepare credentials (one time)
cp RELEASE_ENV_TEMPLATE.sh RELEASE_ENV_LOCAL.sh
# Edit RELEASE_ENV_LOCAL.sh with your real Apple ID, Team ID, and app-specific password

# 2. Source credentials
source ./RELEASE_ENV_LOCAL.sh

# 3. Run the release script
./RELEASE_PULSEBAR.sh
```

The script will:
- Verify project context and required tools
- Detect your signing identity from Keychain
- Update `tauri.conf.json` if `signingIdentity` is null
- Build in release mode
- Submit DMG to Apple notarization
- Staple the notarization ticket to both `.app` and `.dmg`
- Run `spctl` verification and print results

### Post-release verification

```bash
./VERIFY_RELEASE.sh
```

Checks code signature, Gatekeeper acceptance, staple validity, version consistency, and git tag.

---

## Release Checklist

- [ ] Version numbers match in: `Cargo.toml`, `tauri.conf.json`, Rust `APP_VERSION`, HTML About, `RELEASE_NOTES.md`
- [ ] `RELEASE_ENV_LOCAL.sh` sourced with valid credentials
- [ ] `./RELEASE_PULSEBAR.sh` completes without errors
- [ ] `./VERIFY_RELEASE.sh` shows all checks passing
- [ ] DMG uploaded to GitHub release
- [ ] DMG uploaded to Gumroad (if applicable)
- [ ] Test install on clean user account: drag to Applications, launch, verify menu bar appears

---

## Troubleshooting

| Issue | Fix |
|---|---|
| "can't be opened because Apple cannot check it" | Right-click > Open, or run `xattr -cr /Applications/Rise\ PulseBar.app` |
| `spctl` shows "rejected" | Signing identity may be wrong, or notarization not stapled — re-run release script |
| Notarization fails | Verify `entitlements.plist` has `app-sandbox = false`; check Apple Developer portal for issues |
| No signing identity found | Install your Developer ID certificate from Apple Developer portal into Keychain Access |
| `APPLE_ID` / `TEAM_ID` / `APP_PASSWORD` not set | `source ./RELEASE_ENV_LOCAL.sh` before running the release script |

## Requirements

- macOS 12.0 (Monterey) or later
- Apple Silicon or Intel Mac
