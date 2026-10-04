export type Section = "home" | "customize" | "tools";
export type Scenario =
  | "ready"
  | "installed"
  | "game_missing"
  | "unsupported"
  | "resource_missing"
  | "shizuku_off"
  | "permission"
  | "root_permission"
  | "root_ready"
  | "legacy"
  | "recovery";
export type Operation =
  | "install"
  | "repair"
  | "uninstall"
  | "apply_uid"
  | "restore_uid"
  | "launch_game"
  | "request_permission"
  | "request_root"
  | "use_root"
  | "use_shizuku";
export interface ReleaseInfo {
  tag: string;
  gameVersion: string;
  assetName: string;
  downloadUrl: string;
  htmlUrl: string;
  size: number;
  sha256: string;
}
export interface Dashboard {
  mode: "mock" | "native";
  platformSupported: boolean;
  storageWritable: boolean;
  blocker: string | null;
  release: ReleaseInfo | null;
  uidAvailable: boolean;
  game: { installed: boolean; version: string; packageName: string };
  resource: { ready: boolean; version: string };
  shizuku: { running: boolean; granted: boolean };
  access: { method: "shizuku" | "root"; granted: boolean };
  patch: {
    installed: boolean;
    healthy: boolean;
    recoveryPending: boolean;
    legacyPending: boolean;
    version: string;
    message?: string | null;
  };
  uid: { original: string; custom: string | null };
  lastChecked: string | number;
}
export interface Progress {
  percent: number;
  label: string;
}
export interface Diagnostics {
  filename: string;
  content: string;
}
export interface AppUpdateStatus {
  supported: boolean;
  currentVersion: string | null;
  currentVersionCode: number | null;
  availableVersion: string | null;
  size: number;
  ready: boolean;
  checked: boolean;
  message: string;
}
export interface UiState {
  dashboard: Dashboard;
  busy: boolean;
  progress: Progress | null;
  uidDraft: string;
}
export const CONTENT_VERSION = "3.7.0-R2";
export const UID_MAX_LENGTH = 32;

export function compatibleGameVersion(d: Dashboard, version: string): boolean {
  if (d.mode === "mock") return version === "3.7.0";
  return (
    !!d.release &&
    /^\d+\.\d+\.\d+$/.test(version) &&
    version.split(".").slice(0, 2).join(".") ===
      d.release.gameVersion.split(".").slice(0, 2).join(".")
  );
}

export function installationPrerequisites(d: Dashboard): boolean {
  return (
    d.platformSupported &&
    !d.blocker &&
    d.storageWritable &&
    d.game.installed &&
    compatibleGameVersion(d, d.game.version) &&
    d.access.granted &&
    d.resource.ready &&
    compatibleGameVersion(d, d.resource.version)
  );
}
export function readiness(d: Dashboard): {
  ready: boolean;
  label: string;
  detail: string;
} {
  if (!d.platformSupported)
    return {
      ready: false,
      label: "Memerlukan Android",
      detail:
        d.blocker ??
        "Pemasangan memerlukan Android dengan Shizuku atau root.",
    };
  if (!d.game.installed)
    return {
      ready: false,
      label: "Game belum ditemukan",
      detail: "Pasang Wuthering Waves Global atau Galaxy Store, lalu periksa ulang.",
    };
  if (d.access.method === "root" && !d.access.granted)
    return {
      ready: false,
      label: "Izin root diperlukan",
      detail: "Berikan izin root melalui pengelola root, atau pilih Shizuku di Perawatan.",
    };
  if (d.access.method === "shizuku" && !d.shizuku.running)
    return {
      ready: false,
      label: "Shizuku belum aktif",
      detail:
        "Jalankan Shizuku untuk mengakses file game.",
    };
  if (!d.access.granted)
    return {
      ready: false,
      label: "Izin diperlukan",
      detail: "Berikan izin Shizuku.",
    };
  if (d.resource.ready && (d.patch.legacyPending || d.patch.recoveryPending))
    return {
      ready: false,
      label: d.patch.recoveryPending ? "Perlu pemulihan" : "Pemasangan versi lama",
      detail: d.patch.message ?? "Jalankan migrasi atau pemulihan di Perawatan. Tidak perlu mengunduh rilis untuk tindakan ini.",
    };
  if (
    d.mode === "native" &&
    d.access.granted &&
    !d.release
  )
    return {
      ready: false,
      label: "Rilis belum tersedia",
      detail:
        d.blocker ?? "Periksa koneksi, lalu muat ulang rilis.",
    };
  if (!compatibleGameVersion(d, d.game.version))
    return {
      ready: false,
      label: "Versi belum didukung",
      detail:
        d.mode === "mock"
          ? "Game dan resource harus versi 3.7.0."
          : `Rilis ${d.release?.tag ?? "terbaru"} memerlukan game ${d.release?.gameVersion.split(".").slice(0, 2).join(".") ?? "sesuai rilis"}.`,
    };
  if (!d.resource.ready || !compatibleGameVersion(d, d.resource.version))
    return {
      ready: false,
      label: "Resource belum siap",
      detail:
        d.blocker ??
        "Selesaikan unduhan resource di game, lalu periksa ulang.",
    };
  if (d.blocker || !d.storageWritable)
    return {
      ready: false,
      label: "Pemasangan diblokir",
      detail:
        d.blocker ?? "Metode akses terpilih tidak dapat menulis direktori resource game.",
    };
  if (d.patch.recoveryPending || (d.patch.installed && !d.patch.healthy))
    return {
      ready: false,
      label: "Perlu pemulihan",
      detail: "Jalankan Perbaiki di Perawatan.",
    };
  return {
    ready: true,
    label: d.patch.installed ? "Terjemahan terpasang" : "Siap dipasang",
    detail:
      `Game, ${d.access.method === "root" ? "root" : "Shizuku"}, dan resource siap.`,
  };
}
