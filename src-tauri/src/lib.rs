use std::sync::{Arc, Mutex};
use std::time::Duration;
use sysinfo::{Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::{AppHandle, LogicalPosition, Manager, State, WebviewUrl, WebviewWindowBuilder};

const APP_VERSION: &str = "1.0.3";

// ─── Settings ─────────────────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub mode: String,           // "minimal" | "standard" | "full"
    pub refresh_rate_ms: u64,   // 1000 | 2000 | 5000
    pub show_gpu: bool,
    pub show_network: bool,
    pub show_swap: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: "full".into(),
            refresh_rate_ms: 1000,
            show_gpu: true,
            show_network: true,
            show_swap: true,
        }
    }
}

pub struct SettingsState(Arc<Mutex<Settings>>);

fn load_settings(app: &AppHandle) -> Settings {
    let Ok(dir) = app.path().app_data_dir() else { return Settings::default() };
    let path = dir.join("settings.json");
    let Ok(content) = std::fs::read_to_string(path) else { return Settings::default() };
    serde_json::from_str(&content).unwrap_or_default()
}

/// P4 — Atomic write: serialize → temp file → validate → rename
fn save_settings(app: &AppHandle, settings: &Settings) {
    let Ok(dir) = app.path().app_data_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let final_path = dir.join("settings.json");
    let tmp_path = dir.join("settings.json.tmp");

    let Ok(json) = serde_json::to_string_pretty(settings) else { return };

    // Validate the JSON we just produced before touching the real file
    if serde_json::from_str::<serde_json::Value>(&json).is_err() {
        return;
    }

    if std::fs::write(&tmp_path, &json).is_ok() {
        // Atomic rename — replaces final_path in one syscall
        let _ = std::fs::rename(&tmp_path, &final_path);
    }
}

// ─── Metrics ──────────────────────────────────────────────────────────────────

#[derive(Default, Clone, serde::Serialize)]
pub struct Metrics {
    pub cpu_usage: f32,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    pub mem_percent: f32,
    pub swap_used_mb: u64,
    pub swap_total_mb: u64,
    pub net_rx_kbps: f64,
    pub net_tx_kbps: f64,
    pub gpu_usage: Option<f32>,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
    pub disk_percent: f32,
    pub top_process_name: String,
    pub top_process_cpu: f32,
}

pub struct MetricsState(Arc<Mutex<Metrics>>);

// ─── Commands ─────────────────────────────────────────────────────────────────

#[tauri::command]
fn get_metrics(state: State<MetricsState>) -> Metrics {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
fn hide_panel(app: AppHandle) {
    if let Some(w) = app.get_webview_window("panel") {
        let _ = w.hide();
    }
}

#[tauri::command]
fn get_settings(state: State<SettingsState>) -> Settings {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
fn set_settings(app: AppHandle, state: State<SettingsState>, settings: Settings) {
    save_settings(&app, &settings);
    *state.0.lock().unwrap() = settings;

    // v1.0.3: When the dashboard (app.js) writes settings via this
    // command, the Arc<Mutex<Settings>> is updated — but the native
    // menu's checkmarks live in an NSMenu we own, not in the shared
    // settings. Rebuild on the main thread so Mode / Refresh Rate
    // checkmarks reflect the new state immediately. The metrics thread
    // picks up the new mode on its next tick (up to refresh_rate_ms
    // later), which is what drives the menu-bar text update.
    let _ = app.run_on_main_thread(|| {
        colored_tray::rebuild_menu();
    });
}

#[tauri::command]
fn get_autostart_enabled(app: AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    if enabled { al.enable() } else { al.disable() }.map_err(|e| e.to_string())
}

// ─── GPU Thread ───────────────────────────────────────────────────────────────

fn parse_ioreg_value(line: &str, key: &str) -> Option<f32> {
    if let Some(pos) = line.find(key) {
        let rest = &line[pos + key.len()..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(v) = digits.parse::<f32>() {
            return Some(v.clamp(0.0, 100.0));
        }
    }
    None
}

fn parse_gpu_pct(output: &str) -> Option<f32> {
    let mut best: Option<f32> = None;
    for line in output.lines() {
        // Take the max of Device, Renderer, and Tiler utilization
        for key in [
            "\"Device Utilization %\"=",
            "\"Renderer Utilization %\"=",
            "\"Tiler Utilization %\"=",
        ] {
            if let Some(v) = parse_ioreg_value(line, key) {
                best = Some(best.map_or(v, |b: f32| b.max(v)));
            }
        }
    }
    best
}

fn start_gpu_thread(shared: Arc<Mutex<Option<f32>>>, settings: Arc<Mutex<Settings>>) {
    std::thread::spawn(move || loop {
        let gpu = std::process::Command::new("ioreg")
            .args(["-r", "-c", "IOAccelerator", "-l"])
            .output()
            .ok()
            .and_then(|o| parse_gpu_pct(&String::from_utf8_lossy(&o.stdout)));
        *shared.lock().unwrap() = gpu;
        // v1.0.3 FIX-6: honor the user's refresh-rate setting instead
        // of always polling at 1 Hz. Spawning an `ioreg` subprocess is
        // the single most expensive recurring operation in the app, so
        // at the "5 seconds" setting this cuts the GPU-poll overhead
        // by 80%.
        let rate_ms = settings.lock().unwrap().refresh_rate_ms;
        std::thread::sleep(Duration::from_millis(rate_ms));
    });
}

// ─── Inline Unicode bar chart helper ─────────────────────────────────────────
//
// v1.0.3 replaces v1.0.2's separate pixel-based 44×44 RGBA tray-icon bitmap
// with a 4-character Unicode prefix appended to the combined colored title,
// because the pixel icon's NSStatusItem was being evicted by macOS 26
// Control Center under menu-bar space pressure.

/// Map a 0-100 percentage to a Unicode lower-block character for inline bars.
fn bar_char(pct: u32) -> char {
    match pct.min(100) {
        0..=6   => '▁',
        7..=18  => '▂',
        19..=31 => '▃',
        32..=43 => '▄',
        44..=56 => '▅',
        57..=68 => '▆',
        69..=81 => '▇',
        _       => '█',
    }
}

// ─── Native macOS colored tray text ──────────────────────────────────────────
//
// Uses Objective-C runtime to create NSStatusItems with colored attributed text.
// Each metric gets its own status item with a unique color.

#[cfg(target_os = "macos")]
mod colored_tray {
    use objc::declare::ClassDecl;
    use objc::runtime::{Class, Object, Sel};
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::CString;
    use std::sync::{Arc, Mutex, OnceLock};
    use tauri::{AppHandle, Manager};
    use crate::Settings;

    pub struct NativeStatusItem {
        ptr: *mut Object,
    }

    unsafe impl Send for NativeStatusItem {}
    unsafe impl Sync for NativeStatusItem {}

    impl NativeStatusItem {
        pub unsafe fn new() -> Self {
            let status_bar: *mut Object = msg_send![class!(NSStatusBar), systemStatusBar];
            let item: *mut Object =
                msg_send![status_bar, statusItemWithLength: -1.0_f64];
            let _: () = msg_send![item, retain];
            NativeStatusItem { ptr: item }
        }

        /// Expose the raw NSStatusItem pointer for click-handler dispatch.
        pub(crate) unsafe fn raw_ptr(&self) -> *mut Object {
            self.ptr
        }

        /// v1.0.3: Install a custom target/action on the NSStatusBarButton
        /// so left- and right-click are discriminated in `handle_status_click`
        /// rather than always popping the menu. Required because `setMenu:`
        /// alone would hijack left-click for the menu too.
        pub unsafe fn install_click_handler(&self, target: *mut Object) {
            let button: *mut Object = msg_send![self.ptr, button];
            let _: () = msg_send![button, setTarget: target];
            let _: () = msg_send![button, setAction: sel!(statusBarClicked:)];
            // NSEventMaskLeftMouseUp (1<<2) | NSEventMaskRightMouseUp (1<<4)
            let mask: u64 = (1u64 << 2) | (1u64 << 4);
            let _: () = msg_send![button, sendActionOn: mask];
        }

    }

    impl Drop for NativeStatusItem {
        fn drop(&mut self) {
            unsafe {
                let status_bar: *mut Object =
                    msg_send![class!(NSStatusBar), systemStatusBar];
                let _: () = msg_send![status_bar, removeStatusItem: self.ptr];
                let _: () = msg_send![self.ptr, release];
            }
        }
    }

    // ─── v1.0.3: Native NSMenu + click dispatch ─────────────────────────────
    //
    // Tauri's `tauri::menu::Menu` keeps its underlying NSMenu pointer behind
    // a sealed trait (`ContextMenuBase::inner_context`), so we can't reuse
    // it on our native NSStatusItem. Instead we build an NSMenu directly
    // with AppKit and register a single Objective-C target class that reads
    // the clicked NSMenuItem's `tag` and dispatches back into Rust.

    pub struct MenuContext {
        pub app: AppHandle,
        pub settings: Arc<Mutex<Settings>>,
        pub combined_tray: Arc<NativeStatusItem>,
        /// Current NSMenu pointer, stored as usize for Send+Sync. Updated
        /// every time `rebuild_native_menu` runs; the click handler reads
        /// it to pop the menu on right-click.
        pub menu_ptr: Mutex<usize>,
    }

    static MENU_CTX: OnceLock<MenuContext> = OnceLock::new();

    pub fn init_menu_context(
        app: AppHandle,
        settings: Arc<Mutex<Settings>>,
        combined_tray: Arc<NativeStatusItem>,
    ) {
        let _ = MENU_CTX.set(MenuContext {
            app,
            settings,
            combined_tray,
            menu_ptr: Mutex::new(0),
        });
    }

    pub fn menu_context() -> Option<&'static MenuContext> {
        MENU_CTX.get()
    }

    /// v1.0.3: NSStatusBarButton action handler. Called on both left- and
    /// right-mouse-up via `sendActionOn:` masking. Reads the currently-
    /// dispatched NSEvent to discriminate: right-click pops the stored
    /// NSMenu, left-click toggles the dashboard panel.
    extern "C" fn handle_status_click(_this: &Object, _sel: Sel, _sender: *mut Object) {
        unsafe {
            let Some(ctx) = MENU_CTX.get() else { return };
            let ns_app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
            let event:  *mut Object = msg_send![ns_app, currentEvent];
            // NSEventTypeLeftMouseDown = 1, LeftMouseUp = 2,
            // RightMouseDown = 3, RightMouseUp = 4.
            let event_type: u64 = msg_send![event, type];

            if event_type == 3 || event_type == 4 {
                // Right-click → pop the stored menu beside the status item.
                let menu_addr = *ctx.menu_ptr.lock().unwrap();
                if menu_addr != 0 {
                    let item_ptr = ctx.combined_tray.raw_ptr();
                    #[allow(deprecated)]
                    let _: () = msg_send![item_ptr, popUpStatusItemMenu: menu_addr as *mut Object];
                }
            } else {
                // Left-click (or ctrl/option/cmd click) → toggle dashboard.
                crate::toggle_panel(&ctx.app, None);
            }
        }
    }

    // Tag values for native NSMenuItems. Dispatched by handle_menu_item.
    const TAG_OPEN_DASHBOARD: i64 = 1;
    const TAG_MODE_MINIMAL:   i64 = 10;
    const TAG_MODE_STANDARD:  i64 = 11;
    const TAG_MODE_FULL:      i64 = 12;
    const TAG_RATE_1000:      i64 = 20;
    const TAG_RATE_2000:      i64 = 21;
    const TAG_RATE_5000:      i64 = 22;
    const TAG_LAUNCH_LOGIN:   i64 = 30;
    const TAG_ABOUT:          i64 = 40;
    const TAG_QUIT:           i64 = 50;

    extern "C" fn handle_menu_item(_this: &Object, _sel: Sel, sender: *mut Object) {
        let tag: i64 = unsafe { msg_send![sender, tag] };
        dispatch_menu_action(tag);
    }

    fn dispatch_menu_action(tag: i64) {
        let Some(ctx) = MENU_CTX.get() else { return };
        let app = ctx.app.clone();

        match tag {
            TAG_OPEN_DASHBOARD => crate::toggle_panel(&app, None),

            TAG_MODE_MINIMAL | TAG_MODE_STANDARD | TAG_MODE_FULL => {
                let mode = match tag {
                    TAG_MODE_MINIMAL  => "minimal",
                    TAG_MODE_STANDARD => "standard",
                    _                 => "full",
                };
                let new_settings = {
                    let mut s = ctx.settings.lock().unwrap();
                    s.mode = mode.into();
                    s.clone()
                };
                crate::save_settings(&app, &new_settings);
                if let Some(win) = app.get_webview_window("panel") {
                    let _ = win.eval(&format!(
                        "window.dispatchEvent(new CustomEvent('settings-changed',{{detail:{{mode:'{}'}}}}))",
                        mode
                    ));
                }
                rebuild_native_menu();
            }

            TAG_RATE_1000 | TAG_RATE_2000 | TAG_RATE_5000 => {
                let ms: u64 = match tag {
                    TAG_RATE_1000 => 1000,
                    TAG_RATE_2000 => 2000,
                    _             => 5000,
                };
                let new_settings = {
                    let mut s = ctx.settings.lock().unwrap();
                    s.refresh_rate_ms = ms;
                    s.clone()
                };
                crate::save_settings(&app, &new_settings);
                if let Some(win) = app.get_webview_window("panel") {
                    let _ = win.eval(&format!(
                        "window.dispatchEvent(new CustomEvent('settings-changed',{{detail:{{refresh_rate_ms:{ms}}}}}))"
                    ));
                }
                rebuild_native_menu();
            }

            TAG_LAUNCH_LOGIN => {
                use tauri_plugin_autostart::ManagerExt;
                let al = app.autolaunch();
                if al.is_enabled().unwrap_or(false) {
                    let _ = al.disable();
                } else {
                    let _ = al.enable();
                }
                rebuild_native_menu();
            }

            TAG_ABOUT => {
                crate::show_panel(&app, None);
                if let Some(win) = app.get_webview_window("panel") {
                    let _ = win.eval(
                        "window.dispatchEvent(new CustomEvent('show-about'));"
                    );
                }
            }

            TAG_QUIT => app.exit(0),

            _ => {}
        }
    }

    fn rebuild_native_menu() {
        let Some(ctx) = MENU_CTX.get() else { return };
        use tauri_plugin_autostart::ManagerExt;
        let s  = ctx.settings.lock().unwrap().clone();
        let al = ctx.app.autolaunch().is_enabled().unwrap_or(false);
        unsafe {
            let new_menu = build_native_menu(&s, al);
            swap_menu(ctx, new_menu);
        }
    }

    /// v1.0.3: Public entry point for rebuilding the native menu from
    /// outside the module (e.g. after the panel's segmented-control
    /// changes via `set_settings`). Must be invoked on the main thread.
    pub fn rebuild_menu() {
        rebuild_native_menu();
    }

    /// v1.0.3: atomically replace the stored NSMenu pointer and release the
    /// previous one. Called from both initial install and every rebuild,
    /// so we never leak NSMenus across state changes.
    pub unsafe fn swap_menu(ctx: &MenuContext, new_menu: *mut Object) {
        let old_addr = {
            let mut guard = ctx.menu_ptr.lock().unwrap();
            let old = *guard;
            *guard = new_menu as usize;
            old
        };
        if old_addr != 0 && old_addr != new_menu as usize {
            let _: () = msg_send![old_addr as *mut Object, release];
        }
    }

    // Lazily register the PulseBarMenuTarget Objective-C class. Registered
    // exactly once per process. Exposes two selectors:
    //   - handleMenuItem:    for NSMenuItem clicks
    //   - statusBarClicked:  for NSStatusBarButton clicks
    fn menu_target_class() -> &'static Class {
        static REG: OnceLock<usize> = OnceLock::new();
        let addr = *REG.get_or_init(|| {
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("PulseBarMenuTarget", superclass)
                .expect("PulseBarMenuTarget class already exists");
            unsafe {
                decl.add_method(
                    sel!(handleMenuItem:),
                    handle_menu_item as extern "C" fn(&Object, Sel, *mut Object),
                );
                decl.add_method(
                    sel!(statusBarClicked:),
                    handle_status_click as extern "C" fn(&Object, Sel, *mut Object),
                );
            }
            let cls: &Class = decl.register();
            cls as *const Class as usize
        });
        unsafe { &*(addr as *const Class) }
    }

    // Single target instance shared by every NSMenuItem + the status button.
    pub fn menu_target_instance() -> *mut Object {
        static INST: OnceLock<usize> = OnceLock::new();
        let addr = *INST.get_or_init(|| {
            let cls = menu_target_class();
            let obj: *mut Object = unsafe { msg_send![cls, new] };
            obj as usize
        });
        addr as *mut Object
    }

    /// v1.0.3: Build an NSMenu containing every tray item the previous
    /// Tauri-built menu had. Each item routes clicks through
    /// `handleMenuItem:` on the shared PulseBarMenuTarget instance, which
    /// reads the tag and calls back into Rust.
    pub unsafe fn build_native_menu(settings: &Settings, autostart: bool) -> *mut Object {
        let target = menu_target_instance();

        let menu: *mut Object = msg_send![class!(NSMenu), alloc];
        let menu: *mut Object = msg_send![menu, init];
        let _: () = msg_send![menu, setAutoenablesItems: false];

        add_action_item(menu, "Open Dashboard", TAG_OPEN_DASHBOARD, target);
        add_separator(menu);

        // Mode submenu
        let mode_sub: *mut Object = msg_send![class!(NSMenu), alloc];
        let mode_sub: *mut Object = msg_send![mode_sub, init];
        let _: () = msg_send![mode_sub, setAutoenablesItems: false];
        add_action_item(mode_sub,
            if settings.mode == "minimal"  { "✓  Minimal"  } else { "   Minimal"  },
            TAG_MODE_MINIMAL,  target);
        add_action_item(mode_sub,
            if settings.mode == "standard" { "✓  Standard" } else { "   Standard" },
            TAG_MODE_STANDARD, target);
        add_action_item(mode_sub,
            if settings.mode == "full"     { "✓  Full"     } else { "   Full"     },
            TAG_MODE_FULL,     target);
        add_submenu_item(menu, "Mode", mode_sub);

        // Refresh Rate submenu
        let rate_sub: *mut Object = msg_send![class!(NSMenu), alloc];
        let rate_sub: *mut Object = msg_send![rate_sub, init];
        let _: () = msg_send![rate_sub, setAutoenablesItems: false];
        add_action_item(rate_sub,
            if settings.refresh_rate_ms == 1000 { "✓  1 second"  } else { "   1 second"  },
            TAG_RATE_1000, target);
        add_action_item(rate_sub,
            if settings.refresh_rate_ms == 2000 { "✓  2 seconds" } else { "   2 seconds" },
            TAG_RATE_2000, target);
        add_action_item(rate_sub,
            if settings.refresh_rate_ms == 5000 { "✓  5 seconds" } else { "   5 seconds" },
            TAG_RATE_5000, target);
        add_submenu_item(menu, "Refresh Rate", rate_sub);

        add_separator(menu);
        add_action_item(menu,
            if autostart { "✓  Launch at Login" } else { "   Launch at Login" },
            TAG_LAUNCH_LOGIN, target);
        add_separator(menu);
        add_action_item(menu, "About Rise PulseBar…", TAG_ABOUT, target);
        add_separator(menu);
        add_action_item(menu, "Quit Rise PulseBar", TAG_QUIT, target);

        menu
    }

    unsafe fn add_action_item(menu: *mut Object, title: &str, tag: i64, target: *mut Object) {
        let cstr = CString::new(title).unwrap_or_default();
        let ns_title: *mut Object =
            msg_send![class!(NSString), stringWithUTF8String: cstr.as_ptr()];
        let empty: *mut Object = msg_send![class!(NSString), string];

        let item: *mut Object = msg_send![class!(NSMenuItem), alloc];
        let item: *mut Object = msg_send![item,
            initWithTitle: ns_title
            action: sel!(handleMenuItem:)
            keyEquivalent: empty];
        let _: () = msg_send![item, setTag: tag];
        let _: () = msg_send![item, setTarget: target];
        let _: () = msg_send![item, setEnabled: true];
        let _: () = msg_send![menu, addItem: item];
        let _: () = msg_send![item, release];
    }

    unsafe fn add_submenu_item(menu: *mut Object, title: &str, submenu: *mut Object) {
        let cstr = CString::new(title).unwrap_or_default();
        let ns_title: *mut Object =
            msg_send![class!(NSString), stringWithUTF8String: cstr.as_ptr()];

        let item: *mut Object = msg_send![class!(NSMenuItem), alloc];
        let item: *mut Object = msg_send![item, init];
        let _: () = msg_send![item, setTitle: ns_title];
        let _: () = msg_send![item, setSubmenu: submenu];
        let _: () = msg_send![item, setEnabled: true];
        let _: () = msg_send![menu, addItem: item];
        let _: () = msg_send![item, release];
        let _: () = msg_send![submenu, release];
    }

    unsafe fn add_separator(menu: *mut Object) {
        let sep: *mut Object = msg_send![class!(NSMenuItem), separatorItem];
        let _: () = msg_send![menu, addItem: sep];
    }

    // v1.0.3 REFACTOR: a single colored segment of the combined tray title.
    // The whole menu-bar display is now ONE NSStatusItem whose
    // attributedTitle is a concatenation of these segments, each with its
    // own foreground color. This replaces the v1.0.2 four-separate-items
    // layout, which macOS 26 Control Center rotates/evicts under space
    // pressure (notch + crowded right-side system items).
    #[derive(Clone)]
    pub struct ColoredSegment {
        pub text: String,
        pub r: f64,
        pub g: f64,
        pub b: f64,
    }

    impl NativeStatusItem {
        /// v1.0.3: Set the button's attributedTitle to a single attributed
        /// string built by concatenating per-color segments. This keeps the
        /// visual output identical to the 4-item layout (same colors, same
        /// order, same spacing) but occupies only ONE menu-bar slot, so
        /// macOS 26 can't rotate/evict individual metrics.
        pub unsafe fn set_multi_colored_title(&self, segments: &[ColoredSegment]) {
            let font: *mut Object =
                msg_send![class!(NSFont), menuBarFontOfSize: 0.0_f64];

            // Empty NSMutableAttributedString we'll append segments into.
            let mutable_str: *mut Object =
                msg_send![class!(NSMutableAttributedString), alloc];
            let mutable_str: *mut Object = msg_send![mutable_str, init];

            // Attribute keys (constant across segments)
            let ck = CString::new("NSColor").unwrap();
            let color_key: *mut Object =
                msg_send![class!(NSString), stringWithUTF8String: ck.as_ptr()];
            let fk = CString::new("NSFont").unwrap();
            let font_key: *mut Object =
                msg_send![class!(NSString), stringWithUTF8String: fk.as_ptr()];

            for seg in segments {
                let cstr = CString::new(seg.text.as_str()).unwrap_or_default();
                let ns_text: *mut Object =
                    msg_send![class!(NSString), stringWithUTF8String: cstr.as_ptr()];
                let color: *mut Object = msg_send![class!(NSColor),
                    colorWithSRGBRed: seg.r green: seg.g blue: seg.b alpha: 1.0_f64];

                let keys: [*mut Object; 2] = [color_key, font_key];
                let vals: [*mut Object; 2] = [color, font];
                let dict: *mut Object = msg_send![class!(NSDictionary),
                    dictionaryWithObjects: vals.as_ptr()
                    forKeys: keys.as_ptr()
                    count: 2_usize];

                let sub: *mut Object = msg_send![class!(NSAttributedString), alloc];
                let sub: *mut Object =
                    msg_send![sub, initWithString: ns_text attributes: dict];
                let _: () = msg_send![mutable_str, appendAttributedString: sub];
                let _: () = msg_send![sub, release];
            }

            let button: *mut Object = msg_send![self.ptr, button];
            let _: () = msg_send![button, setAttributedTitle: mutable_str];
            let _: () = msg_send![mutable_str, release];
        }
    }
}


// ─── Metrics Thread ───────────────────────────────────────────────────────────

fn start_metrics_thread(
    metrics: Arc<Mutex<Metrics>>,
    gpu: Arc<Mutex<Option<f32>>>,
    settings: Arc<Mutex<Settings>>,
    app: AppHandle,
    combined_tray: Arc<colored_tray::NativeStatusItem>,
) {
    std::thread::spawn(move || {
        let mut sys = System::new_all();
        let mut nets = Networks::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();

        sys.refresh_cpu_usage();
        std::thread::sleep(Duration::from_millis(1000));

        // v1.0.3: single-item model. Only one cache needed:
        //  - last_composed: full attributed-title text, so we only push to
        //    AppKit when the user-visible text actually changes. At 1 Hz
        //    with integer-percent granularity, this skips most ticks.
        let mut last_composed: Option<String> = None;

        loop {
            let (rate_ms, mode) = {
                let s = settings.lock().unwrap();
                (s.refresh_rate_ms, s.mode.clone())
            };

            sys.refresh_cpu_usage();
            sys.refresh_memory();
            nets.refresh();
            disks.refresh_list();
            // v1.0.3 FIX-2: pass `true` for remove_dead_processes so
            // finished processes don't linger with stale cpu_usage
            // values and dominate the "top process" pick.
            sys.refresh_processes_specifics(
                ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::new().with_cpu(),
            );

            let cpu = sys.global_cpu_usage();
            let mem_used  = sys.used_memory()  / 1_048_576;
            let mem_total = sys.total_memory() / 1_048_576;
            let mem_pct = if mem_total > 0 {
                sys.used_memory() as f32 / sys.total_memory() as f32 * 100.0
            } else { 0.0 };
            let swap_used  = sys.used_swap()  / 1_048_576;
            let swap_total = sys.total_swap() / 1_048_576;

            // v1.0.3 FIX-1: sysinfo's `received()` / `transmitted()` return
            // **bytes since the last refresh**, not a per-second rate. To
            // label the value "KB/s" honestly we have to divide by the
            // refresh interval, otherwise a 5-second refresh over-reports
            // the rate by 5×.
            let interval_s = (rate_ms as f64 / 1000.0).max(0.001);
            let rx_kbps: f64 = nets.iter().map(|(_, n)| n.received()    as f64).sum::<f64>() / 1024.0 / interval_s;
            let tx_kbps: f64 = nets.iter().map(|(_, n)| n.transmitted() as f64).sum::<f64>() / 1024.0 / interval_s;
            let gpu_usage = *gpu.lock().unwrap();

            // Disk: only the boot volume (mounted at "/")
            // v1.0.3 FIX-5: macOS Finder reports disk capacity in decimal
            // GB (10^9 bytes). Use the same so our 926 GB matches what
            // the user sees in Disk Utility / About This Mac.
            let mut dsk_total: u64 = 0;
            let mut dsk_available: u64 = 0;
            for disk in disks.iter() {
                if disk.mount_point() == std::path::Path::new("/") {
                    dsk_total = disk.total_space();
                    dsk_available = disk.available_space();
                    break;
                }
            }
            let dsk_used = dsk_total.saturating_sub(dsk_available);
            let dsk_total_gb = dsk_total as f64 / 1_000_000_000.0;
            let dsk_used_gb  = dsk_used  as f64 / 1_000_000_000.0;
            let dsk_pct = if dsk_total > 0 {
                dsk_used as f32 / dsk_total as f32 * 100.0
            } else { 0.0 };

            // v1.0.3 FIX-3: filter processes on the *normalized* (0-100)
            // value, not the raw (0 to 100×cores) value. `>= 0.5` means
            // "at least 0.5% of total system CPU" — anything less noisy
            // isn't worth showing as the top hog.
            let cpu_count = sys.cpus().len() as f32;
            let min_raw = 0.5 * cpu_count; // raw threshold equivalent to 0.5% normalized
            let top = sys.processes().values()
                .filter(|p| p.cpu_usage() >= min_raw)
                .max_by(|a, b| a.cpu_usage().partial_cmp(&b.cpu_usage())
                    .unwrap_or(std::cmp::Ordering::Equal));
            let (top_name, top_cpu) = top.map(|p| {
                let name = p.name().to_string_lossy().to_string();
                let pct  = (p.cpu_usage() / cpu_count).min(100.0);
                (name, pct)
            }).unwrap_or_default();

            let snapshot = Metrics {
                cpu_usage: cpu, mem_used_mb: mem_used, mem_total_mb: mem_total,
                mem_percent: mem_pct, swap_used_mb: swap_used, swap_total_mb: swap_total,
                net_rx_kbps: rx_kbps, net_tx_kbps: tx_kbps, gpu_usage,
                disk_used_gb: dsk_used_gb, disk_total_gb: dsk_total_gb, disk_percent: dsk_pct,
                top_process_name: top_name, top_process_cpu: top_cpu,
            };

            let show_std = mode == "standard" || mode == "full";
            let show_all = mode == "full";
            // v1.0.3 FIX-4: round instead of truncate so the menu-bar
            // text and the dashboard (which uses toFixed(0)) always
            // agree. Previously 8.9% rendered as "CPU 8" in the bar and
            // "9%" on the dashboard.
            let cpu_val = cpu.round() as u32;
            let mem_val = mem_pct.round() as u32;
            let dsk_val = dsk_pct.round() as u32;
            let gpu_val = gpu_usage.map(|g| g.round() as u32);

            // ── v1.0.3: Build a single combined attributed title for ONE
            // NSStatusItem. Prior versions used 4 separate status items
            // per metric, which macOS 26 Control Center rotates/evicts
            // under menu-bar space pressure. One item cannot be rotated.
            //
            // Layout (all in one item):
            //   ▄▅▇▁  CPU   8%  RAM  53%  DSK  88%  GPU   0%
            //   └┬─┘  └───────┬────────────────────────────┘
            //  hot-pink Unicode    fixed-width colored labels
            //  bar chart prefix    (:>3 padded percentages)
            //
            // The Unicode block chars replace the previous separate Tauri-
            // managed bar-chart tray icon, which macOS 26 was evicting
            // because it was the leftmost of 2 status items under a
            // narrow menu bar.
            let mut segments: Vec<colored_tray::ColoredSegment> = Vec::with_capacity(5);

            // Hot-pink Unicode bar chart, heights driven by live metrics.
            // Colors: #FF2D55 → sRGB (1.0, 0.176, 0.333).
            let gpu_for_bars = gpu_val.unwrap_or(0);
            segments.push(colored_tray::ColoredSegment {
                text: format!(
                    "{}{}{}{} ",
                    bar_char(cpu_val),
                    bar_char(mem_val),
                    bar_char(dsk_val),
                    bar_char(gpu_for_bars),
                ),
                r: 1.0, g: 0.176, b: 0.333,
            });

            // Variable-width (no :>3 padding) to keep each label slim,
            // plus 3-space gaps between labels (1 trailing + 2 leading)
            // so they breathe without bloating the bar.
            segments.push(colored_tray::ColoredSegment {
                text: format!("   CPU {}% ", cpu_val),
                r: 0.29, g: 0.56, b: 0.85,
            });
            if show_std {
                segments.push(colored_tray::ColoredSegment {
                    text: format!("   RAM {}% ", mem_val),
                    r: 0.20, g: 0.78, b: 0.35,
                });
            }
            if show_all {
                segments.push(colored_tray::ColoredSegment {
                    text: format!("   DSK {}% ", dsk_val),
                    r: 1.0, g: 0.58, b: 0.0,
                });
                if let Some(g) = gpu_val {
                    segments.push(colored_tray::ColoredSegment {
                        text: format!("   GPU {}% ", g),
                        r: 0.69, g: 0.32, b: 0.87,
                    });
                }
            }

            // Composed plain string for cache comparison (color-agnostic —
            // colors per segment are fixed, so text-equality is enough).
            let composed: String = segments.iter().map(|s| s.text.as_str()).collect();

            if last_composed.as_deref() != Some(composed.as_str()) {
                let item = Arc::clone(&combined_tray);
                let segs = segments;
                let _ = app.run_on_main_thread(move || unsafe {
                    item.set_multi_colored_title(&segs);
                });
                last_composed = Some(composed);
            }

            *metrics.lock().unwrap() = snapshot;
            std::thread::sleep(Duration::from_millis(rate_ms));
        }
    });
}

// ─── Panel Helpers ────────────────────────────────────────────────────────────

fn show_panel(app: &AppHandle, _cursor_x_phys: Option<f64>) {
    let Some(win) = app.get_webview_window("panel") else { return };
    if let Ok(Some(monitor)) = win.primary_monitor() {
        let scale     = monitor.scale_factor();
        let logical_w = monitor.size().width as f64 / scale;
        let panel_w   = 340.0_f64;

        // v1.0.3: Tauri tray is gone, so there's no `rect()` to anchor
        // against. Anchor the panel at the right edge of the screen where
        // menu-bar status items live. Close enough to the click target
        // without requiring raw AppKit queries.
        let x = (logical_w - panel_w - 20.0).max(4.0);
        let _ = win.set_position(LogicalPosition::new(x, 28.0));
    }
    let _ = win.show();
    let _ = win.set_focus();
}

fn toggle_panel(app: &AppHandle, cursor_x_phys: Option<f64>) {
    if let Some(win) = app.get_webview_window("panel") {
        if win.is_visible().unwrap_or(false) { let _ = win.hide(); }
        else { show_panel(app, cursor_x_phys); }
    }
}

// ─── Entry Point ─────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
            // Second instance attempted — do nothing, first instance stays running
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let settings_val    = load_settings(app.handle());
            let settings_shared = Arc::new(Mutex::new(settings_val.clone()));
            let metrics_shared  = Arc::new(Mutex::new(Metrics::default()));
            let gpu_shared: Arc<Mutex<Option<f32>>> = Arc::new(Mutex::new(None));

            // Initial autostart state
            let autostart_now = {
                use tauri_plugin_autostart::ManagerExt;
                app.autolaunch().is_enabled().unwrap_or(false)
            };

            // ── v1.0.3: Single combined colored-text NSStatusItem. The
            // v1.0.2 layout used 4 separate NSStatusItems (one per metric)
            // plus a Tauri-managed tray icon. macOS 26 rotated/evicted
            // them under menu-bar space pressure. One item holds it all
            // now: a hot-pink Unicode bar prefix + colored labels + the
            // native NSMenu and click handler.
            let combined_tray = Arc::new(unsafe {
                colored_tray::NativeStatusItem::new()
            });

            colored_tray::init_menu_context(
                app.handle().clone(),
                Arc::clone(&settings_shared),
                Arc::clone(&combined_tray),
            );

            // Install left/right-click discrimination on the NSStatusBarButton
            // and attach the initial NSMenu via swap_menu (which tracks the
            // pointer so subsequent rebuilds can release the previous menu).
            unsafe {
                let target = colored_tray::menu_target_instance();
                combined_tray.install_click_handler(target);

                let ns_menu = colored_tray::build_native_menu(
                    &settings_val, autostart_now,
                );
                if let Some(ctx) = colored_tray::menu_context() {
                    colored_tray::swap_menu(ctx, ns_menu);
                }
            }

            // Panel window
            let _panel = WebviewWindowBuilder::new(
                app, "panel", WebviewUrl::App("index.html".into()),
            )
            .title("Rise PulseBar")
            .inner_size(340.0, 640.0)
            .min_inner_size(340.0, 400.0)
            .max_inner_size(340.0, 640.0)
            .decorations(false)
            .always_on_top(true)
            .visible(false)
            .resizable(false)
            .skip_taskbar(true)
            .shadow(true)
            .build()?;

            // Panel closes via X button or hide_panel command — no auto-hide on blur

            start_gpu_thread(Arc::clone(&gpu_shared), Arc::clone(&settings_shared));
            start_metrics_thread(
                Arc::clone(&metrics_shared),
                Arc::clone(&gpu_shared),
                Arc::clone(&settings_shared),
                app.handle().clone(),
                combined_tray,
            );

            app.manage(MetricsState(metrics_shared));
            app.manage(SettingsState(settings_shared));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_metrics, hide_panel,
            get_settings, set_settings,
            get_autostart_enabled, set_autostart_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error running Rise PulseBar");
}
