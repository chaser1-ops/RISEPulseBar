# Rise PulseBar v1.0.2 — Final Release Closeout

**Product:** Rise PulseBar v1.0.2
**Date:** 2026-04-03
**Operator:** Chase Neel (RISE Studio Labs)
**Executor:** Claude Code (Opus 4.6)
**Supervising Architect:** ChatGPT

---

## Release Checklist Status

| # | Task | Status |
|---|------|--------|
| R1 | Create local Apple credential file | COMPLETE |
| R2 | Source credentials and run release script | COMPLETE |
| R3 | Verify notarization result | COMPLETE |
| R4 | Capture screenshots | COMPLETE (7 screenshots) |
| R5 | Integrate screenshots into README | COMPLETE |
| R6 | Finalize GitHub release | DRAFT READY (notarized DMG attached, publish when ready) |
| R7 | Finalize Gumroad listing | PENDING (quiet publish when ready) |
| R8 | Produce final release closeout | THIS DOCUMENT |

---

## Notarization

- **Status:** ACCEPTED
- **Final Submission ID:** f1bc404b-cb79-4e4b-943d-2f20341dbc57
- **App staple:** validated
- **DMG staple:** validated
- **spctl assessment:** accepted — source=Notarized Developer ID
- **Signing identity:** Developer ID Application: ANTHONY CHASE NEEL (7S662F7L7Y)

---

## Bug Fixes Applied During Release Session

| Fix | Description |
|-----|-------------|
| DSK accuracy | Disk metric now reads boot volume (/) only — ignores external/Thunderbolt drives |
| Panel positioning | Panel opens near tray icon instead of far-right corner |
| Panel draggable | Header bar is a drag surface via Tauri 2 startDragging() API |
| Missing capability | Added core:window:allow-start-dragging to capabilities/default.json |
| Auto-hide removed | Panel no longer vanishes on blur — closes via X button only |
| White flash fixed | Body background set to dark theme color instead of transparent |
| Panel height | Increased to 640px to show Mode/Refresh/Login controls without cutoff |
| Website branding | risestudiolabs.com added to about overlay (clickable link) |

---

## Screenshots

All saved in `screenshots/`:

| File | Content |
|------|---------|
| screenshot-menubar-full.jpg | Menu bar — Full mode (CPU + RAM + DSK + GPU) |
| screenshot-menubar-std.jpg | Menu bar — Standard mode (CPU + RAM) |
| screenshot-menubar-min.jpg | Menu bar — Minimal mode (CPU only) |
| screenshot-panel.jpg | Dashboard panel — Full mode |
| screenshot-panel-min.jpg | Dashboard panel — Minimal mode |
| screenshot-rightclick.jpg | Right-click context menu with Mode submenu |
| screenshot-about.jpg | About overlay with version and branding |

---

## Distribution Status

| Channel | Status | Action to Publish |
|---------|--------|-------------------|
| **DMG (direct share)** | READY | Send `Rise PulseBar_1.0.2_aarch64.dmg` via AirDrop/iMessage/Drive |
| **GitHub Releases** | DRAFT | Run: `gh release edit v1.0.2 --draft=false` |
| **Gumroad** | NOT YET PUBLISHED | Upload DMG + screenshots, set $0+ price, publish quietly |

---

## Git State

- **Branch:** main
- **HEAD:** 8889089
- **Remote:** https://github.com/chaser1-ops/RISEPulseBar.git
- **Tag:** v1.0.2
- **Working tree:** clean (except untracked planning docs)

---

## Final Build Artifacts

- **App:** `src-tauri/target/release/bundle/macos/Rise PulseBar.app`
- **DMG:** `src-tauri/target/release/bundle/dmg/Rise PulseBar_1.0.2_aarch64.dmg` (8.9 MB)
- **Notarized:** YES
- **Stapled:** YES
- **Code-signed:** YES (Developer ID Application)

---

## What Ships in v1.0.2

- Native colored menu bar text (CPU blue, RAM green, DSK orange, GPU purple)
- Live animated hot pink 4-bar chart tray icon
- Draggable dashboard panel with sparkline graphs
- 3 display modes (Minimal / Standard / Full)
- Configurable refresh rate (1s / 2s / 5s)
- Launch at Login toggle
- Boot volume disk monitoring (not external drives)
- Apple Silicon GPU monitoring
- Top process display
- Network download/upload speeds
- About overlay with risestudiolabs.com branding
- ~0.3% CPU, ~45 MB RAM, 8.9 MB DMG
- No tracking, no telemetry, no accounts

---

## Next Steps

1. Share DMG with friends for beta feedback
2. Collect feedback for 1-2 weeks
3. Fix any issues found
4. Publish GitHub release + Gumroad listing
5. Begin v1.1.0 implementation (temperature, battery, network tray, top 5 processes)

---

**RISE Studio Labs** — lightweight tools, zero noise.
