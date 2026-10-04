import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AppUpdateStatus,
  Dashboard,
  Diagnostics,
  Operation,
  Progress,
  Scenario,
} from "./model";

// Decide before importing mockIPC: mocks themselves populate Tauri's window internals.
const native = isTauri();
const progressListeners = new Set<(progress: Progress) => void>();
let recording = false;
let nextRequest = 0;
let busyRequest: number | undefined;
function progressReceived(progress: Progress) {
  for (const listener of progressListeners) listener(progress);
}
const ready = native
  ? Promise.all([
      listen<Progress>("wuwa-progress", (event) => progressReceived(event.payload)),
      invoke<boolean>("wuwa_trace_enabled").then((enabled) => { recording = enabled; }),
    ])
  : import("./mock-ipc").then((mock) =>
      mock.subscribeProgress(progressReceived),
    );

async function command<T>(
  name: string,
  args?: Record<string, unknown>,
): Promise<T> {
  await ready;
  const requestId = busyRequest ?? ++nextRequest;
  const result = await invoke<T>(name, { ...args, requestId });
  if (recording) await uiTrace(requestId, "response");
  return result;
}

async function uiTrace(requestId: number, event: "busy_start" | "busy_end" | "response", startedUs?: number) {
  if (!recording) return;
  const frontendUs = startedUs ?? Math.round(performance.now() * 1000);
  performance.mark(`wuwaid:${requestId}:${event}`, { startTime: frontendUs / 1000 });
  // Recording must never turn a completed operation into a failed one.
  await invoke<void>("wuwa_ui_trace", { requestId, event, frontendUs }).catch(() => {});
}

export const api = {
  async beginBusy() {
    const startedUs = recording ? Math.round(performance.now() * 1000) : undefined;
    await ready;
    busyRequest = ++nextRequest;
    await uiTrace(busyRequest, "busy_start", startedUs);
  },
  async endBusy() {
    const requestId = busyRequest;
    busyRequest = undefined;
    if (requestId !== undefined) await uiTrace(requestId, "busy_end");
  },
  load: () => command<Dashboard>("wuwa_load"),
  setScenario(scenario: Scenario) {
    if (native)
      return Promise.reject(
        new Error("Skenario simulasi hanya tersedia di browser."),
      );
    return command<Dashboard>("wuwa_set_scenario", { scenario });
  },
  run: (operation: Operation, uidText?: string) =>
    command<Dashboard>("wuwa_run", { operation, uidText }),
  diagnostics: () => command<Diagnostics>("wuwa_diagnostics"),
  openSetup: (target: "game" | "shizuku") =>
    command<void>("wuwa_open_setup", { target }),
  appUpdate(action: "status" | "check" | "download" | "install", onProgress?: (progress: Progress) => void): Promise<AppUpdateStatus> {
    if (!native) return Promise.resolve({
      supported: false, currentVersion: null, currentVersionCode: null,
      availableVersion: null, size: 0, ready: false, checked: false,
      message: "Updater APK hanya tersedia pada aplikasi Android release; browser tidak melakukan update aplikasi.",
    });
    const progress = new Channel<Progress>();
    progress.onmessage = (value) => onProgress?.(value);
    return command<AppUpdateStatus>("wuwa_app_update", { action, progress });
  },
  subscribeProgress(listener: (progress: Progress) => void): () => void {
    progressListeners.add(listener);
    return () => progressListeners.delete(listener);
  },
};
