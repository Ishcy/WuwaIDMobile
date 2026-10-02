import {
  UID_MAX_LENGTH,
  compatibleGameVersion,
  installationPrerequisites,
  readiness,
  type Scenario,
  type UiState,
} from "./model";
import { escapeHtml, icon } from "./ui";

const scenarios: Array<{ id: Scenario; label: string }> = [
  { id: "ready", label: "Semua siap" },
  { id: "installed", label: "Terjemahan terpasang" },
  { id: "game_missing", label: "Game belum terpasang" },
  { id: "unsupported", label: "Versi tidak didukung" },
  { id: "resource_missing", label: "Resource belum siap" },
  { id: "shizuku_off", label: "Shizuku belum berjalan" },
  { id: "permission", label: "Perlu izin Shizuku" },
  { id: "recovery", label: "Pemulihan diperlukan" },
];

function button(
  action: string,
  label: string,
  disabled: boolean,
  className = "btn btn-secondary",
): string {
  return `<button type="button" class="${className}" data-action="${action}"${disabled ? ' disabled aria-disabled="true"' : ""}>${label}</button>`;
}

export function renderCustomize(state: UiState): string {
  const d = state.dashboard;
  if (!d.uidAvailable)
    return `<section class="page-heading"><h1>Kustomisasi</h1></section><section class="card unavailable-studio"><h2>Belum tersedia</h2><p class="muted">UID kustom hanya tersedia di simulasi browser.</p><button class="text-button" data-section="home">Beranda ${icon("arrow")}</button></section>`;
  const status = readiness(d);
  const canCustomize =
    !state.busy && d.patch.installed && d.patch.healthy && status.ready;
  const draft = state.uidDraft.trim();
  const current = d.uid.custom ?? d.uid.original;
  const canApply =
    canCustomize &&
    draft.length > 0 &&
    draft.length <= UID_MAX_LENGTH &&
    draft !== current;
  const gate = !status.ready
    ? status.detail
    : !d.patch.installed
      ? "Pasang terjemahan terlebih dahulu."
      : "";

  return `<section class="page-heading"><h1>Kustomisasi</h1><p>Simulasi tampilan UID. UID akun tetap.</p></section>
    <div class="uid-grid">
      <section class="card uid-editor" aria-labelledby="uid-title">
        <div class="card-header"><h2 id="uid-title">UID kustom</h2>${icon("customize")}</div>
        ${gate ? `<p class="muted uid-gate">${escapeHtml(gate)}</p>` : ""}
        <form id="uid-form">
          <label class="field-label" for="uid-input">Teks</label>
          <input class="uid-input" id="uid-input" name="customUid" type="text" maxlength="${UID_MAX_LENGTH}" value="${escapeHtml(state.uidDraft)}" placeholder="Rover Indonesia" autocomplete="off" spellcheck="false" aria-describedby="uid-help" ${state.busy ? "disabled" : ""} />
          <div class="uid-input-meta"><span id="uid-help">Maks. ${UID_MAX_LENGTH} karakter</span><span id="uid-count">${state.uidDraft.length}/${UID_MAX_LENGTH}</span></div>
          <div class="actions"><button class="btn btn-primary" id="apply-uid" type="submit" ${canApply ? "" : "disabled"}>${icon("check")} Terapkan</button>${button("restore-uid", `${icon("refresh")} Reset`, !canCustomize || d.uid.custom === null)}</div>
        </form>
        <dl class="info-list uid-info">
          <div class="info-row"><dt>Bawaan</dt><dd>${escapeHtml(d.uid.original)}</dd></div>
          <div class="info-row"><dt>Aktif</dt><dd id="uid-active">${escapeHtml(current)}</dd></div>
        </dl>
      </section>
      <section class="uid-preview-card" aria-labelledby="uid-preview-title">
        <div class="preview-heading"><h2 id="uid-preview-title">Pratinjau</h2></div>
        <div class="uid-preview-stage"><span class="uid-preview-label">UID</span><output id="uid-preview" for="uid-input">${escapeHtml(draft || d.uid.original)}</output></div>
      </section>
    </div>`;
}

export function renderTools(state: UiState): string {
  const d = state.dashboard;
  const status = readiness(d);
  const dependenciesReady = installationPrerequisites(d);
  const canUninstall =
    d.platformSupported &&
    d.storageWritable &&
    d.game.installed &&
    d.shizuku.running &&
    d.shizuku.granted &&
    d.resource.ready &&
    !d.patch.recoveryPending;
  const canRecover =
    d.platformSupported &&
    d.storageWritable &&
    d.game.installed &&
    d.shizuku.running &&
    d.shizuku.granted &&
    d.resource.ready &&
    d.patch.recoveryPending;
  const canRepair =
    !state.busy && (canRecover || (dependenciesReady && d.patch.installed));
  const currentScenario: Scenario = !d.game.installed
    ? "game_missing"
    : !compatibleGameVersion(d, d.game.version)
      ? "unsupported"
      : !d.shizuku.running
        ? "shizuku_off"
        : !d.shizuku.granted
          ? "permission"
          : !d.resource.ready
            ? "resource_missing"
            : d.patch.recoveryPending
              ? "recovery"
              : d.patch.installed
                ? "installed"
                : "ready";
  const scenarioOptions = scenarios
    .map(
      ({ id, label }) =>
        `<option value="${id}"${id === currentScenario ? " selected" : ""}>${label}</option>`,
    )
    .join("");
  const rows = [
    {
      name: "Status perangkat",
      detail: `Diperiksa ${escapeHtml(new Date(d.lastChecked).toLocaleTimeString("id-ID", { hour: "2-digit", minute: "2-digit" }))}`,
      action: "refresh",
      label: "Periksa",
      disabled: state.busy,
      icon: "refresh",
    },
    {
      name: "Perbaiki pemasangan",
      detail: d.patch.recoveryPending
        ? "Pulihkan transaksi tertunda."
        : "Periksa dan pulihkan paket.",
      action: "repair",
      label: "Perbaiki",
      disabled: !canRepair,
      icon: "shield",
    },
    {
      name: "Ekspor diagnostik",
      detail: "Laporan JSON.",
      action: "export-diagnostics",
      label: "Ekspor",
      disabled: state.busy,
      icon: "download",
    },
    {
      name: "Hapus terjemahan",
      detail:
        "File game asli tetap utuh.",
      action: "uninstall",
      label: "Hapus",
      disabled: state.busy || !d.patch.installed || !canUninstall,
      icon: "trash",
    },
  ]
    .map(
      (row) =>
        `<div class="tool-row ${row.action === "uninstall" ? "tool-destructive" : ""}"><span class="tool-icon">${icon(row.icon)}</span><div class="tool-copy"><h3>${row.name}</h3><p class="muted">${row.detail}</p></div>${button(row.action, row.label, row.disabled, row.action === "uninstall" ? "btn btn-danger" : "btn btn-secondary")}</div>`,
    )
    .join("");
  return `<section class="page-heading"><h1>Perawatan</h1></section>
    <section class="health-banner" aria-label="Status pemasangan"><span class="health-icon ${status.ready ? "healthy" : "needs-attention"}">${icon(status.ready ? "shield" : "info")}</span><div><h2>${escapeHtml(status.label)}</h2>${!status.ready ? `<p class="muted">${escapeHtml(status.detail)}</p>` : ""}</div></section>
    <section class="tool-list" aria-label="Tindakan perawatan">${rows}</section>
    <div class="tools-detail-grid">
      <details class="card technical-details"><summary>Detail teknis</summary><dl class="info-list">
        <div class="info-row"><dt>Game</dt><dd>${d.game.installed ? "Terpasang" : "Belum terpasang"} · ${escapeHtml(d.game.version)}<code>${escapeHtml(d.game.packageName)}</code></dd></div>
        <div class="info-row"><dt>Shizuku</dt><dd>${d.shizuku.running ? "Aktif" : "Tidak aktif"} · ${d.shizuku.granted ? "Diizinkan" : "Belum diizinkan"}</dd></div>
        <div class="info-row"><dt>Resource</dt><dd>${d.resource.ready ? "Siap" : "Belum siap"} · ${escapeHtml(d.resource.version)}</dd></div>
        <div class="info-row"><dt>Terjemahan</dt><dd>${d.patch.installed ? "Terpasang" : "Belum terpasang"} · ${d.patch.healthy ? "Sehat" : "Perlu pemeriksaan"} · ${escapeHtml(d.patch.version)}${d.patch.recoveryPending ? " · Pemulihan tertunda" : ""}</dd></div>
        <div class="info-row"><dt>Rilis terbaru</dt><dd>${escapeHtml(d.mode === "mock" ? "3.7.0-R2 (contoh)" : (d.release?.tag ?? "Belum dimuat"))}</dd></div>
        ${d.uidAvailable ? `<div class="info-row"><dt>Tampilan UID</dt><dd>${escapeHtml(d.uid.custom ?? d.uid.original)}</dd></div>` : ""}
        ${d.patch.message ? `<div class="info-row"><dt>Pemasangan</dt><dd>${escapeHtml(d.patch.message)}</dd></div>` : ""}
      </dl></details>
      ${d.mode === "mock" ? `<section class="card scenario-card"><label class="field-label" for="scenario-select">Skenario simulasi</label><select class="select" id="scenario-select"${state.busy ? " disabled" : ""}>${scenarioOptions}</select></section>` : ""}
    </div>`;
}
