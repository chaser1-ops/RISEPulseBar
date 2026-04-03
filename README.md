# RISE PulseBar

RISE PulseBar is a real-time system monitor for macOS with a clean, minimal interface. Track CPU, memory, network, and performance stats directly from your menu bar — fast, private, and distraction-free. No tracking. No noise. Brought to you by **RISE Studio Labs**.

## Features

- **Live menu bar** — CPU, memory, and GPU usage update every second with fixed-width text (no jitter)
- **GPU monitoring** — Apple Silicon GPU utilization; hides gracefully if unavailable
- **Top process** — shows the most CPU-hungry process and its usage
- **Sparklines** — 60-second rolling history graphs for CPU and GPU
- **Network** — real-time download / upload speeds
- **Settings** — mode, refresh rate, and launch-at-login from right-click tray menu or in-panel controls; all changes persist instantly
- **About** — version and credits accessible from tray menu
- **Fully local** — no cloud, no analytics, no telemetry, no accounts

## Performance

| Metric | Target | Actual |
|---|---|---|
| CPU usage | < 2% | ~0.3% idle |
| Memory | < 80 MB | ~45 MB |
| Panel open | < 100 ms | instant |

## Install

1. Download `Rise PulseBar_1.0.2_aarch64.dmg` from [Releases](https://github.com/chaser1-ops/RISEPulseBar/releases)
2. Open the DMG, drag **Rise PulseBar** to **Applications**
3. Launch from Applications
4. Click the menu bar item to open the dashboard

See [INSTALL.md](INSTALL.md) for build-from-source, signing, and notarization instructions.

## Usage

| Action | Result |
|---|---|
| Left-click menu bar | Open / close dashboard |
| Right-click menu bar | Mode, refresh rate, launch at login, about, quit |
| Mode: MIN | CPU only in menu bar |
| Mode: STD | CPU + MEM in menu bar, all cards in panel |
| Mode: FULL | CPU + MEM + GPU in menu bar |

## Build from Source

```bash
# Prerequisites: Rust 1.77+, Tauri CLI 2.x, Xcode Command Line Tools
git clone https://github.com/chaser1-ops/RISEPulseBar
cd RISEPulseBar
cargo tauri build
```

## Requirements

- macOS 12.0 (Monterey) or later
- Apple Silicon or Intel Mac

## License

MIT

---

**RISE Studio Labs** — lightweight tools, zero noise.
