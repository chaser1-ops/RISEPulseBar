'use strict';

// ─── Tauri bridge ─────────────────────────────────────────────────────────────
const invoke = (...a) => window.__TAURI__.core.invoke(...a);

// ─── History buffers ──────────────────────────────────────────────────────────
const MAX_H   = 60;
const cpuHist = [];
const gpuHist = [];

// ─── State ────────────────────────────────────────────────────────────────────
let cfg        = null;
let pollTimer  = null;

// ─── DOM refs ─────────────────────────────────────────────────────────────────
const $ = id => document.getElementById(id);
const cpuValue     = $('cpuValue');
const cpuBar       = $('cpuBar');
const cpuSpark     = $('cpuSpark');
const topProcInfo  = $('topProcInfo');
const gpuCard      = $('gpuCard');
const gpuValue     = $('gpuValue');
const gpuBar       = $('gpuBar');
const gpuSpark     = $('gpuSpark');
const memValue     = $('memValue');
const memBar       = $('memBar');
const memSub       = $('memSub');
const swapCard     = $('swapCard');
const swapValue    = $('swapValue');
const swapBar      = $('swapBar');
const swapSub      = $('swapSub');
const netCard      = $('netCard');
const netRx        = $('netRx');
const netTx        = $('netTx');
const closeBtn     = $('closeBtn');
const autoTog      = $('autostartToggle');
const modeCtrl     = $('modeCtrl');
const rateCtrl     = $('rateCtrl');
const aboutOverlay = $('aboutOverlay');
const aboutClose   = $('aboutClose');

// ─── Formatting ───────────────────────────────────────────────────────────────
function fmtSpeed(kbps) {
  if (kbps >= 1024) return (kbps / 1024).toFixed(1) + ' MB/s';
  if (kbps >= 1)    return kbps.toFixed(0)           + ' KB/s';
  return (kbps * 1024).toFixed(0) + ' B/s';
}
function fmtMB(mb) {
  return mb >= 1024 ? (mb / 1024).toFixed(1) + ' GB' : mb + ' MB';
}
function trunc(s, n = 22) {
  return s.length > n ? s.slice(0, n - 1) + '…' : s;
}

// ─── Bar helpers ──────────────────────────────────────────────────────────────
function setBar(el, pct) {
  el.style.width = Math.min(100, Math.max(0, pct)).toFixed(1) + '%';
}
function colorBar(el, pct) {
  el.classList.toggle('warn', pct >= 70 && pct < 90);
  el.classList.toggle('crit', pct >= 90);
}

// ─── Sparkline renderer (DPR-aware, no external libs) ────────────────────────
const DPR = window.devicePixelRatio || 1;

function sizeCanvas(canvas) {
  const w = Math.floor(canvas.parentElement.getBoundingClientRect().width) || 316;
  const h = 24;
  if (canvas.width !== w * DPR || canvas.height !== h * DPR) {
    canvas.width  = w * DPR;
    canvas.height = h * DPR;
    canvas.style.width  = w + 'px';
    canvas.style.height = h + 'px';
  }
  return { w: canvas.width, h: canvas.height };
}

function drawSpark(canvas, hist, hex) {
  const { w, h } = sizeCanvas(canvas);
  const ctx = canvas.getContext('2d');
  ctx.clearRect(0, 0, w, h);
  if (hist.length < 2) return;

  const padded = Array(MAX_H - hist.length).fill(0).concat(hist);
  const step   = w / (MAX_H - 1);
  const pts    = padded.map((v, i) => ({
    x: i * step,
    y: h - (Math.min(v, 100) / 100) * (h * 0.88) - h * 0.06,
  }));

  // Gradient fill
  ctx.beginPath();
  ctx.moveTo(0, h);
  pts.forEach(p => ctx.lineTo(p.x, p.y));
  ctx.lineTo(w, h);
  ctx.closePath();
  const g = ctx.createLinearGradient(0, 0, 0, h);
  g.addColorStop(0, hex + '55');
  g.addColorStop(1, hex + '08');
  ctx.fillStyle = g;
  ctx.fill();

  // Line
  ctx.beginPath();
  pts.forEach((p, i) => i === 0 ? ctx.moveTo(p.x, p.y) : ctx.lineTo(p.x, p.y));
  ctx.strokeStyle = hex;
  ctx.lineWidth   = 1.5 * DPR;
  ctx.lineJoin    = 'round';
  ctx.lineCap     = 'round';
  ctx.stroke();
}

function pushHist(arr, v) { arr.push(v); if (arr.length > MAX_H) arr.shift(); }

// ─── Mode-based card visibility ───────────────────────────────────────────────
function applyMode(mode) {
  const min = mode === 'minimal';
  swapCard.style.display = min ? 'none' : '';
  netCard.style.display  = min ? 'none' : '';
}

// ─── Render one snapshot ──────────────────────────────────────────────────────
function render(m) {
  // CPU
  cpuValue.textContent = m.cpu_usage.toFixed(0) + '%';
  setBar(cpuBar, m.cpu_usage);
  colorBar(cpuBar, m.cpu_usage);
  pushHist(cpuHist, m.cpu_usage);
  drawSpark(cpuSpark, cpuHist, '#7b6ef6');

  // Top process
  topProcInfo.textContent = m.top_process_name
    ? 'Top: ' + trunc(m.top_process_name) + '  ' + m.top_process_cpu.toFixed(1) + '%'
    : 'Top: —';

  // GPU — show card only when data + settings allow
  const showGpu = m.gpu_usage != null && cfg?.show_gpu !== false;
  gpuCard.classList.toggle('card-hidden', !showGpu);
  if (showGpu) {
    gpuValue.textContent = m.gpu_usage.toFixed(0) + '%';
    setBar(gpuBar, m.gpu_usage);
    colorBar(gpuBar, m.gpu_usage);
    pushHist(gpuHist, m.gpu_usage);
    drawSpark(gpuSpark, gpuHist, '#a78bfa');
  }

  // Memory
  memValue.textContent = m.mem_percent.toFixed(0) + '%';
  setBar(memBar, m.mem_percent);
  colorBar(memBar, m.mem_percent);
  memSub.textContent = fmtMB(m.mem_used_mb) + ' / ' + fmtMB(m.mem_total_mb);

  // Swap
  if (cfg?.show_swap !== false) {
    const sp = m.swap_total_mb > 0 ? (m.swap_used_mb / m.swap_total_mb) * 100 : 0;
    swapValue.textContent = fmtMB(m.swap_used_mb);
    setBar(swapBar, sp);
    swapSub.textContent = fmtMB(m.swap_used_mb) + ' / ' + fmtMB(m.swap_total_mb);
  }

  // Network
  if (cfg?.show_network !== false) {
    netRx.textContent = fmtSpeed(m.net_rx_kbps);
    netTx.textContent = fmtSpeed(m.net_tx_kbps);
  }
}

// ─── Polling ──────────────────────────────────────────────────────────────────
async function poll() {
  try { render(await invoke('get_metrics')); }
  catch { /* backend not ready yet — skip silently */ }
}

function startPolling(ms) {
  clearInterval(pollTimer);
  poll();
  pollTimer = setInterval(poll, ms || 1000);
}

// ─── Settings ─────────────────────────────────────────────────────────────────
function syncSegButtons(ctrl, val) {
  ctrl.querySelectorAll('.seg-btn').forEach(b =>
    b.classList.toggle('active', b.dataset.val === String(val)));
}

function applySettings(s) {
  cfg = s;
  syncSegButtons(modeCtrl, s.mode);
  syncSegButtons(rateCtrl, s.refresh_rate_ms);
  applyMode(s.mode);
  startPolling(s.refresh_rate_ms);
}

async function loadSettings() {
  try {
    applySettings(await invoke('get_settings'));
  } catch {
    applySettings({ mode: 'standard', refresh_rate_ms: 1000,
                    show_gpu: true, show_network: true, show_swap: true });
  }
}

async function saveSetting(patch) {
  cfg = { ...cfg, ...patch };
  try { await invoke('set_settings', { settings: cfg }); } catch { /* local fallback */ }
  applySettings(cfg);
}

// Panel-side segmented controls
modeCtrl.addEventListener('click', e => {
  const b = e.target.closest('.seg-btn');
  if (b) saveSetting({ mode: b.dataset.val });
});
rateCtrl.addEventListener('click', e => {
  const b = e.target.closest('.seg-btn');
  if (b) saveSetting({ refresh_rate_ms: Number(b.dataset.val) });
});

// ─── Tray → panel sync (settings changed via right-click menu) ───────────────
// Rust fires this custom event via win.eval() when the tray menu changes settings
window.addEventListener('settings-changed', e => {
  if (!e.detail) return;
  const patch = {};
  if (e.detail.mode              != null) patch.mode             = e.detail.mode;
  if (e.detail.refresh_rate_ms   != null) patch.refresh_rate_ms  = e.detail.refresh_rate_ms;
  if (Object.keys(patch).length) saveSetting(patch);
});

// ─── P3 — About modal ─────────────────────────────────────────────────────────
function showAbout() {
  aboutOverlay.classList.remove('hidden');
}
function hideAbout() {
  aboutOverlay.classList.add('hidden');
}
aboutClose.addEventListener('click', hideAbout);
aboutOverlay.addEventListener('click', e => { if (e.target === aboutOverlay) hideAbout(); });
document.addEventListener('keydown', e => { if (e.key === 'Escape') hideAbout(); });

// Rust fires this via win.eval() from the "about" menu item and show_about command
window.addEventListener('show-about', showAbout);

// ─── Close button ─────────────────────────────────────────────────────────────
closeBtn.addEventListener('click', () =>
  invoke('hide_panel').catch(() =>
    window.__TAURI__.window.getCurrentWindow().hide()));

// ─── Autostart toggle ─────────────────────────────────────────────────────────
(async () => {
  try { autoTog.checked = await invoke('get_autostart_enabled'); } catch { /* ok */ }
})();

autoTog.addEventListener('change', async e => {
  try {
    await invoke('set_autostart_enabled', { enabled: e.target.checked });
  } catch {
    e.target.checked = !e.target.checked;
  }
});

// ─── Boot ─────────────────────────────────────────────────────────────────────
loadSettings();
