# Release Notes — v1.0.3

## v1.0.3 — 2026-04-18

### Bug Fixes

- Fixed menu bar label flicker on macOS 26.4 caused by redundant AppKit writes on every refresh tick
- Eliminated CPU spike (7% → <1% idle) by caching metric values and skipping AppKit calls when unchanged
- Prevented unnecessary tray icon re-renders when metric percentages have not changed
- Suppressed implicit NSSceneFenceAction animations on per-metric NSStatusItem label updates

### Technical Notes

- AppKit write caching: set_visible, set_colored_title (x4), and set_icon now only fire on value change
- NSAnimationContext grouping wraps all label updates atomically to prevent stagger on macOS 26
- No user-visible feature changes — purely a stability and performance release

---

# Release Notes — v1.0.2

## What's new in v1.0.2

### Right-click tray menu (expanded)
- **Mode selector** — Minimal / Standard / Full, with live checkmarks showing active selection
- **Refresh rate** — 1s / 2s / 5s, with live checkmarks
- **Launch at Login** — toggle with checkmark reflecting current state
- **About Rise PulseBar…** — opens version/author modal in the panel
- All tray menu changes instantly sync to the in-panel controls (no stale state)

### About modal (P3)
- Native-feeling modal overlay: app name, v1.0.2, Rise Studio Labs, tagline
- Opens from tray menu "About…" or programmatically
- Dismisses via Done button, clicking outside, or Escape

### Atomic settings write (P4)
- Settings now written to `.tmp` file first, JSON validated, then atomically renamed to `settings.json`
- Eliminates any possibility of corruption under rapid setting changes

### DMG background
- Regenerated with dot-grid texture, accent glow, drag arrow, app/Applications labels, tagline

### Notarization preflight (P5)
- All version numbers verified consistent across `Cargo.toml`, `tauri.conf.json`, Rust const, HTML
- `entitlements.plist` correctly referenced; no restricted APIs without entitlement
- Bundle identifier `com.rise.pulsebar` confirmed

---

# Release Notes — v1.0.1

## What's new

### Menu bar
- **Zero-jitter title** — values are right-padded to fixed character width (`CPU   9%  MEM  68%`), no layout shift ever
- **GPU stat** — Apple Silicon GPU utilization shown in Full mode; hides gracefully if unavailable
- **Full mode** adds GPU to menu bar: `CPU  9%  MEM 68%  GPU 12%`

### Dashboard panel
- **Sparklines** — 60-second rolling history graph for CPU and GPU, rendered on canvas with no library overhead
- **Top process** — most CPU-hungry process name + normalized usage shown below CPU bar
- **Entry animation** — subtle 120ms scale+fade when panel opens
- **Accurate positioning** — panel opens centered under the tray icon, clamped to screen edges

### Settings (in panel footer)
- **Mode selector** — MIN (CPU + MEM only) / STD (all stats) / FULL (all stats + GPU in menu bar)
- **Refresh rate** — 1s / 2s / 5s, persisted across restarts
- **Right-click tray menu** — Open Dashboard / Launch at Login / Quit

### Performance
- CPU usage: ~0.3% idle, ~0.6% active polling
- Memory: ~45 MB RSS
- GPU sampling runs in a separate thread every 2 s (ioreg, no root required)
- Process list refresh uses minimal `ProcessRefreshKind::cpu_only()` — no extra I/O

### Distribution
- `entitlements.plist` configured for notarization
- DMG background image included
- Signed + notarized workflow documented in `INSTALL.md`

## v1.0.0 → v1.0.1 changes

- Added GPU monitoring, sparklines, top process, settings persistence
- Fixed menu bar text jitter with fixed-width formatting
- Panel now positions under cursor (not always right edge)
- Right-click tray menu added
- Panel height increased to 560px to fit all cards
- Version bump
