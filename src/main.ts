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
let renderedSection: Section | undefined;
const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");
reducedMotion.addEventListener("change", () => {
  if (reducedMotion.matches)
    app.querySelector("#main-content")?.getAnimations().forEach((animation) => animation.cancel());
});
let error = "";
let toastTimer: number;
const navigation: { id: Section; label: string }[] = [
  { id: "home", label: "Beranda" },
  { id: "customize", label: "Kustomisasi" },
  { id: "tools", label: "Perawatan" },
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
    label: "Pasang terjemahan",
  };
  if (!d.platformSupported)
    primary = { action: "refresh", label: "Perlu Android" };
  else if (!d.game.installed)
    primary = { action: "setup-game", label: "Pasang game" };
  else if (d.access.method === "root" && !d.access.granted)
    primary = { action: "request_root", label: "Berikan izin root" };
  else if (d.access.method === "shizuku" && !d.shizuku.running)
    primary = { action: "setup-shizuku", label: "Buka Shizuku" };
  else if (!d.access.granted)
    primary = { action: "request_permission", label: "Berikan izin Shizuku" };
  else if (d.mode === "native" && !d.release)
    primary = { action: "refresh", label: "Muat ulang rilis" };
  else if (!compatibleGameVersion(d, d.game.version))
    primary = { action: "tools", label: "Lihat kompatibilitas" };
  else if (!d.resource.ready)
    primary = {
      action: "launch_game",
      label: "Buka game",
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
      name: d.access.method === "root" ? "Root" : "Shizuku",
      value: !d.platformSupported
        ? "Memerlukan Android"
        : d.access.granted
          ? "Aktif · diizinkan"
          : d.access.method === "root"
            ? "Menunggu izin root"
            : d.shizuku.running ? "Menunggu izin" : "Belum aktif",
      ok: d.access.granted,
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
  return `<section class="page-heading home-heading">
      <div><h1>Bahasa Indonesia</h1><p>Terjemahan Wuthering Waves</p></div>
      <button class="btn btn-primary" data-action="${primary.action}" ${disabled}>${icon(d.patch.installed ? "external" : "download")} ${primary.label}</button>
    </section>
    <div class="home-grid">
      <section class="card translation-card" aria-labelledby="translation-title">
        <div class="card-header"><h2 id="translation-title">Paket</h2>${icon("download")}</div>
        <div class="package-state"><span class="status-dot ${ready.ready ? "good" : "warning"}"></span><div><strong>${esc(ready.label)}</strong>${!ready.ready ? `<p>${esc(ready.detail)}</p>` : ""}</div></div>
        <div class="package-facts"><div><span>Terbaru</span><strong>${esc(contentVersion)}</strong></div><div><span>Versi game</span><strong>${esc(gameLine)}</strong></div>${d.patch.installed ? `<div><span>Terpasang</span><strong>${esc(d.patch.version)}</strong></div>` : ""}</div>
        ${d.patch.installed && ready.ready ? `<button class="text-button" data-action="install" ${disabled}>Pasang ulang ${icon("refresh")}</button>` : ""}
      </section>
      <section class="card device-card" aria-labelledby="device-title">
        <div class="card-header"><h2 id="device-title">Perangkat</h2><button class="icon-btn" data-action="refresh" aria-label="Periksa perangkat" ${state.busy ? "disabled" : ""}>${icon("refresh")}</button></div>
        <div class="device-list">${deviceRows.map((row) => `<div class="device-row"><span class="device-icon">${icon(row.icon)}</span><div><strong>${row.name}</strong><p>${esc(row.value)}</p></div><span class="device-check ${row.ok ? "" : "pending"}" aria-label="${row.ok ? "Siap" : "Perlu perhatian"}">${row.ok ? icon("check") : icon("info")}</span></div>`).join("")}</div>
      </section>
    </div>`;
}

function render() {
  const focused = document.activeElement as HTMLElement | null;
  const focusAction = focused?.dataset.action;
  if (!app.querySelector(".app-header")) {
    app.innerHTML = `<header class="app-header"><div class="header-inner"><a class="brand" href="#home" aria-label="WuwaIDMobile Beranda"><span class="brand-symbol">${logo}</span><span>wuwa<span class="brand-id">id</span></span></a><nav class="desktop-nav" aria-label="Navigasi utama">${navigation.map((item) => `<button class="nav-item ${section === item.id ? "active" : ""}" data-section="${item.id}" ${section === item.id ? 'aria-current="page"' : ""}>${item.label}</button>`).join("")}</nav>${renderThemeControl()}</div></header>
    <div class="workspace">${state.dashboard.mode === "mock" ? `<p class="mode-notice">${icon("info")} Simulasi · file game tidak diubah.</p>` : ""}
    <main id="main-content" tabindex="-1"></main><footer class="footer">Tidak berafiliasi dengan Kuro Games.</footer></div>
    <nav class="mobile-nav" aria-label="Navigasi seluler">${navigation.map((item) => `<button data-section="${item.id}" class="${section === item.id ? "active" : ""}" ${section === item.id ? 'aria-current="page"' : ""}>${icon(item.id)}<span>${item.label}</span></button>`).join("")}</nav>`;
  }
  app.querySelectorAll<HTMLButtonElement>(".desktop-nav [data-section], .mobile-nav [data-section]").forEach((button) => {
    const active = button.dataset.section === section;
    button.classList.toggle("active", active);
    if (active) button.setAttribute("aria-current", "page");
    else button.removeAttribute("aria-current");
  });
  const content = app.querySelector<HTMLElement>("#main-content")!;
  content.innerHTML = `
    ${error ? `<div class="error-banner" role="alert">${icon("info")}<span>${esc(error)}</span><button class="icon-btn" data-action="dismiss-error" aria-label="Tutup pesan">${icon("close")}</button></div>` : ""}
    ${state.busy ? `<div class="progress-card" role="status" aria-live="polite"><div><strong>${esc(state.progress?.label ?? "Menyiapkan…")}</strong><span>${state.progress?.percent ?? 0}%</span></div><div class="progress-track" role="progressbar" aria-label="Progres pemasangan" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${state.progress?.percent ?? 0}"><span class="progress-fill" style="transform: scaleX(${(state.progress?.percent ?? 0) / 100})"></span></div></div>` : ""}
    ${section === "home" ? renderHome() : section === "customize" ? renderCustomize(state) : renderTools(state)}`;
  if (renderedSection !== section) {
    content.getAnimations().forEach((animation) => animation.cancel());
    if (!reducedMotion.matches)
      content.animate(
        [{ opacity: 0.65, transform: "translateY(4px)" }, { opacity: 1, transform: "translateY(0)" }],
        { duration: 180, easing: "cubic-bezier(.2, .8, .2, 1)" },
      );
    renderedSection = section;
  }
  dialog.querySelector("p")!.textContent =
    state.dashboard.mode === "mock"
      ? "Hapus terjemahan simulasi dan reset tampilan UID? File game tidak diubah."
      : "Hapus paket dan mount WuwaIDMobile? File game asli tetap utuh.";
  if (focusAction)
    Array.from(app.querySelectorAll<HTMLElement>("[data-action]"))
      .find((el) => el.dataset.action === focusAction)
      ?.focus({ preventScroll: true });
}

async function execute(work: () => Promise<void>) {
  if (state.busy) return;
  const busyTrace = api.beginBusy();
  state.busy = true;
  state.progress = null;
  error = "";
  render();
  try {
    await busyTrace;
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
    await api.endBusy();
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
      install: `Terjemahan ${state.dashboard.patch.version} terpasang.`,
      repair: state.dashboard.patch.installed
        ? "Pemasangan dipulihkan."
        : "Transaksi dipulihkan. Terjemahan belum terpasang.",
      uninstall: state.dashboard.mode === "mock"
        ? "Terjemahan dihapus. Tampilan UID direset."
        : "Terjemahan dihapus.",
      apply_uid: "Tampilan UID diterapkan.",
      restore_uid: "Tampilan UID direset.",
      launch_game: state.dashboard.mode === "mock"
        ? "Simulasi buka game selesai."
        : "Game dibuka.",
      request_permission: "Izin Shizuku diberikan.",
      request_root: "Akses root aktif.",
      use_root: "Metode Root dipilih. Berikan izin root untuk mengakses game.",
      use_shizuku: "Metode Shizuku dipilih.",
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
      notify("Tema berlaku untuk sesi ini; preferensi tidak dapat disimpan.");
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
      notify("Status diperbarui.");
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
      notify("Diagnostik diekspor.");
    });
  else if (action === "setup-game" || action === "setup-shizuku") {
    if (state.dashboard.mode === "native") {
      await execute(async () => {
        await api.openSetup(action === "setup-game" ? "game" : "shizuku");
        notify(
          "Selesaikan pengaturan, lalu periksa ulang.",
        );
      });
    } else {
      section = "tools";
      render();
      notify('Pilih skenario "Semua siap".');
    }
  } else if (
    action &&
    ["install", "repair", "launch_game", "request_permission", "request_root", "use_root", "use_shizuku"].includes(action)
  )
    await operate(action as Operation);
});

app.addEventListener("input", (event) => {
  const input = event.target as HTMLInputElement;
  if (input.id !== "uid-input") return;
  state.uidDraft = input.value;
  const draft = state.uidDraft.trim();
  const d = state.dashboard;
  app.querySelector<HTMLElement>("#uid-count")!.textContent =
    `${state.uidDraft.length}/${UID_MAX_LENGTH}`;
  app.querySelector<HTMLButtonElement>("#apply-uid")!.disabled =
    state.busy ||
    !d.uidAvailable ||
    !readiness(d).ready ||
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
      notify("Skenario diubah.");
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
      const track = card.querySelector<HTMLElement>(".progress-track")!;
      track.setAttribute("aria-valuenow", String(progress.percent));
      track.querySelector<HTMLElement>(".progress-fill")!.style.transform = `scaleX(${progress.percent / 100})`;
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
    app.innerHTML = `<main class="startup-error"><h1>Gagal memuat</h1><p>${esc(String(cause))}</p><button class="btn btn-primary" id="retry-start">Coba lagi</button></main>`;
    document.querySelector("#retry-start")!.addEventListener("click", start);
  }
}
void start();
