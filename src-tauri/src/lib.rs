use std::sync::{Arc, Mutex};
use std::time::Duration;
use sysinfo::{Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::menu::{MenuBuilder, MenuItem, Submenu, SubmenuBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, LogicalPosition, Manager, State, WebviewUrl, WebviewWindowBuilder};

const APP_VERSION: &str = "1.0.2";

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

/// P3 — About: show a native macOS message dialog (no extra window)
#[tauri::command]
fn show_about(app: AppHandle) {
    if let Some(win) = app.get_webview_window("panel") {
        let _ = win.eval(&format!(
            r#"window.__pulsebarAbout = {{
  name: 'Rise PulseBar',
  version: 'v{APP_VERSION}',
  author: 'Rise Studio Labs',
  tagline: 'Ultra-lightweight macOS system monitor'
}}; window.dispatchEvent(new CustomEvent('show-about'));"#
        ));
    }
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
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

fn start_gpu_thread(shared: Arc<Mutex<Option<f32>>>) {
    std::thread::spawn(move || loop {
        let gpu = std::process::Command::new("ioreg")
            .args(["-r", "-c", "IOAccelerator", "-l"])
            .output()
            .ok()
            .and_then(|o| parse_gpu_pct(&String::from_utf8_lossy(&o.stdout)));
        *shared.lock().unwrap() = gpu;
        std::thread::sleep(Duration::from_millis(1000));
    });
}

// ─── Live animated bar chart icon ────────────────────────────────────────────
//
// 4 bars (CPU, RAM, DSK, GPU) that move in real-time. Hot pink.

fn render_live_bars(cpu: u32, ram: u32, dsk: u32, gpu: u32) -> tauri::image::Image<'static> {
    let canvas: u32 = 44;
    let mut rgba = vec![0u8; (canvas * canvas * 4) as usize];

    let bar_w: u32 = 6;
    let gap: u32 = 3;
    let total_w = 4 * bar_w + 3 * gap;
    let x_start = (canvas - total_w) / 2;
    let bottom: u32 = 40;
    let top_limit: u32 = 4;
    let max_h = bottom - top_limit;

    // Hot pink
    let (r, g, b): (u8, u8, u8) = (255, 45, 85);

    let bars = [cpu, ram, dsk, gpu];
    for (i, &pct) in bars.iter().enumerate() {
        let x0 = x_start + i as u32 * (bar_w + gap);
        let fill_h = (pct.min(100) as f32 / 100.0 * max_h as f32).round() as u32;
        let fill_h = fill_h.max(2);
        let bar_top = bottom - fill_h;

        for py in bar_top..bottom {
            for px in x0..(x0 + bar_w) {
                let idx = ((py * canvas + px) * 4) as usize;
                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = 240;
            }
        }
    }

    tauri::image::Image::new_owned(rgba, canvas, canvas)
}

// ─── Native macOS colored tray text ──────────────────────────────────────────
//
// Uses Objective-C runtime to create NSStatusItems with colored attributed text.
// Each metric gets its own status item with a unique color.

#[cfg(target_os = "macos")]
mod colored_tray {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::CString;

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

        pub unsafe fn set_colored_title(&self, text: &str, r: f64, g: f64, b: f64) {
            let cstr = CString::new(text).unwrap_or_default();
            let ns_text: *mut Object =
                msg_send![class!(NSString), stringWithUTF8String: cstr.as_ptr()];

            let color: *mut Object = msg_send![class!(NSColor),
                colorWithSRGBRed: r green: g blue: b alpha: 1.0_f64];
            let font: *mut Object =
                msg_send![class!(NSFont), menuBarFontOfSize: 0.0_f64];

            let ck = CString::new("NSColor").unwrap();
            let color_key: *mut Object =
                msg_send![class!(NSString), stringWithUTF8String: ck.as_ptr()];
            let fk = CString::new("NSFont").unwrap();
            let font_key: *mut Object =
                msg_send![class!(NSString), stringWithUTF8String: fk.as_ptr()];

            let keys: [*mut Object; 2] = [color_key, font_key];
            let vals: [*mut Object; 2] = [color, font];
            let dict: *mut Object = msg_send![class!(NSDictionary),
                dictionaryWithObjects: vals.as_ptr()
                forKeys: keys.as_ptr()
                count: 2_usize];

            let attr_str: *mut Object = msg_send![class!(NSAttributedString), alloc];
            let attr_str: *mut Object =
                msg_send![attr_str, initWithString: ns_text attributes: dict];

            let button: *mut Object = msg_send![self.ptr, button];
            let _: () = msg_send![button, setAttributedTitle: attr_str];
            let _: () = msg_send![attr_str, release];
        }

        pub unsafe fn set_visible(&self, visible: bool) {
            let length: f64 = if visible { -1.0 } else { 0.0 };
            let _: () = msg_send![self.ptr, setLength: length];
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

    pub struct MetricTrays {
        pub cpu: NativeStatusItem,
        pub ram: NativeStatusItem,
        pub dsk: NativeStatusItem,
        pub gpu: NativeStatusItem,
    }
}


// ─── Metrics Thread ───────────────────────────────────────────────────────────

fn start_metrics_thread(
    metrics: Arc<Mutex<Metrics>>,
    gpu: Arc<Mutex<Option<f32>>>,
    settings: Arc<Mutex<Settings>>,
    app: AppHandle,
    native_trays: Arc<colored_tray::MetricTrays>,
) {
    std::thread::spawn(move || {
        let mut sys = System::new_all();
        let mut nets = Networks::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();

        sys.refresh_cpu_usage();
        std::thread::sleep(Duration::from_millis(1000));

        loop {
            let (rate_ms, mode) = {
                let s = settings.lock().unwrap();
                (s.refresh_rate_ms, s.mode.clone())
            };

            sys.refresh_cpu_usage();
            sys.refresh_memory();
            nets.refresh();
            disks.refresh_list();
            sys.refresh_processes_specifics(
                ProcessesToUpdate::All,
                false,
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
            let rx_kbps: f64 = nets.iter().map(|(_, n)| n.received()    as f64).sum::<f64>() / 1024.0;
            let tx_kbps: f64 = nets.iter().map(|(_, n)| n.transmitted() as f64).sum::<f64>() / 1024.0;
            let gpu_usage = *gpu.lock().unwrap();

            // Disk: only the boot volume (mounted at "/")
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
            let dsk_total_gb = dsk_total as f64 / 1_073_741_824.0;
            let dsk_used_gb  = dsk_used  as f64 / 1_073_741_824.0;
            let dsk_pct = if dsk_total > 0 {
                dsk_used as f32 / dsk_total as f32 * 100.0
            } else { 0.0 };

            let cpu_count = sys.cpus().len() as f32;
            let top = sys.processes().values()
                .filter(|p| p.cpu_usage() > 0.1)
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

            // ── Update live bar chart icon ────────────────────────────
            if let Some(tray) = app.tray_by_id("main") {
                let _ = tray.set_icon(Some(render_live_bars(
                    cpu as u32, mem_pct as u32, dsk_pct as u32,
                    gpu_usage.unwrap_or(0.0) as u32)));
            }

            // ── Update native colored tray items ─────────────────────
            let show_std = mode == "standard" || mode == "full";
            let show_all = mode == "full";
            let cpu_val = cpu as u32;
            let mem_val = mem_pct as u32;
            let dsk_val = dsk_pct as u32;
            let gpu_val = gpu_usage.map(|g| g as u32);

            let trays = Arc::clone(&native_trays);
            let _ = app.run_on_main_thread(move || unsafe {
                // CPU — sky blue — always visible
                trays.cpu.set_colored_title(
                    &format!("CPU {}% ", cpu_val), 0.29, 0.56, 0.85);

                // RAM — green
                trays.ram.set_visible(show_std);
                if show_std {
                    trays.ram.set_colored_title(
                        &format!(" RAM {}% ", mem_val), 0.20, 0.78, 0.35);
                }

                // DSK — orange
                trays.dsk.set_visible(show_all);
                if show_all {
                    trays.dsk.set_colored_title(
                        &format!(" DSK {}% ", dsk_val), 1.0, 0.58, 0.0);
                }

                // GPU — purple
                let gpu_vis = show_all && gpu_val.is_some();
                trays.gpu.set_visible(gpu_vis);
                if let Some(g) = gpu_val {
                    if gpu_vis {
                        trays.gpu.set_colored_title(
                            &format!(" GPU {}% ", g), 0.69, 0.32, 0.87);
                    }
                }
            });

            *metrics.lock().unwrap() = snapshot;
            std::thread::sleep(Duration::from_millis(rate_ms));
        }
    });
}

// ─── Panel Helpers ────────────────────────────────────────────────────────────

fn show_panel(app: &AppHandle, cursor_x_phys: Option<f64>) {
    let Some(win) = app.get_webview_window("panel") else { return };
    if let Ok(Some(monitor)) = win.primary_monitor() {
        let scale     = monitor.scale_factor();
        let logical_w = monitor.size().width as f64 / scale;
        let panel_w   = 340.0_f64;

        // Use click position, or fall back to the tray icon's screen position
        let phys_x = cursor_x_phys.or_else(|| {
            app.tray_by_id("main")
                .and_then(|t| t.rect().ok().flatten())
                .map(|r| {
                    let px = match r.position {
                        tauri::Position::Physical(p) => p.x as f64,
                        tauri::Position::Logical(p) => p.x * scale,
                    };
                    let sw = match r.size {
                        tauri::Size::Physical(s) => s.width as f64,
                        tauri::Size::Logical(s) => s.width * scale,
                    };
                    px + sw / 2.0
                })
        });

        let x = match phys_x {
            Some(px) => (px / scale - panel_w / 2.0).max(4.0).min(logical_w - panel_w - 4.0),
            None     => 4.0,
        };
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

// ─── P2 — Tray menu builder ───────────────────────────────────────────────────

fn build_tray_menu(
    app: &AppHandle,
    settings: &Settings,
    autostart: bool,
) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {

    // Mode submenu — active item gets a checkmark prefix
    let mode_min = MenuItem::with_id(app, "mode_minimal",
        if settings.mode == "minimal" { "✓  Minimal" } else { "   Minimal" },
        true, None::<&str>)?;
    let mode_std = MenuItem::with_id(app, "mode_standard",
        if settings.mode == "standard" { "✓  Standard" } else { "   Standard" },
        true, None::<&str>)?;
    let mode_full = MenuItem::with_id(app, "mode_full",
        if settings.mode == "full" { "✓  Full" } else { "   Full" },
        true, None::<&str>)?;
    let mode_sub: Submenu<tauri::Wry> = SubmenuBuilder::new(app, "Mode")
        .item(&mode_min).item(&mode_std).item(&mode_full)
        .build()?;

    // Refresh rate submenu
    let rate_1 = MenuItem::with_id(app, "rate_1000",
        if settings.refresh_rate_ms == 1000 { "✓  1 second" } else { "   1 second" },
        true, None::<&str>)?;
    let rate_2 = MenuItem::with_id(app, "rate_2000",
        if settings.refresh_rate_ms == 2000 { "✓  2 seconds" } else { "   2 seconds" },
        true, None::<&str>)?;
    let rate_5 = MenuItem::with_id(app, "rate_5000",
        if settings.refresh_rate_ms == 5000 { "✓  5 seconds" } else { "   5 seconds" },
        true, None::<&str>)?;
    let rate_sub: Submenu<tauri::Wry> = SubmenuBuilder::new(app, "Refresh Rate")
        .item(&rate_1).item(&rate_2).item(&rate_5)
        .build()?;

    let open_item   = MenuItem::with_id(app, "open",        "Open Dashboard", true, None::<&str>)?;
    let login_item  = MenuItem::with_id(app, "launch_login",
        if autostart { "✓  Launch at Login" } else { "   Launch at Login" },
        true, None::<&str>)?;
    let about_item  = MenuItem::with_id(app, "about",       "About Rise PulseBar…", true, None::<&str>)?;
    let quit_item   = MenuItem::with_id(app, "quit",        "Quit Rise PulseBar", true, None::<&str>)?;

    let menu = MenuBuilder::new(app)
        .item(&open_item)
        .separator()
        .item(&mode_sub)
        .item(&rate_sub)
        .separator()
        .item(&login_item)
        .separator()
        .item(&about_item)
        .separator()
        .item(&quit_item)
        .build()?;

    Ok(menu)
}

// ─── Entry Point ─────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
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

            let initial_menu = build_tray_menu(app.handle(), &settings_val, autostart_now)?;

            let settings_for_handler = Arc::clone(&settings_shared);

            // ── Native colored metric trays ──────────────────────────────
            // Created in reverse order so macOS lays them out left→right
            let native_trays = Arc::new(unsafe {
                colored_tray::MetricTrays {
                    gpu: colored_tray::NativeStatusItem::new(),
                    dsk: colored_tray::NativeStatusItem::new(),
                    ram: colored_tray::NativeStatusItem::new(),
                    cpu: colored_tray::NativeStatusItem::new(),
                }
            });

            // ── Main logo tray (leftmost) ────────────────────────────────
            let _tray = TrayIconBuilder::with_id("main")
                .icon(render_live_bars(0, 0, 0, 0))
                .tooltip("Rise PulseBar")
                .menu(&initial_menu)
                .on_menu_event(move |app, event| {
                    let id = event.id().0.as_str();

                    // Helper: rebuild menu after state change
                    let rebuild = |app: &AppHandle| {
                        use tauri_plugin_autostart::ManagerExt;
                        let s  = settings_for_handler.lock().unwrap().clone();
                        let al = app.autolaunch().is_enabled().unwrap_or(false);
                        if let Ok(menu) = build_tray_menu(app, &s, al) {
                            if let Some(tray) = app.tray_by_id("main") {
                                let _ = tray.set_menu(Some(menu));
                            }
                        }
                    };

                    match id {
                        "open" => toggle_panel(app, None),

                        // Mode
                        "mode_minimal" | "mode_standard" | "mode_full" => {
                            let new_mode = id.strip_prefix("mode_").unwrap_or("standard").to_string();
                            let new_settings = {
                                let mut s = settings_for_handler.lock().unwrap();
                                s.mode = new_mode;
                                s.clone()
                            };
                            save_settings(app, &new_settings);
                            // Notify frontend
                            if let Some(win) = app.get_webview_window("panel") {
                                let _ = win.eval(&format!(
                                    "window.dispatchEvent(new CustomEvent('settings-changed',{{detail:{{mode:'{}'}}}}))",
                                    new_settings.mode
                                ));
                            }
                            rebuild(app);
                        }

                        // Refresh rate
                        "rate_1000" | "rate_2000" | "rate_5000" => {
                            let ms: u64 = id.strip_prefix("rate_").unwrap_or("1000").parse().unwrap_or(1000);
                            let new_settings = {
                                let mut s = settings_for_handler.lock().unwrap();
                                s.refresh_rate_ms = ms;
                                s.clone()
                            };
                            save_settings(app, &new_settings);
                            if let Some(win) = app.get_webview_window("panel") {
                                let _ = win.eval(&format!(
                                    "window.dispatchEvent(new CustomEvent('settings-changed',{{detail:{{refresh_rate_ms:{ms}}}}}))"
                                ));
                            }
                            rebuild(app);
                        }

                        // Launch at login
                        "launch_login" => {
                            use tauri_plugin_autostart::ManagerExt;
                            let al = app.autolaunch();
                            if al.is_enabled().unwrap_or(false) {
                                let _ = al.disable();
                            } else {
                                let _ = al.enable();
                            }
                            rebuild(app);
                        }

                        // P3 — About
                        "about" => {
                            if let Some(win) = app.get_webview_window("panel") {
                                show_panel(app, None);
                                let _ = win.eval(
                                    "window.dispatchEvent(new CustomEvent('show-about'));"
                                );
                            }
                        }

                        "quit" => app.exit(0),
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray: &TrayIcon, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        position,
                        ..
                    } = event {
                        toggle_panel(tray.app_handle(), Some(position.x));
                    }
                })
                .build(app)?;

            // Panel window
            let panel = WebviewWindowBuilder::new(
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

            start_gpu_thread(Arc::clone(&gpu_shared));
            start_metrics_thread(
                Arc::clone(&metrics_shared),
                Arc::clone(&gpu_shared),
                Arc::clone(&settings_shared),
                app.handle().clone(),
                native_trays,
            );

            app.manage(MetricsState(metrics_shared));
            app.manage(SettingsState(settings_shared));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_metrics, hide_panel,
            get_settings, set_settings,
            get_autostart_enabled, set_autostart_enabled,
            show_about, quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error running Rise PulseBar");
}
