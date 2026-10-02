export type Section = "home" | "customize" | "tools";
export type Scenario =
  | "ready"
  | "installed"
  | "game_missing"
  | "unsupported"
  | "resource_missing"
  | "shizuku_off"
  | "permission"
  | "recovery";
export type Operation =
  | "install"
  | "repair"
  | "uninstall"
  | "apply_uid"
  | "restore_uid"
  | "launch_game"
  | "request_permission";
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
  patch: {
    installed: boolean;
    healthy: boolean;
    recoveryPending: boolean;
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
    d.shizuku.running &&
    d.shizuku.granted &&
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
        "Pemasangan game menggunakan Shizuku pada Android. Browser hanya menyediakan pratinjau simulasi.",
    };
  if (!d.game.installed)
    return {
      ready: false,
      label: "Game belum ditemukan",
      detail: "Pasang Wuthering Waves Global, lalu periksa kembali perangkat.",
    };
  if (
    d.mode === "native" &&
    d.shizuku.running &&
    d.shizuku.granted &&
    !d.release
  )
    return {
      ready: false,
      label: "Rilis belum tersedia",
      detail:
        d.blocker ?? "Periksa koneksi, lalu muat ulang rilis terbaru WuwaID.",
    };
  if (!d.shizuku.running)
    return {
      ready: false,
      label: "Shizuku belum aktif",
      detail:
        "Jalankan Shizuku terlebih dahulu untuk mengakses direktori game.",
    };
  if (!d.shizuku.granted)
    return {
      ready: false,
      label: "Izin diperlukan",
      detail: "Berikan izin Shizuku sebelum memasang terjemahan.",
    };
  if (!compatibleGameVersion(d, d.game.version))
    return {
      ready: false,
      label: "Versi belum didukung",
      detail:
        d.mode === "mock"
          ? "Paket simulasi ini memerlukan game dan resource versi 3.7.0."
          : `Rilis ${d.release?.tag ?? "terbaru"} mendukung lini game ${d.release?.gameVersion.split(".").slice(0, 2).join(".") ?? "yang tercantum di rilis"}.`,
    };
  if (!d.resource.ready || !compatibleGameVersion(d, d.resource.version))
    return {
      ready: false,
      label: "Resource belum siap",
      detail:
        d.blocker ??
        "Selesaikan unduhan resource di dalam game, lalu periksa kembali.",
    };
  if (d.blocker || !d.storageWritable)
    return {
      ready: false,
      label: "Pemasangan diblokir",
      detail:
        d.blocker ?? "Shizuku tidak dapat menulis direktori resource game.",
    };
  if (d.patch.recoveryPending || (d.patch.installed && !d.patch.healthy))
    return {
      ready: false,
      label: "Perlu pemulihan",
      detail: "Pulihkan pemasangan melalui menu Perawatan sebelum melanjutkan.",
    };
  return {
    ready: true,
    label: d.patch.installed ? "Terjemahan terpasang" : "Siap dipasang",
    detail:
      "Game, izin Shizuku, dan resource kompatibel dengan paket terjemahan.",
  };
}
