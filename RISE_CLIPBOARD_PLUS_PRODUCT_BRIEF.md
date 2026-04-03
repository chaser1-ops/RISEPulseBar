# RISE Clipboard+ -- Product Brief

**Product:** RISE Clipboard+
**Tagline:** Smart clipboard history for macOS -- instant recall, zero clutter
**Brand:** RISE Studio Labs
**Version:** 0.1.0 (Initial Release)
**Status:** Planning
**Date:** 2026-04-02

---

## Brand Context

RISE Studio Labs builds lightweight tools with zero noise. Every product ships lean, runs local, and respects the user. No telemetry. No accounts. No subscriptions. RISE Clipboard+ is the second product in the RISE lineup, following PulseBar.

---

## Problem Statement

macOS provides a single clipboard slot. Power users -- developers, designers, writers -- copy and paste dozens of items per hour. Losing a previously copied snippet means hunting through files, browser tabs, or terminal history to find it again. Existing clipboard managers are either bloated with features, require cloud accounts, or carry subscription pricing for basic functionality.

---

## Product Overview

RISE Clipboard+ is a clipboard history manager for macOS. It monitors the system clipboard, stores a searchable history of copied items (text, images, files), and provides instant recall through a global keyboard shortcut. Everything stays local by default. The interface is minimal and fast.

---

## Core Features

### Clipboard History
- Captures text, images, and file references automatically.
- Deduplicates consecutive identical copies.
- Configurable history size (Free: 50 items, Pro: unlimited).
- Timestamps and source application metadata for each entry.

### Search and Filter
- Full-text search across all clipboard entries.
- Filter by content type (text, image, file).
- Filter by source application.
- Results update as you type.

### Pin Favorites
- Pin frequently used items to a persistent favorites section.
- Pinned items are excluded from automatic history rotation.
- Drag to reorder pinned items.

### Keyboard Shortcut Recall
- Global hotkey opens the clipboard panel from any application.
- Arrow key navigation through history entries.
- Press Return to paste the selected item into the frontmost application.
- Number keys (1-9) for quick access to recent items.

### Categories and Tags
- Assign color-coded tags to clipboard entries.
- Create custom categories for organization.
- Filter history by tag or category.

### iCloud Sync (Optional)
- Opt-in sync of clipboard history across devices via iCloud.
- Disabled by default. No account required to use the product.
- Sync scope is configurable (all items, pinned only, tagged only).

### Privacy-First Design
- All data stored locally by default.
- No telemetry, analytics, or usage tracking.
- No user accounts or registration.
- Sensitive content detection with optional auto-exclude for password manager entries.
- Clear history with a single action.

---

## Target User

Primary audience: power users, developers, and designers who copy and paste frequently throughout their workflow.

Characteristics:
- Uses macOS as a primary operating system.
- Works across multiple applications simultaneously.
- Values speed, keyboard-driven workflows, and minimal UI.
- Skeptical of tools that phone home or require accounts.
- Willing to pay a fair one-time price for a tool that works well.

---

## Tech Stack

| Component       | Technology                          |
|-----------------|-------------------------------------|
| Framework       | Tauri 2 (Rust backend + WebView UI) |
| Backend         | Rust (clipboard monitoring, storage, system integration) |
| Frontend        | HTML/CSS/JS in system WebView       |
| Storage         | SQLite (local, embedded)            |
| Architecture    | Same foundation as PulseBar         |

Rationale: Tauri 2 provides native performance with a small binary size, no Electron overhead, and direct access to macOS system APIs through Rust. Reusing the PulseBar architecture reduces development time and maintains consistency across the RISE product line.

---

## Platform Requirements

- macOS 12 (Monterey) and later.
- Apple Silicon (M1/M2/M3/M4) and Intel x86_64.
- Universal binary distribution.
- Accessibility permissions required for clipboard monitoring and global hotkey.

---

## Pricing

| Tier   | Price              | Includes                                      |
|--------|--------------------|-----------------------------------------------|
| Free   | $0                 | 50-item history, search, pin favorites, hotkey |
| Pro    | $4.99 (one-time)   | Unlimited history, categories/tags, iCloud sync |

No subscription. No recurring charges. One purchase, lifetime access to the version.

---

## Distribution

- **GitHub Releases:** DMG downloads, release notes, checksums.
- **Gumroad:** Storefront for Pro license purchases and DMG delivery.
- **No Mac App Store** at launch. Evaluate based on demand post-release.

---

## Non-Goals

The following are explicitly out of scope for RISE Clipboard+:

- **No cloud requirement.** The product must function fully offline.
- **No subscription model.** One-time purchase only.
- **No telemetry.** Zero data collection, zero analytics.
- **No user accounts.** No registration, no login, no profile.
- **No cross-platform.** macOS only. No Windows or Linux builds.
- **No clipboard sharing.** This is not a collaboration tool.
- **No AI features.** No summarization, categorization by ML, or smart suggestions.

---

## Success Criteria

- Binary size under 15 MB.
- Clipboard capture latency under 50ms.
- Panel open-to-visible time under 100ms from hotkey press.
- Zero network requests during normal operation (sync disabled).
- Positive reception from developer and design communities.

---

## Open Questions

- Exact global hotkey default (Cmd+Shift+V is common but may conflict).
- Whether to support rich text preservation or normalize to plain text.
- Image thumbnail generation strategy for large screenshots.
- Migration path from other clipboard managers (Paste, Maccy, CopyClip).

---

*RISE Studio Labs -- Lightweight tools, zero noise.*
