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
    return `<section class="page-heading"><span class="eyebrow">Identitas di layar</span><h1>Custom UID</h1><p>Pratinjau kustomisasi tampilan UID.</p></section><section class="simulation-panel"><strong>Custom UID hanya tersedia di pratinjau browser</strong><p>Pemasangan PAK memakai backend native. Kustomisasi UID masih berupa simulasi browser; aplikasi tidak mengubah UID akun atau mengklaim sudah menerapkannya ke game.</p></section>`;
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

  return `<section class="page-heading"><span class="eyebrow">Identitas di layar</span><h1>Custom UID</h1><p>Buat tampilan UID sesuai gayamu, tanpa mengubah identitas akun.</p></section>
    <div class="uid-grid">
      <section class="card" aria-labelledby="uid-title">
        <div class="card-header"><div><p class="eyebrow">KUSTOMISASI TAMPILAN</p><h2 id="uid-title">UID versimu</h2></div><span class="badge ${canCustomize ? "badge-good" : "badge-warning"}">${canCustomize ? "Siap dikustomisasi" : "Perlu persiapan"}</span></div>
        <p class="muted">${escapeHtml(gate)}</p>
        <form id="uid-form">
          <label class="field-label" for="uid-input">Teks UID kustom</label>
          <input class="uid-input" id="uid-input" name="customUid" type="text" maxlength="${UID_MAX_LENGTH}" value="${escapeHtml(state.uidDraft)}" placeholder="Contoh: Rover Indonesia" autocomplete="off" spellcheck="false" aria-describedby="uid-help" ${state.busy ? "disabled" : ""} />
          <div class="uid-input-meta"><span id="uid-help">Teks atau angka · maksimal ${UID_MAX_LENGTH} karakter</span><span id="uid-count">${state.uidDraft.length}/${UID_MAX_LENGTH}</span></div>
          <div class="actions"><button class="btn btn-primary" id="apply-uid" type="submit" ${canApply ? "" : "disabled"}>${icon("check")} Terapkan UID</button>${button("restore-uid", `${icon("refresh")} Pulihkan UID bawaan`, !canCustomize || d.uid.custom === null)}</div>
        </form>
        <dl class="info-list uid-info">
          <div class="info-row"><dt>UID bawaan</dt><dd>${escapeHtml(d.uid.original)} <span class="muted">(contoh mock)</span></dd></div>
          <div class="info-row"><dt>Tampilan aktif</dt><dd id="uid-active">${escapeHtml(current)}<span class="badge ${d.uid.custom === null ? "" : "badge-good"}">${d.uid.custom === null ? "Bawaan" : "Kustom"}</span></dd></div>
        </dl>
      </section>
      <section class="card uid-preview-card" aria-labelledby="uid-preview-title">
        <div class="card-header"><div><p class="eyebrow">PRATINJAU LANGSUNG</p><h2 id="uid-preview-title">Di sudut petualanganmu</h2></div></div>
        <div class="uid-preview-stage">
          <span class="uid-preview-kicker">SOLARIS–3 / RUANG ROVER</span><div class="uid-preview-emblem" aria-hidden="true">${icon("game")}</div>
          <div class="uid-preview-copy"><span>IDENTITAS DI LAYAR</span><strong>Petualanganmu.<br>Sentuhanmu.</strong></div>
          <div class="uid-preview-footer"><span>MOCK PREVIEW</span><span class="uid-preview-value">UID: <output id="uid-preview" for="uid-input">${escapeHtml(draft || d.uid.original)}</output></span></div>
        </div>
        <p class="muted">Pratinjau ilustratif, bukan tangkapan layar game. Teks baru aktif dalam simulasi setelah kamu menekan Terapkan UID.</p>
      </section>
    </div>
    <section class="simulation-panel" aria-label="Batas kustomisasi UID"><strong>Hanya tampilan, bukan UID akun</strong><p>UID akun asli tetap sama. Mock IPC hanya menyimpan teks tampilan; tidak mengubah akun, menulis file game, atau menyembunyikan UID di game nyata.</p></section>`;
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
        `<div class="tool-row"><span class="tool-icon">${icon(row.icon)}</span><div class="tool-copy"><h3>${row.name}</h3><p class="muted">${row.detail}</p></div>${button(row.action, row.label, row.disabled, row.action === "uninstall" ? "btn btn-danger" : "btn btn-secondary")}</div>`,
    )
    .join("");
  return `<section class="page-heading"><span class="eyebrow">Status perangkat</span><h1>Perawatan</h1><p>Periksa kesiapan dan kelola pemasangan terjemahan.</p></section>
    <section class="simulation-panel" aria-label="Mode operasi"><strong>${d.mode === "mock" ? "Mode simulasi browser" : "Backend native Tauri"}</strong><p>${d.mode === "mock" ? "Operasi di panel ini hanya mensimulasikan tindakan. Tidak ada file game yang ditulis atau game yang dijalankan." : "PAK diunduh dari rilis latest TitoTFP/WuwaID. SHA-256 diverifikasi sebelum Shizuku menulis paket dan mount terpisah. Hanya berkas milik aplikasi yang dikelola."}</p></section>
    <section class="card"><div class="card-header"><div><h2>Kesiapan sistem</h2><p class="muted">${escapeHtml(status.detail)}</p></div><span class="badge ${status.ready ? "badge-good" : "badge-warning"}">${escapeHtml(status.label)}</span></div><div class="tool-list">${rows}</div></section>
    <section class="card"><div class="card-header"><div><h2>Informasi game dan resource</h2><p class="muted">${d.mode === "mock" ? "Rincian status simulasi." : "Rincian status perangkat dan pemasangan native."}</p></div></div><dl class="info-list">
      <div class="info-row"><dt>Game</dt><dd>${d.game.installed ? "Terpasang" : "Belum ditemukan"} · ${escapeHtml(d.game.version)} · <code>${escapeHtml(d.game.packageName)}</code></dd></div>
      <div class="info-row"><dt>Shizuku</dt><dd>${d.shizuku.running ? "Berjalan" : "Tidak berjalan"} · ${d.shizuku.granted ? "Izin diberikan" : "Izin belum diberikan"}</dd></div>
      <div class="info-row"><dt>Resource</dt><dd>${d.resource.ready ? "Siap" : "Belum siap"} · versi ${escapeHtml(d.resource.version)}</dd></div>
      <div class="info-row"><dt>Terjemahan</dt><dd>${d.patch.installed ? "Terpasang" : "Belum terpasang"} · ${d.patch.healthy ? "Sehat" : "Perlu pemeriksaan"} · versi ${escapeHtml(d.patch.version)}${d.patch.recoveryPending ? " · Pemulihan tertunda" : ""}</dd></div>
      <div class="info-row"><dt>Konten latest</dt><dd>${escapeHtml(d.mode === "mock" ? "3.7.0-R2 (contoh)" : (d.release?.tag ?? "Belum dimuat"))}</dd></div>
      ${d.uidAvailable ? `<div class="info-row"><dt>Tampilan UID</dt><dd>${escapeHtml(d.uid.custom ?? d.uid.original)} · ${d.uid.custom === null ? "Bawaan" : "Kustom"} (simulasi)</dd></div>` : ""}
      ${d.patch.message ? `<div class="info-row"><dt>Detail pemasangan</dt><dd>${escapeHtml(d.patch.message)}</dd></div>` : ""}
    </dl></section>
    ${d.mode === "mock" ? `<section class="card"><div class="card-header"><div><h2>Skenario simulasi</h2><p class="muted">Pilih kondisi contoh untuk melihat perilaku antarmuka. Ini tidak mengubah perangkat.</p></div></div><label class="field-label" for="scenario-select">Kondisi simulasi</label><select class="select" id="scenario-select"${state.busy ? " disabled" : ""}>${scenarioOptions}</select></section>` : ""}`;
}
