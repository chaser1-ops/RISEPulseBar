# Rise PulseBar v1.1.0 — Feature Plan

**Product:** Rise PulseBar
**Current Release:** v1.0.2 (frozen)
**Target Release:** v1.1.0
**Brand:** RISE Studio Labs
**Date:** 2026-04-03

---

## Objective

Close the feature gap with Stats and iStat Menus on the four highest-impact missing capabilities, while preserving PulseBar's core differentiators: tiny footprint, native colored menu bar text, live animated icon, zero config.

---

## Approved Tier 1 Features (in implementation order)

### 1. Temperature Sensors

**What:** CPU package temp, GPU temp, SSD temp displayed in the dashboard panel. Optional menu bar tray item: `TMP 52°` in red/warm color.

**Architecture Impact:**
- New data source: Apple Silicon temps via IOKit `SMC` or `IOHIDSensor` keys
- The `sysinfo` crate (0.32) exposes `Components` which wraps IOKit thermal sensors on macOS
- Fallback: parse `sudo powermetrics --samplers smc` (requires elevated privileges — not ideal)
- Preferred: `sysinfo::Components::new_with_refreshed_list()` — no root required, covers CPU/GPU/SSD temps on Apple Silicon

**UI Impact:**
- New row(s) in dashboard panel for temperature readings
- Optional new `NativeStatusItem` with warm red/orange color
- Bar chart icon stays 4-bar (temps don't map well to a bar)

**Risk Level:** LOW
- `sysinfo::Components` handles the hard IOKit work
- Apple Silicon reports temps without root on macOS 12+
- No new crate dependencies needed

**macOS Caveats:**
- Intel Macs report different sensor names than Apple Silicon
- Some sensors may not be available on all hardware (e.g., no discrete GPU temp on MacBook Air)
- Sensor names vary: `SOC MTR Temp Sensor0`, `GPU MTR Temp Sensor0`, etc. — need fuzzy matching

---

### 2. Battery Module

**What:** Battery percentage, charging state (charging/discharging/full), health %, cycle count, time remaining estimate. Dashboard panel section + optional menu bar item: `BAT 87%` in yellow/amber.

**Architecture Impact:**
- New data source: IOKit `IOPMPowerSource` via `sysinfo` crate or direct IOKit calls
- `sysinfo` 0.32 does NOT expose battery info — need a new approach
- Options:
  - **A.** `battery` crate (cross-platform, 0.7.x) — provides charge %, state, health, time-to-empty
  - **B.** Direct `ioreg -r -c AppleSmartBattery` parsing (similar to GPU approach)
  - **C.** `pmset -g batt` shell command parsing
- Recommendation: **Option B** (`ioreg` parsing) — consistent with existing GPU pattern, no new dependencies, gives all fields including cycle count and design capacity

**UI Impact:**
- New dashboard section: battery icon + percentage + state + health bar
- Optional `NativeStatusItem`: `BAT 87%` in amber (#FFB800)
- Should hide automatically on desktops (no battery present)

**Risk Level:** LOW
- `ioreg -r -c AppleSmartBattery` is stable across macOS versions
- Desktop Macs simply return no results — graceful degradation
- No elevated privileges required

**macOS Caveats:**
- `TimeRemaining` from IOKit can be `-1` (calculating) or `0` (unlimited/AC) — need special handling
- Health = `MaxCapacity / DesignCapacity * 100` — both available from AppleSmartBattery
- Cycle count directly available as `CycleCount` key

---

### 3. Network Speed in Tray Text

**What:** Live upload/download speed as colored menu bar text: `NET 2.3/0.1 MB/s` in teal/cyan.

**Architecture Impact:**
- Data already collected in metrics thread (`net_rx_kbps`, `net_tx_kbps`)
- Bug to fix first: current calculation is bytes-since-last-refresh / 1024, NOT actual rate. Must divide by `rate_ms / 1000.0` to get true KB/s
- New `NativeStatusItem` for network (5th colored tray item)
- Ordering: icon | CPU | RAM | DSK | GPU | NET (or NET before GPU)

**UI Impact:**
- New colored tray item in teal/cyan (#00BCD4)
- Format: `NET ↓2.3 ↑0.1` or `NET 2.3/0.1 MB/s` — need to test readability at menu bar size
- Auto-scale units: B/s → KB/s → MB/s → GB/s
- Visibility: tied to "full" display mode (or "standard" and above)

**Risk Level:** VERY LOW
- All infrastructure exists
- Just a new NativeStatusItem + rate calculation fix + unit formatting

**macOS Caveats:**
- VPN interfaces may inflate numbers — consider filtering to `en0`/`en1` only, or let user choose
- Sleep/wake can cause a spike on first refresh (accumulated bytes during sleep) — clamp to reasonable max

---

### 4. Top 5 Processes

**What:** Expand the dashboard panel to show a ranked list of top 5 CPU-consuming processes (name + CPU %) instead of just the single top process.

**Architecture Impact:**
- Data already available: `sys.processes()` is fully populated each tick
- Change: collect top 5 instead of top 1 in the metrics snapshot
- Modify `Metrics` struct: `top_processes: Vec<(String, f32)>` (max 5 entries)
- Frontend: render a small table/list in the panel

**UI Impact:**
- Replace single "Top Process" line with a 5-row mini-table
- Each row: process name (truncated to ~20 chars) + CPU % + optional small bar
- Sortable by CPU % (default) with memory as secondary

**Risk Level:** VERY LOW
- No new APIs, no new dependencies
- Pure data reshaping + frontend layout

**macOS Caveats:**
- Process names from `sysinfo` use `p.name()` which returns the executable name, not the display name. `kernel_task`, `WindowServer`, `mds_stores` are not user-friendly — consider a display name mapping for common macOS processes
- Per-process CPU is reported as 0-100*num_cores by sysinfo; already normalizing by core count

---

## Implementation Order Rationale

1. **Temperature** first — biggest perception gap, all competitors have it, proves PulseBar is a "real" monitor
2. **Battery** second — most requested feature for laptop users, easy win
3. **Network speed** third — data pipeline exists, mostly plumbing work
4. **Top 5 processes** fourth — pure frontend enhancement, lowest risk

---

## Scope Boundaries

### In scope for v1.1.0
- The four features above
- Any bug fixes discovered during v1.0.2 release
- Updated screenshots and marketing copy

### Out of scope for v1.1.0
- Fan control
- Memory pressure (green/yellow/red) — consider for v1.2
- Notification/alert system — consider for v1.2
- Per-core CPU breakdown — consider for v1.2
- Homebrew cask — can ship independently anytime
- Global keyboard shortcut — consider for v1.2
- Weather, clocks, calendar — never

---

## Estimated Effort

| Feature | Backend | Frontend | Testing | Total |
|---------|---------|----------|---------|-------|
| Temperature sensors | 2-3 hrs | 1-2 hrs | 1 hr | ~5 hrs |
| Battery module | 2-3 hrs | 1-2 hrs | 1 hr | ~5 hrs |
| Network speed tray | 1 hr | 0.5 hr | 0.5 hr | ~2 hrs |
| Top 5 processes | 0.5 hr | 1-2 hrs | 0.5 hr | ~3 hrs |
| **Total** | | | | **~15 hrs** |

---

## Success Criteria

- All four features working on Apple Silicon (M1/M2/M3/M4)
- Temperature gracefully degrades on hardware without sensors
- Battery module auto-hides on desktop Macs
- Network speed accurately reflects actual throughput at all refresh rates
- Top 5 processes list updates every tick
- Memory footprint stays under 60MB
- CPU usage stays under 1%
- DMG size stays under 15MB

---

*RISE Studio Labs — lightweight tools, zero noise.*
