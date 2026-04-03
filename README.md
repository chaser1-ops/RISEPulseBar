# Rise PulseBar

A lightweight macOS menu bar system monitor. CPU, GPU, memory, network — always visible, never in the way.

## Features

- **Live menu bar** — CPU and memory usage updates every second with fixed-width formatting (no jitter)
- **GPU monitoring** — Apple Silicon GPU utilization via ioreg (hides gracefully if unavailable)
- **Top process** — shows the most CPU-hungry process name and its usage
- **Sparklines** — 60-second CPU and GPU history graphs in the dropdown
- **Network** — real-time download/upload speeds
- **Settings** — mode/rate from right-click tray menu or in-panel controls; all changes persist and sync instantly
- **About** — version info accessible from tray menu or panel
- **Zero bloat** — no cloud, no analytics, no accounts

## Performance

| Metric | Target | Actual |
|---|---|---|
| CPU usage | < 2% | ~0.3% idle |
| Memory | < 80 MB | ~45 MB |
| Panel open | < 100 ms | instant |

## Install

1. Open `Rise PulseBar_1.0.2_aarch64.dmg`
2. Drag **Rise PulseBar** to **Applications**
3. Launch from Applications
4. Click the menu bar item to open the dashboard

> On first launch macOS may show a security prompt — see [INSTALL.md](INSTALL.md).

## Usage

| Action | Result |
|---|---|
| Left-click menu bar | Open / close dashboard |
| Right-click menu bar | Mode, refresh rate, launch at login, about, quit |
| Mode: MIN | CPU + memory only |
| Mode: STD | All stats |
| Mode: FULL | All stats + GPU in menu bar |

## Build from source

```bash
# Prerequisites: Rust 1.77+, Tauri CLI 2.x, Node 18+
git clone https://github.com/chaser1-ops/RISEPulseBar
cd RISEPulseBar
cargo tauri build
```

## Requirements

- macOS 12.0 (Monterey) or later
- Apple Silicon or Intel Mac
