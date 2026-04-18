# Rise PulseBar — Documentation

Rise PulseBar is a privacy-first macOS menu bar system monitor by RISE Studio Labs. It displays real-time CPU, RAM, disk, and GPU metrics directly in the menu bar alongside an animated tray icon that pulses with system load.

## Quick Links

- **Overview & install:** see [`../README.md`](../README.md) at the project root
- **License:** [MIT](../LICENSE)
- **Technical config:** [`../src-tauri/Cargo.toml`](../src-tauri/Cargo.toml) and [`../src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json)
- **Distribution:** [gotsentinel.gumroad.com/l/rise-pulsebar](https://gotsentinel.gumroad.com/l/rise-pulsebar)
- **Source:** [github.com/chaser1-ops/RISEPulseBar](https://github.com/chaser1-ops/RISEPulseBar)

## Architecture

- **Stack:** Tauri v2 + Rust 2021 + vanilla HTML/JS/CSS frontend
- **Platform:** macOS 12.0 Monterey or later, Apple Silicon native (aarch64)
- **Bundle ID:** `com.rise.pulsebar`
- **Distribution:** Developer ID signed, free on Gumroad with suggested tip

## Privacy

- Zero telemetry
- Zero accounts
- Zero cloud calls
- All metrics read locally via `sysinfo` and `ioreg`

## Reporting Issues

GitHub Issues on the public repo, or reach out via the Gumroad listing.
