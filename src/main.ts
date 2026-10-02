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
const navigation: {
  id: Section;
  label: string;
  subtitle: string;
  number: string;
}[] = [
  {
    id: "home",
    label: "Beranda",
    subtitle: "Terjemahan & perangkat",
    number: "01",
  },
  {
    id: "customize",
    label: "Kustomisasi",
    subtitle: "Custom UID & tampilan",
    number: "02",
  },
  {
    id: "tools",
    label: "Perawatan",
    subtitle: "Pemeriksaan & pemulihan",
    number: "03",
  },
];
const logo =
  '<svg viewBox="0 0 40 40" fill="none" aria-hidden="true"><path d="M6 11 13 29l7-17 7 17 7-18M6 20l7 9m14 0 7-9" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/></svg>';

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
  return `<div class="page-heading"><div><p class="eyebrow">SELAMAT DATANG, ROVER</p><h1>Dunia yang sama.<br><span>Cerita dalam bahasa kita.</span></h1><p>Jelajahi Solaris-3 dengan Bahasa Indonesia.</p></div><span class="edition-label">LOKALISASI INDONESIA<br><strong>WUWA / ID</strong></span></div>
    <section class="hero" aria-labelledby="hero-title">
      <div class="hero-art" aria-hidden="true"><div class="orbit orbit-one"></div><div class="orbit orbit-two"></div><div class="orbit orbit-three"></div><div class="art-core"></div><span class="art-coordinate">SOLARIS–3 / 06:42</span><span class="art-label">RESONANCE<br>WITHOUT BORDERS.</span></div>
      <div class="hero-content"><span class="hero-kicker"><span class="tiny-dot"></span> BAHASA INDONESIA</span><h2 id="hero-title">Lebih dekat<br>dengan setiap cerita.</h2><p>Dialog, misi, dan dunia Wuthering Waves.<br>Dalam bahasa yang terasa seperti rumah.</p><div class="hero-meta"><span>${icon("game")} Wuthering Waves Global</span><span>PATCH ${esc(contentVersion)}</span></div></div>
    </section>
    <div class="home-grid"><section class="card translation-card" aria-labelledby="translation-title"><div class="card-header"><div><p class="eyebrow">PAKET TERJEMAHAN</p><h2 id="translation-title">Bahasa Indonesia</h2></div><span class="language-mark">ID<span>EN → ID</span></span></div>
      <p class="muted">Cerita diterjemahkan. Nama karakter, senjata, dan lokasi tetap menggunakan nama Inggris.</p>
      <div class="package-facts"><div><span>Versi konten</span><strong>${esc(contentVersion)}</strong></div><div><span>Kompatibilitas</span><strong>Game ${esc(gameLine)}</strong></div><div><span>Nama dalam game</span><strong>Nama Inggris</strong></div>${d.patch.installed ? `<div><span>Terpasang</span><strong>${esc(d.patch.version)}</strong></div>` : ""}</div>
      ${d.release ? `<details class="release-details"><summary>Detail rilis GitHub · ${(d.release.size / 1048576).toFixed(1)} MB</summary><p>${esc(d.release.assetName)}</p><p>SHA-256: <code>${esc(d.release.sha256)}</code></p><a href="https://github.com/TitoTFP/WuwaID/releases/latest" target="_blank" rel="noopener noreferrer">TitoTFP/WuwaID · rilis latest</a></details>` : ""}
      <div class="package-state"><span class="status-dot ${ready.ready ? "good" : "warning"}"></span><div><strong>${esc(ready.label)}</strong><p>${esc(ready.detail)}</p></div></div>
      <button class="btn btn-primary btn-wide" data-action="${primary.action}" ${disabled}>${icon(d.patch.installed ? "external" : "download")} ${primary.label} ${icon("arrow", "end-icon")}</button>
      ${d.patch.installed && ready.ready ? `<button class="text-button" data-action="install" ${disabled}>Pasang ulang terjemahan ${icon("refresh")}</button>` : ""}
      <p class="safety-note">${icon("shield")} ${d.mode === "mock" ? "Simulasi pemasangan · file game asli tidak diubah" : "SHA-256 diperiksa sebelum pemasangan · PAK asli tidak ditimpa"}</p>
    </section>
    <section class="card device-card" aria-labelledby="device-title"><div class="card-header"><div><p class="eyebrow">PEMERIKSAAN SISTEM</p><h2 id="device-title">Perangkatmu</h2></div><button class="icon-btn" data-action="refresh" aria-label="Periksa ulang perangkat" ${state.busy ? "disabled" : ""}>${icon("refresh")}</button></div>
      <div class="device-list">${deviceRows.map((row) => `<div class="device-row"><span class="device-icon">${icon(row.icon)}</span><div><strong>${row.name}</strong><p>${esc(row.value)}</p></div><span class="device-check ${row.ok ? "" : "pending"}" aria-label="${row.ok ? "Siap" : "Perlu perhatian"}">${row.ok ? icon("check") : "!"}</span></div>`).join("")}</div>
      <div class="device-note">${icon("info")} <p>${d.mode === "mock" ? "Data perangkat berasal dari mock IPC, bukan hasil pemindaian perangkat asli." : d.platformSupported ? "Status dibaca melalui bridge Android/Shizuku. Tutup game sebelum pemasangan; aplikasi akan menghentikan game saat commit." : "Desktop tidak menyediakan akses resource Android. Gunakan aplikasi Android untuk pemasangan nyata."}</p></div><div class="checked-time">Terakhir diperiksa <span>${new Date(d.lastChecked).toLocaleTimeString("id-ID", { hour: "2-digit", minute: "2-digit" })}</span></div>
    </section></div>
    <section class="bottom-note"><span class="note-number">01 /</span><div><h3>Terjemahan dulu, petualangan kemudian.</h3><p>Pasang paket, sesuaikan tampilan UID, lalu kembali ke dunia Wuthering Waves.</p></div><button class="text-button" data-section="customize">Custom UID ${icon("arrow")}</button></section>`;
}

function render() {
  const ready = readiness(state.dashboard);
  const focused = document.activeElement as HTMLElement | null;
  const focusAction = focused?.dataset.action;
  app.innerHTML = `<aside class="sidebar"><a class="brand" href="#home" aria-label="WuwaIDMobile Beranda"><span class="brand-symbol">${logo}</span><span>WUWA<span class="brand-id">ID</span><small>MOBILE COMPANION</small></span></a><div class="sidebar-label">RUANG ROVER</div><nav aria-label="Navigasi utama">${navigation.map((item) => `<button class="nav-item ${section === item.id ? "active" : ""}" data-section="${item.id}" ${section === item.id ? 'aria-current="page"' : ""}>${icon(item.id)}<span>${item.label}<small>${item.subtitle}</small></span><span class="nav-number">${item.number}</span></button>`).join("")}</nav><div class="sidebar-bottom"><div class="sidebar-decoration" aria-hidden="true">${logo}</div><p>Dibuat untuk cerita.<br><strong>Dibangun untuk kita.</strong></p><div class="sidebar-version"><span class="tiny-dot"></span> FRONTEND PREVIEW <span>v0.1.0</span></div></div></aside>
    <div class="workspace"><header class="topbar"><span class="breadcrumb">Ruang Rover <span>/</span> <strong>${navigation.find((n) => n.id === section)!.label}</strong></span><div class="topbar-right"><span class="mock-badge">${icon("info")} ${state.dashboard.mode === "mock" ? "MODE SIMULASI" : "NATIVE · TAURI"}</span><span class="avatar" aria-label="Lokalisasi Indonesia">ID</span></div></header>
    <main id="main-content"><div class="mobile-brand">${logo}<strong>WUWA<span>ID</span></strong><span class="badge ${ready.ready ? "badge-good" : "badge-warning"}">${ready.label}</span></div>
    ${error ? `<div class="error-banner" role="alert">${icon("info")}<span>${esc(error)}</span><button class="icon-btn" data-action="dismiss-error" aria-label="Tutup pesan">${icon("close")}</button></div>` : ""}
    ${state.busy ? `<div class="progress-card" role="status" aria-live="polite"><div><strong>${esc(state.progress?.label ?? (state.dashboard.mode === "mock" ? "Menyiapkan simulasi…" : "Menyiapkan operasi…"))}</strong><span>${state.progress?.percent ?? 0}%</span></div><progress max="100" value="${state.progress?.percent ?? 0}" aria-label="Progres pemasangan"></progress><p>${state.dashboard.mode === "mock" ? "Simulasi IPC · tidak ada file game yang ditulis" : "Unduhan terverifikasi · pemasangan melalui Shizuku"}</p></div>` : ""}
    ${section === "home" ? renderHome() : section === "customize" ? renderCustomize(state) : renderTools(state)}
    <footer class="footer"><span>WUWA<span class="footer-id">ID</span> <span class="footer-divider">/</span> Bahasanya kita. Petualangannya bersama.</span><span>Komunitas independen · tidak berafiliasi dengan Kuro Games</span></footer></main></div>
    <nav class="mobile-nav" aria-label="Navigasi seluler">${navigation.map((item) => `<button data-section="${item.id}" class="${section === item.id ? "active" : ""}" ${section === item.id ? 'aria-current="page"' : ""}>${icon(item.id)}<span>${item.label}</span></button>`).join("")}</nav>`;
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
  const button = (event.target as Element).closest<HTMLElement>(
    "[data-section], [data-action], .brand",
  );
  if (!button || button.hasAttribute("disabled")) return;
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
      notify("Status perangkat simulasi diperbarui.");
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
    render();
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
