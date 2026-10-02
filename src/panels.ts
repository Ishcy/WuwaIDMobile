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
  { id: "installed", label: "Terjemahan sudah terpasang" },
  { id: "game_missing", label: "Game belum terpasang" },
  { id: "unsupported", label: "Versi tidak didukung" },
  { id: "resource_missing", label: "Resource belum siap" },
  { id: "shizuku_off", label: "Shizuku belum berjalan" },
  { id: "permission", label: "Izin Shizuku belum diberikan" },
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
    return `<section class="page-heading"><p class="eyebrow">02 / STUDIO TAMPILAN</p><h1>Sedikit lebih <em>kamu.</em></h1><p>Sentuhan personal untuk identitas di layar.</p></section><section class="card unavailable-studio"><div class="studio-glyph" aria-hidden="true">R<span>↗</span></div><div><span class="badge">Pratinjau browser saja</span><h2>Studio UID belum tersedia di Android</h2><p class="muted">Kustomisasi UID saat ini hanya simulasi browser. Backend native memasang paket terjemahan, bukan mengubah UID akun atau tampilan UID game.</p><button class="text-button" data-section="home">Kembali ke paket bahasa ${icon("arrow")}</button></div></section>`;
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
      ? "Pasang terjemahan melalui Beranda sebelum menerapkan UID kustom."
      : "Tampilan UID siap dikustomisasi dalam simulasi ini.";

  return `<section class="page-heading"><p class="eyebrow">02 / STUDIO TAMPILAN</p><h1>Sedikit lebih <em>kamu.</em></h1><p>Eksperimen dengan identitas di layar. UID akun tetap sama.</p></section>
    <div class="uid-grid">
      <section class="card uid-editor" aria-labelledby="uid-title">
        <div class="card-header"><div><p class="eyebrow">PERSONALISASI</p><h2 id="uid-title">Tulis identitasmu</h2></div>${icon("customize")}</div>
        <span class="badge ${canCustomize ? "badge-good" : "badge-warning"}">${canCustomize ? "Studio siap" : "Perlu persiapan"}</span>
        <p class="muted uid-gate">${escapeHtml(gate)}</p>
        <form id="uid-form">
          <label class="field-label" for="uid-input">Tampilan UID</label>
          <input class="uid-input" id="uid-input" name="customUid" type="text" maxlength="${UID_MAX_LENGTH}" value="${escapeHtml(state.uidDraft)}" placeholder="Contoh: Rover Indonesia" autocomplete="off" spellcheck="false" aria-describedby="uid-help" ${state.busy ? "disabled" : ""} />
          <div class="uid-input-meta"><span id="uid-help">Teks atau angka · maks. ${UID_MAX_LENGTH}</span><span id="uid-count">${state.uidDraft.length}/${UID_MAX_LENGTH}</span></div>
          <div class="actions"><button class="btn btn-primary" id="apply-uid" type="submit" ${canApply ? "" : "disabled"}>${icon("check")} Terapkan di simulasi</button>${button("restore-uid", `${icon("refresh")} Reset tampilan`, !canCustomize || d.uid.custom === null)}</div>
        </form>
        <dl class="info-list uid-info">
          <div class="info-row"><dt>UID contoh</dt><dd>${escapeHtml(d.uid.original)}</dd></div>
          <div class="info-row"><dt>Tampilan aktif</dt><dd id="uid-active">${escapeHtml(current)} <span class="badge ${d.uid.custom === null ? "" : "badge-good"}">${d.uid.custom === null ? "Bawaan" : "Kustom"}</span></dd></div>
        </dl>
      </section>
      <section class="uid-preview-card" aria-labelledby="uid-preview-title">
        <div class="preview-heading"><span class="eyebrow">KANVAS LANGSUNG</span><h2 id="uid-preview-title">Satu dunia. Versimu.</h2></div>
        <div class="uid-preview-stage">
          <span class="uid-preview-kicker">RUANG RESONANSI / ID</span><span class="preview-orbit" aria-hidden="true"></span>
          <div class="uid-preview-copy"><span>CATATAN SEORANG ROVER</span><strong>Jejak kecil.<br><em>Cerita besar.</em></strong></div>
          <div class="uid-preview-footer"><span>PRATINJAU ILUSTRATIF</span><span class="uid-preview-value">UID: <output id="uid-preview" for="uid-input">${escapeHtml(draft || d.uid.original)}</output></span></div>
        </div>
        <p class="muted">Bukan tangkapan layar game. Teks baru disimpan dalam simulasi setelah diterapkan.</p>
      </section>
    </div>
    <section class="simulation-panel" aria-label="Batas kustomisasi UID">${icon("info")}<div><strong>Ruang bereksperimen, bukan pengubah akun.</strong><p>Mock IPC hanya menyimpan teks tampilan. Tidak mengubah UID akun, menulis file game, atau menyembunyikan UID di game nyata.</p></div></section>`;
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
      name: "Periksa kembali",
      detail: `Status terakhir diperiksa: ${escapeHtml(new Date(d.lastChecked).toLocaleString("id-ID"))}`,
      action: "refresh",
      label: "Periksa",
      disabled: state.busy,
      icon: "refresh",
    },
    {
      name: "Perbaiki pemasangan",
      detail: d.patch.recoveryPending
        ? "Pemulihan tertunda; jalankan perbaikan."
        : "Memeriksa dan memperbaiki pemasangan yang dikelola aplikasi.",
      action: "repair",
      label: "Perbaiki",
      disabled: !canRepair,
      icon: "shield",
    },
    {
      name: "Ekspor diagnostik",
      detail:
        d.mode === "mock"
          ? "Buat laporan diagnostik simulasi untuk membantu pemeriksaan."
          : "Ekspor status perangkat, rilis GitHub, dan hasil pemeriksaan pemasangan.",
      action: "export-diagnostics",
      label: "Ekspor laporan",
      disabled: state.busy,
      icon: "download",
    },
    {
      name: "Hapus terjemahan",
      detail:
        "Menghapus patch yang dikelola aplikasi; file game asli tidak ditimpa.",
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
  return `<section class="page-heading"><p class="eyebrow">03 / PUSAT PERAWATAN</p><h1>Tetap siap. <em>Tetap tenang.</em></h1><p>Semua yang perlu diperiksa, dalam satu ruang.</p></section>
    <section class="health-banner" aria-label="Kesiapan sistem"><span class="health-icon ${status.ready ? "healthy" : "needs-attention"}">${icon(status.ready ? "shield" : "info")}</span><div><p class="eyebrow">KONDISI SAAT INI</p><h2>${escapeHtml(status.label)}</h2><p class="muted">${escapeHtml(status.detail)}</p></div><button class="icon-btn" data-action="refresh" aria-label="Periksa ulang kondisi sistem" ${state.busy ? "disabled" : ""}>${icon("refresh")}</button></section>
    <div class="section-heading"><h2>Kontrol pemasangan</h2><span class="section-index">AMAN & TERKELOLA</span></div>
    <section class="tool-list" aria-label="Tindakan perawatan">${rows}</section>
    <div class="tools-detail-grid">
      <section class="card"><div class="card-header"><div><p class="eyebrow">CATATAN TEKNIS</p><h2>Di balik layar</h2></div>${icon("folder")}</div><dl class="info-list">
        <div class="info-row"><dt>Game</dt><dd>${d.game.installed ? "Terpasang" : "Belum ditemukan"} · ${escapeHtml(d.game.version)}<code>${escapeHtml(d.game.packageName)}</code></dd></div>
        <div class="info-row"><dt>Shizuku</dt><dd>${d.shizuku.running ? "Berjalan" : "Tidak berjalan"} · ${d.shizuku.granted ? "Izin diberikan" : "Izin belum diberikan"}</dd></div>
        <div class="info-row"><dt>Resource</dt><dd>${d.resource.ready ? "Siap" : "Belum siap"} · ${escapeHtml(d.resource.version)}</dd></div>
        <div class="info-row"><dt>Terjemahan</dt><dd>${d.patch.installed ? "Terpasang" : "Belum terpasang"} · ${d.patch.healthy ? "Sehat" : "Perlu pemeriksaan"} · ${escapeHtml(d.patch.version)}${d.patch.recoveryPending ? " · Pemulihan tertunda" : ""}</dd></div>
        <div class="info-row"><dt>Rilis latest</dt><dd>${escapeHtml(d.mode === "mock" ? "3.7.0-R2 (contoh)" : (d.release?.tag ?? "Belum dimuat"))}</dd></div>
        ${d.uidAvailable ? `<div class="info-row"><dt>Tampilan UID</dt><dd>${escapeHtml(d.uid.custom ?? d.uid.original)} · ${d.uid.custom === null ? "Bawaan" : "Kustom"} (simulasi)</dd></div>` : ""}
        ${d.patch.message ? `<div class="info-row"><dt>Pemasangan</dt><dd>${escapeHtml(d.patch.message)}</dd></div>` : ""}
      </dl></section>
      <div class="tools-aside">
        <section class="simulation-panel" aria-label="Mode operasi">${icon("shield")}<div><strong>${d.mode === "mock" ? "Lingkungan simulasi" : "Pemasangan native"}</strong><p>${d.mode === "mock" ? "Semua tindakan browser menggunakan data contoh. Tidak ada file game yang ditulis atau game yang dijalankan." : "Rilis latest TitoTFP/WuwaID diperiksa dengan SHA-256 sebelum ditulis melalui Shizuku. Hanya paket dan mount milik aplikasi yang dikelola."}</p></div></section>
        ${d.mode === "mock" ? `<section class="card scenario-card"><div class="card-header"><div><p class="eyebrow">LAB ANTARMUKA</p><h2>Coba kondisi lain</h2></div>${icon("game")}</div><p class="muted">Lihat respons antarmuka saat perangkat siap, izin belum diberikan, atau pemasangan perlu dipulihkan.</p><label class="field-label" for="scenario-select">Skenario simulasi</label><select class="select" id="scenario-select"${state.busy ? " disabled" : ""}>${scenarioOptions}</select></section>` : ""}
      </div>
    </div>`;
}
