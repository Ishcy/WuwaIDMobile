import { api } from "./ipc";
import type { AppUpdateStatus, Progress } from "./model";
import { escapeHtml as esc, icon } from "./ui";

let status: AppUpdateStatus | undefined;
let pending: "check" | "download" | "install" | undefined;
let progress: Progress | null = null;
let failure = "";
let changed: () => void;
let initialized = false;

// A background version check must not delay access to game storage.
export function appUpdateBusy(): boolean {
  return pending === "download" || pending === "install";
}

function showProgress(value: Progress) {
  progress = value;
  const card = document.querySelector<HTMLElement>("#app-update-progress");
  if (!card) return;
  card.querySelector("strong")!.textContent = value.label;
  card.querySelector("div > span")!.textContent = `${value.percent}%`;
  const track = card.querySelector<HTMLElement>(".progress-track")!;
  track.setAttribute("aria-valuenow", String(value.percent));
  track.querySelector<HTMLElement>(".progress-fill")!.style.transform = `scaleX(${value.percent / 100})`;
}

async function run(action: "check" | "download" | "install") {
  if (pending) return;
  pending = action;
  failure = "";
  progress = action === "check" ? null : { percent: 0, label: "Menyiapkan pembaruan aplikasi…" };
  changed();
  try {
    status = await api.appUpdate(action, showProgress);
  } catch (cause) {
    failure = cause instanceof Error ? cause.message : String(cause);
    if (action !== "check") {
      try {
        status = await api.appUpdate("status");
      } catch (refreshError) {
        failure += ` Status updater belum dapat dibaca ulang: ${String(refreshError)}`;
      }
    }
  } finally {
    pending = undefined;
    progress = null;
    changed();
  }
}

export async function initAppUpdates(render: () => void) {
  changed = render;
  if (initialized) return;
  initialized = true;
  try {
    status = await api.appUpdate("status");
    changed();
    if (status.supported) await run("check");
  } catch (cause) {
    failure = cause instanceof Error ? cause.message : String(cause);
    changed();
  }
}

export async function handleAppUpdate(action: string | undefined, gameBusy: boolean): Promise<boolean> {
  const operation = action === "app-update-check" ? "check"
    : action === "app-update-download" ? "download"
      : action === "app-update-install" ? "install" : undefined;
  if (!operation) return false;
  if (!gameBusy && !pending) await run(operation);
  return true;
}

export function renderAppUpdate(gameBusy: boolean, details: boolean): string {
  if (!details && !status?.availableVersion && !appUpdateBusy()) return "";
  const disabled = gameBusy || !!pending ? "disabled" : "";
  const version = status?.availableVersion;
  const supported = status?.supported !== false;
  const size = status?.size ? `${(status.size / (1024 * 1024)).toFixed(1)} MiB` : "";
  return `<section class="card app-update-card" aria-labelledby="app-update-title">
    <div class="card-header"><h2 id="app-update-title">Pembaruan aplikasi</h2>${icon("download")}</div>
    <div role="status" aria-live="polite">
      ${status?.currentVersion ? `<p class="muted">Terpasang: ${esc(status.currentVersion)}${status.currentVersionCode !== null ? ` · kode ${status.currentVersionCode}` : ""}</p>` : ""}
      ${version ? `<p><strong>Versi ${esc(version)} tersedia${size ? ` · ${size}` : ""}.</strong></p>` : ""}
      <p class="muted">${esc(pending === "check" ? "Memeriksa versi aplikasi di GitHub…" : status?.message ?? "Belum memeriksa pembaruan aplikasi.")}</p>
    </div>
    ${failure ? `<p class="app-update-error" role="alert">${esc(failure)}</p>` : ""}
    <div class="actions">
      <button class="btn btn-secondary" data-action="app-update-check" ${disabled} ${supported ? "" : "disabled"}>${icon("refresh")} Periksa pembaruan</button>
      ${supported && version ? `<button class="btn btn-primary" data-action="${status?.ready ? "app-update-install" : "app-update-download"}" ${disabled}>${icon(status?.ready ? "external" : "download")} ${status?.ready ? "Pasang update" : "Unduh APK"}</button>` : ""}
    </div>
    ${progress ? `<div class="progress-card" id="app-update-progress" role="status" aria-live="polite"><div><strong>${esc(progress.label)}</strong><span>${progress.percent}%</span></div><div class="progress-track" role="progressbar" aria-label="Progres pembaruan aplikasi" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${progress.percent}"><span class="progress-fill" style="transform: scaleX(${progress.percent / 100})"></span></div></div>` : ""}
  </section>`;
}
