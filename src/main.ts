import { api } from "./ipc";
import {
  CONTENT_VERSION,
  UID_MAX_LENGTH,
  compatibleGameVersion,
  readiness,
  type Operation,
  type Scenario,
  type Section,
  type UiState,
} from "./model";
import { renderCustomize, renderTools } from "./panels";
import { escapeHtml as esc, icon } from "./ui";

const app = document.querySelector<HTMLDivElement>("#app")!;
const toast = document.querySelector<HTMLDivElement>("#toast")!;
const dialog = document.querySelector<HTMLDialogElement>("#confirm-dialog")!;
let state: UiState;
let section: Section = "home";
let error = "";
let toastTimer: number;
const navigation: { id: Section; label: string; number: string }[] = [
  { id: "home", label: "Beranda", number: "01" },
  { id: "customize", label: "Kustomisasi", number: "02" },
  { id: "tools", label: "Perawatan", number: "03" },
];
const logo = '<svg viewBox="0 0 32 32" fill="none" aria-hidden="true"><path d="M3 16c5-17 9 17 14 0s9 17 12 0M3 23c5-17 9 17 14 0s9 17 12 0" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>';
type ThemePreference = "light" | "dark" | "system";
const systemTheme = matchMedia("(prefers-color-scheme: dark)");
let themePreference = document.documentElement.dataset.themePreference as ThemePreference;
const themeOptions: { id: ThemePreference; label: string; icon: string }[] = [
  { id: "light", label: "Terang", icon: "sun" },
  { id: "dark", label: "Gelap", icon: "moon" },
  { id: "system", label: "Sistem", icon: "system" },
];

function applyTheme() {
  const theme = themePreference === "system"
    ? (systemTheme.matches ? "dark" : "light")
    : themePreference;
  document.documentElement.dataset.theme = theme;
  document.documentElement.dataset.themePreference = themePreference;
  document.querySelector<HTMLMetaElement>('meta[name="theme-color"]')!.content =
    theme === "dark" ? "#15151b" : "#f5f3ef";
  app.querySelectorAll<HTMLButtonElement>("[data-theme-choice]").forEach((button) => {
    button.setAttribute("aria-pressed", String(button.dataset.themeChoice === themePreference));
  });
}
systemTheme.addEventListener("change", () => {
  if (themePreference === "system") applyTheme();
});

function renderThemeControl() {
  return `<div class="theme-control" role="group" aria-label="Tema tampilan">${themeOptions.map((option) => `<button type="button" data-theme-choice="${option.id}" aria-label="Tema ${option.label.toLowerCase()}" title="${option.label}" aria-pressed="${themePreference === option.id}">${icon(option.icon)}</button>`).join("")}</div>`;
}

function notify(message: string) {
  clearTimeout(toastTimer);
  toast.textContent = message;
  toast.hidden = false;
  toastTimer = setTimeout(() => {
    toast.hidden = true;
  }, 5500);
}

function renderHome() {
  const d = state.dashboard;
  const ready = readiness(d);
  const disabled = state.busy || !d.platformSupported ? "disabled" : "";
  let primary: { action: string; label: string } = {
    action: "install",
    label:
      d.mode === "mock" ? "Pasang terjemahan" : "Unduh & pasang terjemahan",
  };
  if (!d.platformSupported)
    primary = { action: "refresh", label: "Pemasangan memerlukan Android" };
  else if (!d.game.installed)
    primary = { action: "setup-game", label: "Pasang Wuthering Waves Global" };
  else if (!d.shizuku.running)
    primary = { action: "setup-shizuku", label: "Buka Shizuku" };
  else if (!d.shizuku.granted)
    primary = { action: "request_permission", label: "Berikan izin Shizuku" };
  else if (d.mode === "native" && !d.release)
    primary = { action: "refresh", label: "Muat ulang rilis terbaru" };
  else if (!compatibleGameVersion(d, d.game.version))
    primary = { action: "tools", label: "Lihat kompatibilitas" };
  else if (!d.resource.ready)
    primary = {
      action: "launch_game",
      label: "Buka game untuk unduh resource",
    };
  else if (!ready.ready) primary = { action: "tools", label: "Buka Perawatan" };
  else if (
    d.patch.installed &&
    d.mode === "native" &&
    d.release?.tag !== d.patch.version
  )
    primary = { action: "install", label: "Pasang rilis terbaru" };
  else if (d.patch.installed)
    primary = { action: "launch_game", label: "Buka game" };
  const deviceRows = [
    {
      name: "Wuthering Waves",
      value: !d.platformSupported
        ? "Hanya tersedia di Android"
        : d.game.installed
          ? `Global · ${d.game.version}`
          : "Belum terpasang",
      ok: d.game.installed && compatibleGameVersion(d, d.game.version),
      icon: "game",
    },
    {
      name: "Shizuku",
      value: !d.platformSupported
        ? "Memerlukan Android"
        : !d.shizuku.running
          ? "Belum aktif"
          : d.shizuku.granted
            ? "Aktif · izin diberikan"
            : "Menunggu izin",
      ok: d.shizuku.running && d.shizuku.granted,
      icon: "shield",
    },
    {
      name: "Resource game",
      value: !d.platformSupported
        ? "Perlu akses Android"
        : d.resource.ready
          ? `Lengkap · ${d.resource.version}`
          : "Unduhan belum selesai",
      ok: d.resource.ready && compatibleGameVersion(d, d.resource.version),
      icon: "folder",
    },
  ];
  const contentVersion =
    d.mode === "mock" ? CONTENT_VERSION : (d.release?.tag ?? "Belum dimuat");
  const gameLine =
    d.mode === "mock"
      ? "3.7.0"
      : d.release
        ? `${d.release.gameVersion.split(".").slice(0, 2).join(".")}.*`
        : "—";
  return `<section class="resonance-hero" aria-labelledby="hero-title">
      <div class="hero-copy">
        <p class="eyebrow"><span class="tiny-dot"></span> WUTHERING WAVES, DALAM BAHASA KITA</p>
        <h1 id="hero-title">Pahami ceritanya.<br><em>Rasakan dunianya.</em></h1>
        <p class="hero-description">Jarak boleh jauh. Cerita tetap dekat.<br>Bawa Bahasa Indonesia ke petualanganmu di Solaris-3.</p>
        <div class="hero-actions"><button class="btn btn-primary" data-action="${primary.action}" ${disabled}>${icon(d.patch.installed ? "external" : "download")} ${primary.label} ${icon("arrow")}</button><button class="text-button" data-section="tools">Cek perangkat ${icon("arrow")}</button></div>
        <div class="hero-status"><span class="status-dot ${ready.ready ? "good" : "warning"}"></span><span>${esc(ready.label)}</span><span class="hero-status-divider">/</span><span>${d.mode === "mock" ? "Pratinjau browser" : "Pemasangan native"}</span></div>
      </div>
      <div class="resonance-art" aria-hidden="true">
        <span class="art-caption">FREKUENSI YANG SAMA.<br>BAHASA YANG BERBEDA.</span>
        <svg class="resonance-sculpture" viewBox="0 0 520 440" fill="none">
          <defs><linearGradient id="wave-ink" x1="100" y1="0" x2="430" y2="440" gradientUnits="userSpaceOnUse"><stop stop-color="currentColor"/><stop offset="1" stop-color="currentColor" stop-opacity=".12"/></linearGradient></defs>
          <g stroke="url(#wave-ink)" stroke-width="1.2">
            ${Array.from({ length: 30 }, (_, i) => `<ellipse cx="260" cy="223" rx="${76 + i * 3.9}" ry="${147 - i * 2.3}" transform="rotate(${i * 3 - 43} 260 223)"/>`).join("")}
          </g><path d="M52 224h416M260 32v382" stroke="currentColor" stroke-opacity=".14" stroke-dasharray="3 7"/><circle cx="260" cy="223" r="5" fill="currentColor"/><circle cx="450" cy="224" r="3" fill="currentColor"/>
        </svg>
        <div class="art-footer"><span>RESONANSI / 001</span><span>EN <span class="art-arrow">↗</span> ID</span></div>
      </div>
    </section>
    <div class="section-heading"><div><span class="eyebrow">RUANG KENDALI</span><h2>Siap untuk cerita berikutnya.</h2></div><span class="section-index">01 — 03</span></div>
    <div class="home-grid">
      <section class="card translation-card" aria-labelledby="translation-title">
        <div class="card-header"><div><p class="eyebrow">01 / PAKET BAHASA</p><h2 id="translation-title">Bahasa Indonesia</h2></div><span class="language-mark">Aa<span>EN → ID</span></span></div>
        <p class="muted">Dialog dan misi dalam bahasa kita. Nama karakter, senjata, dan lokasi tetap dalam bahasa Inggris.</p>
        <div class="package-facts"><div><span>Rilis terbaru</span><strong>${esc(contentVersion)}</strong></div><div><span>Untuk game</span><strong>${esc(gameLine)}</strong></div>${d.patch.installed ? `<div><span>Terpasang</span><strong>${esc(d.patch.version)}</strong></div>` : ""}</div>
        <div class="package-state"><span class="status-dot ${ready.ready ? "good" : "warning"}"></span><div><strong>${esc(ready.label)}</strong><p>${esc(ready.detail)}</p></div></div>
        ${d.patch.installed && ready.ready ? `<button class="text-button" data-action="install" ${disabled}>Pasang ulang terjemahan ${icon("refresh")}</button>` : ""}
        ${d.release ? `<details class="release-details"><summary>Detail paket · ${(d.release.size / 1048576).toFixed(1)} MB</summary><p>${esc(d.release.assetName)}</p><p>SHA-256: <code>${esc(d.release.sha256)}</code></p><a href="https://github.com/TitoTFP/WuwaID/releases/latest" target="_blank" rel="noopener noreferrer">Lihat rilis di GitHub ${icon("external")}</a></details>` : ""}
        <p class="safety-note">${icon("shield")} ${d.mode === "mock" ? "Simulasi saja. File game tidak diubah." : "SHA-256 diverifikasi. PAK asli tidak ditimpa."}</p>
      </section>
      <section class="card device-card" aria-labelledby="device-title"><div class="card-header"><div><p class="eyebrow">02 / KESIAPAN PERANGKAT</p><h2 id="device-title">Sebelum berangkat</h2></div><button class="icon-btn" data-action="refresh" aria-label="Periksa ulang perangkat" ${state.busy ? "disabled" : ""}>${icon("refresh")}</button></div>
        <div class="device-list">${deviceRows.map((row) => `<div class="device-row"><span class="device-icon">${icon(row.icon)}</span><div><strong>${row.name}</strong><p>${esc(row.value)}</p></div><span class="device-check ${row.ok ? "" : "pending"}" aria-label="${row.ok ? "Siap" : "Perlu perhatian"}">${row.ok ? icon("check") : icon("info")}</span></div>`).join("")}</div>
        <div class="device-note">${icon("info")}<p>${d.mode === "mock" ? "Ini data contoh, bukan pemindaian perangkat. Pemasangan nyata tersedia di aplikasi Android." : d.platformSupported ? "Status dari Android/Shizuku. Game akan dihentikan sebelum penulisan paket." : "Desktop tidak mengakses resource Android. Gunakan aplikasi Android untuk pemasangan."}</p></div>
        <div class="checked-time">Diperiksa <span>${new Date(d.lastChecked).toLocaleTimeString("id-ID", { hour: "2-digit", minute: "2-digit" })}</span></div>
      </section>
      <button class="studio-card" data-section="customize"><span class="eyebrow">03 / SENTUHAN PERSONAL</span><span class="studio-glyph" aria-hidden="true">R<span>↗</span></span><span class="studio-title">Identitasmu.<br>Gayamu.</span><span class="studio-description">Jelajahi pratinjau UID kustom.</span><span class="studio-link">Buka Kustomisasi ${icon("arrow")}</span></button>
    </div>`;
}

function render() {
  const focused = document.activeElement as HTMLElement | null;
  const focusAction = focused?.dataset.action;
  if (!app.querySelector(".app-header")) {
    app.innerHTML = `<header class="app-header"><div class="header-inner"><a class="brand" href="#home" aria-label="WuwaIDMobile Beranda"><span class="brand-symbol">${logo}</span><span>wuwa<span class="brand-id">id</span><small>RUANG RESONANSI</small></span></a><nav class="desktop-nav" aria-label="Navigasi utama">${navigation.map((item) => `<button class="nav-item ${section === item.id ? "active" : ""}" data-section="${item.id}" ${section === item.id ? 'aria-current="page"' : ""}><span>${item.number}</span>${item.label}</button>`).join("")}</nav>${renderThemeControl()}</div></header>
    <div class="workspace"><div class="context-bar"><span>LOKALISASI INDEPENDEN <span class="context-separator">/</span> INDONESIA</span><span class="mode-label"><span class="tiny-dot"></span>${state.dashboard.mode === "mock" ? "SIMULASI BROWSER" : "NATIVE · TAURI"}</span></div>
    <main id="main-content" tabindex="-1"></main><footer class="footer"><div class="footer-brand">${logo}<span>Bahasanya kita.<br><strong>Petualangannya bersama.</strong></span></div><p>Proyek komunitas independen.<br>Tidak berafiliasi dengan Kuro Games.</p><span class="footer-coordinate">SOLARIS–3 ↗ ID</span></footer></div>
    <nav class="mobile-nav" aria-label="Navigasi seluler">${navigation.map((item) => `<button data-section="${item.id}" class="${section === item.id ? "active" : ""}" ${section === item.id ? 'aria-current="page"' : ""}>${icon(item.id)}<span>${item.label}</span></button>`).join("")}</nav>`;
  }
  app.querySelectorAll<HTMLButtonElement>(".desktop-nav [data-section], .mobile-nav [data-section]").forEach((button) => {
    const active = button.dataset.section === section;
    button.classList.toggle("active", active);
    if (active) button.setAttribute("aria-current", "page");
    else button.removeAttribute("aria-current");
  });
  app.querySelector("#main-content")!.innerHTML = `
    ${error ? `<div class="error-banner" role="alert">${icon("info")}<span>${esc(error)}</span><button class="icon-btn" data-action="dismiss-error" aria-label="Tutup pesan">${icon("close")}</button></div>` : ""}
    ${state.busy ? `<div class="progress-card" role="status" aria-live="polite"><div><strong>${esc(state.progress?.label ?? "Menyiapkan operasi…")}</strong><span>${state.progress?.percent ?? 0}%</span></div><progress max="100" value="${state.progress?.percent ?? 0}" aria-label="Progres pemasangan"></progress><p>${state.dashboard.mode === "mock" ? "Simulasi IPC · tidak ada file game yang ditulis" : "Unduhan terverifikasi · pemasangan melalui Shizuku"}</p></div>` : ""}
    ${section === "home" ? renderHome() : section === "customize" ? renderCustomize(state) : renderTools(state)}`;
  dialog.querySelector("p")!.textContent =
    state.dashboard.mode === "mock"
      ? "Simulasi ini menghapus paket terjemahan dan memulihkan tampilan UID bawaan. File game asli dan UID akun tidak diubah."
      : "Hapus PAK, salinan signature, dan mount yang dimiliki WuwaIDMobile? File game asli tidak ditimpa atau dihapus.";
  if (focusAction)
    Array.from(app.querySelectorAll<HTMLElement>("[data-action]"))
      .find((el) => el.dataset.action === focusAction)
      ?.focus({ preventScroll: true });
}

async function execute(work: () => Promise<void>) {
  if (state.busy) return;
  state.busy = true;
  state.progress = null;
  error = "";
  render();
  try {
    await work();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
    notify(error);
    if (state.dashboard.mode === "native") {
      try {
        state.dashboard = await api.load();
      } catch (refreshError) {
        error += ` Status perangkat belum dapat dibaca ulang: ${String(refreshError)}`;
      }
    }
  } finally {
    state.busy = false;
    state.progress = null;
    render();
  }
}

async function operate(operation: Operation, uidText?: string) {
  await execute(async () => {
    state.dashboard = await api.run(operation, uidText);
    if (
      operation === "apply_uid" ||
      operation === "restore_uid" ||
      operation === "uninstall"
    )
      state.uidDraft = state.dashboard.uid.custom ?? "";
    const messages: Record<Operation, string> = {
      install:
        state.dashboard.mode === "mock"
          ? "Simulasi selesai. Terjemahan Bahasa Indonesia terpasang."
          : `Rilis ${state.dashboard.patch.version} terpasang dan terverifikasi.`,
      repair:
        state.dashboard.mode === "mock"
          ? "Simulasi pemulihan selesai. Pemasangan kembali sehat."
          : state.dashboard.patch.installed
            ? "Pemulihan selesai. Pemasangan telah diverifikasi."
            : "Transaksi dipulihkan. Belum ada terjemahan terpasang.",
      uninstall:
        state.dashboard.mode === "mock"
          ? "Simulasi selesai. Terjemahan dilepas dan tampilan UID dipulihkan."
          : "Berkas terjemahan milik WuwaIDMobile berhasil dihapus.",
      apply_uid:
        "Tampilan UID kustom diterapkan dalam simulasi. UID akun tidak berubah.",
      restore_uid: "Tampilan UID bawaan dipulihkan dalam simulasi.",
      launch_game:
        state.dashboard.mode === "mock"
          ? "Permintaan membuka game berhasil disimulasikan. Tidak ada game yang benar-benar dibuka."
          : "Wuthering Waves dibuka.",
      request_permission:
        state.dashboard.mode === "mock"
          ? "Izin Shizuku diberikan dalam simulasi."
          : "Izin Shizuku diberikan.",
    };
    notify(messages[operation]);
  });
}

app.addEventListener("click", async (event) => {
  const themeButton = (event.target as Element).closest<HTMLButtonElement>("[data-theme-choice]");
  if (themeButton) {
    themePreference = themeButton.dataset.themeChoice as ThemePreference;
    try {
      localStorage.setItem("wuwaidmobile.theme", themePreference);
    } catch {
      notify("Tema berubah untuk sesi ini; penyimpanan preferensi tidak tersedia.");
    }
    applyTheme();
    return;
  }
  const button = (event.target as Element).closest<HTMLElement>(
    "[data-section], [data-action], .brand",
  );
  if (!button || button.hasAttribute("disabled")) return;
  if (!state) return;
  if (button.dataset.section || button.classList.contains("brand")) {
    event.preventDefault();
    section = (button.dataset.section as Section) || "home";
    render();
    window.scrollTo({ top: 0, behavior: "instant" });
    return;
  }
  const action = button.dataset.action;
  if (action === "dismiss-error") {
    error = "";
    render();
  } else if (action === "tools") {
    section = "tools";
    render();
  } else if (action === "refresh")
    await execute(async () => {
      state.dashboard = await api.load();
      notify(state.dashboard.mode === "mock" ? "Status perangkat simulasi diperbarui." : "Status perangkat diperbarui.");
    });
  else if (action === "restore-uid") await operate("restore_uid");
  else if (action === "uninstall") dialog.showModal();
  else if (action === "export-diagnostics")
    await execute(async () => {
      const report = await api.diagnostics();
      const url = URL.createObjectURL(
        new Blob([report.content], { type: "application/json" }),
      );
      const link = document.createElement("a");
      link.href = url;
      link.download = report.filename;
      link.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
      notify(
        state.dashboard.mode === "mock"
          ? "Laporan diagnostik simulasi diekspor sebagai JSON."
          : "Laporan diagnostik native diekspor sebagai JSON.",
      );
    });
  else if (action === "setup-game" || action === "setup-shizuku") {
    if (state.dashboard.mode === "native") {
      await execute(async () => {
        await api.openSetup(action === "setup-game" ? "game" : "shizuku");
        notify(
          "Selesaikan pengaturan, lalu kembali dan periksa ulang perangkat.",
        );
      });
    } else {
      section = "tools";
      render();
      notify(
        action === "setup-game"
          ? "Untuk demo browser, pilih skenario Semua siap."
          : "Di Android, jalankan Shizuku dan berikan izin. Untuk demo browser, ubah skenario perangkat.",
      );
    }
  } else if (
    action &&
    ["install", "repair", "launch_game", "request_permission"].includes(action)
  )
    await operate(action as Operation);
});

app.addEventListener("input", (event) => {
  const input = event.target as HTMLInputElement;
  if (input.id !== "uid-input") return;
  state.uidDraft = input.value;
  const draft = state.uidDraft.trim();
  const d = state.dashboard;
  app.querySelector<HTMLOutputElement>("#uid-preview")!.textContent =
    draft || d.uid.original;
  app.querySelector<HTMLElement>("#uid-count")!.textContent =
    `${state.uidDraft.length}/${UID_MAX_LENGTH}`;
  app.querySelector<HTMLButtonElement>("#apply-uid")!.disabled =
    state.busy ||
    !d.uidAvailable ||
    !readiness(d).ready ||
    !d.patch.installed ||
    !d.patch.healthy ||
    !draft ||
    draft.length > UID_MAX_LENGTH ||
    draft === (d.uid.custom ?? d.uid.original);
});

app.addEventListener("submit", async (event) => {
  if ((event.target as HTMLFormElement).id !== "uid-form") return;
  event.preventDefault();
  if (app.querySelector<HTMLButtonElement>("#apply-uid")!.disabled) return;
  await operate("apply_uid", state.uidDraft);
});

app.addEventListener("change", async (event) => {
  const target = event.target as HTMLSelectElement;
  if (target.id === "scenario-select")
    await execute(async () => {
      state.dashboard = await api.setScenario(target.value as Scenario);
      state.uidDraft = state.dashboard.uid.custom ?? "";
      notify("Skenario perangkat simulasi diubah.");
    });
});

document
  .querySelector("#cancel-uninstall")!
  .addEventListener("click", () => dialog.close());
document
  .querySelector("#confirm-uninstall")!
  .addEventListener("click", async () => {
    dialog.close();
    await operate("uninstall");
  });

api.subscribeProgress((progress) => {
  if (state?.busy) {
    state.progress = progress;
    // Progress must not replace navigation, theme controls, or focused inputs.
    const card = app.querySelector<HTMLElement>(".progress-card");
    if (card) {
      card.querySelector("strong")!.textContent = progress.label;
      card.querySelector("div > span")!.textContent = `${progress.percent}%`;
      card.querySelector("progress")!.value = progress.percent;
    }
  }
});

async function start() {
  try {
    const dashboard = await api.load();
    state = {
      dashboard,
      busy: false,
      progress: null,
      uidDraft: dashboard.uid.custom ?? "",
    };
    render();
  } catch (cause) {
    app.innerHTML = `<main class="startup-error"><h1>Pratinjau belum dapat dimuat</h1><p>${esc(String(cause))}</p><button class="btn btn-primary" id="retry-start">Coba lagi</button></main>`;
    document.querySelector("#retry-start")!.addEventListener("click", start);
  }
}
void start();
