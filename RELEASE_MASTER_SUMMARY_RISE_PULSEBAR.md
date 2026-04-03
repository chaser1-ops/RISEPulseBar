# RISE PulseBar — Master Release Summary

**Product:** RISE PulseBar
**Version:** v1.0.2 (canonical release)
**Organization:** RISE Studio Labs
**Operator:** Chase Neel
**Summary Date:** 2026-04-03

---

## Release Identity

| Field | Value |
|---|---|
| Product name | RISE PulseBar |
| Version | v1.0.2 |
| Canonical tag | `v1.0.2` → commit `42c9ac0` |
| Branch | `main` (HEAD contains post-release doc commits ahead of tag) |
| Architecture | arm64 (Apple Silicon) |
| Framework | Tauri 2 (Rust + WebView) |
| Min macOS | 12.0 (Monterey) |

---

## What Shipped

A real-time macOS menu bar system monitor with:

- **Native colored metrics** — CPU (blue), RAM (green), DSK (orange), GPU (purple) as individual status items
- **Live animated tray icon** — hot pink 4-bar chart pulsing with real system values
- **Three display modes** — Minimal, Standard, Full
- **Apple Silicon GPU monitoring** via IOKit (hides gracefully on Intel)
- **Draggable dark-mode dashboard** with 60-second sparkline history graphs
- **Top process display** with CPU usage
- **Real-time network speeds** (download + upload)
- **Right-click context menu** — Mode, Refresh Rate, Launch at Login, About, Quit
- **Atomic settings persistence** — survives restarts
- **Fully local** — no cloud, no analytics, no telemetry, no accounts

### Performance

| Metric | Value |
|---|---|
| CPU usage | ~0.3% idle |
| Memory | ~45 MB |
| DMG size | 8.9 MB |
| Panel open | Instant |

---

## Build Trust Status

| Check | Status |
|---|---|
| Code-signed | **Yes** — Developer ID: ANTHONY CHASE NEEL |
| Team Identifier | 7S662F7L7Y |
| Notarized | **Yes** — Apple Notary Service (2026-04-03 04:45) |
| Stapled (.app) | **Yes** — `stapler validate` passes |
| Stapled (.dmg) | **Yes** — `stapler validate` passes |
| spctl assessment | **Accepted** — source: Notarized Developer ID |
| Hardened runtime | **Yes** — flags=0x10000(runtime) |

### Artifacts

| Artifact | Path | Size |
|---|---|---|
| App bundle | `src-tauri/target/release/bundle/macos/Rise PulseBar.app` | — |
| DMG installer | `src-tauri/target/release/bundle/dmg/Rise PulseBar_1.0.2_aarch64.dmg` | 8.9 MB |

---

## Distribution Status

| Channel | State | Details |
|---|---|---|
| **Gumroad** | **LIVE** | gotsentinel.gumroad.com/l/rise-pulsebar |
| | | Free ($0+), $5 suggested tip (pay what you want) |
| | | DMG uploaded (8.92 MB) |
| | | Cover images: **not yet uploaded** |
| **GitHub Release** | **DRAFT (intentional)** | github.com/chaser1-ops/RISEPulseBar/releases |
| | | Tag v1.0.2, DMG attached, release notes complete |
| | | Held private by operator decision — not an oversight |
| **GitHub Repo** | **Private** | Code pushed, tag pushed, main branch current |
| **Website** | **Updated** | risestudiolabs.com reflects current product |

---

## Intentional Open Items (Non-Blockers)

These are known, acknowledged, and intentionally deferred:

1. **Gumroad cover images** — No product screenshots uploaded to the listing carousel. Screenshots exist locally in `/screenshots/` (7 files).
2. **GitHub release remains draft** — Operator decision to keep repo/release private for now.
3. **GitHub DMG filename** — Attached as `Rise.PulseBar_1.0.2_aarch64.dmg` (dot instead of space). Cosmetic only. Fix if/when publishing.
4. **3 untracked icon option PNGs** — `src-tauri/icons/option-{P,alien,dot}.png`. Candidate tray icons, not finalized. Commit the chosen one or delete when decided.

---

## Repo Truth

| File | Purpose |
|---|---|
| `RELEASE_PULSEBAR.sh` | Automated build, sign, notarize, staple script |
| `RELEASE_ENV_TEMPLATE.sh` | Apple credential template (secrets gitignored) |
| `VERIFY_RELEASE.sh` | 10-check post-release validation |
| `RELEASE_NOTES.md` | Version history and changelog |
| `INSTALL.md` | Install guide and release ops reference |
| `EXECUTION_CLOSEOUT_REPORT.md` | Detailed phase-by-phase execution record |
| `PULSEBAR_V1_1_FEATURE_PLAN.md` | v1.1.0 feature plan (not started) |
| `PULSEBAR_V1_1_BUILD_BLUEPRINT.json` | v1.1.0 task breakdown (33 tasks) |
| `RISE_CLIPBOARD_PLUS_PRODUCT_BRIEF.md` | Next product brief (Clipboard+) |
| `RISE_CLIPBOARD_PLUS_BUILD_BLUEPRINT.json` | Clipboard+ task breakdown |

---

## Version Consistency (Verified)

Version `1.0.2` confirmed across: `Cargo.toml`, `tauri.conf.json`, `src/lib.rs`, `dist/index.html` (header + about modal), `README.md`, `INSTALL.md`, `RELEASE_NOTES.md`, and git tag.

---

## Future Work

**v1.1.0** is planned but not started. Recommended direction:

- Temperature sensors
- Battery module
- Network speed in tray text
- Top 5 processes panel

Planning docs are committed. Implementation should begin only after v1.0.2 distribution decisions are finalized.

---

## For Future Sessions

**Read this file first.** It is the single source of truth for the shipped state of RISE PulseBar v1.0.2. If any other document contradicts this summary, this summary takes precedence (it was written last and verified against all other sources).

Key things to know:
- v1.0.2 is **shipped and stable** — do not re-release or re-notarize unless there is a real reason.
- GitHub release is **draft on purpose** — do not publish without operator approval.
- Gumroad is **live and free** — the $5 is a suggested tip, not a minimum.
- No v1.1 feature work until operator says go.

---

**RISE Studio Labs** — lightweight tools, zero noise.
