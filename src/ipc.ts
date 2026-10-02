import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Dashboard,
  Diagnostics,
  Operation,
  Progress,
  Scenario,
} from "./model";

// Decide before importing mockIPC: mocks themselves populate Tauri's window internals.
const native = isTauri();
const progressListeners = new Set<(progress: Progress) => void>();
function progressReceived(progress: Progress) {
  for (const listener of progressListeners) listener(progress);
}
const ready = native
  ? listen<Progress>("wuwa-progress", (event) =>
      progressReceived(event.payload),
    )
  : import("./mock-ipc").then((mock) =>
      mock.subscribeProgress(progressReceived),
    );

async function command<T>(
  name: string,
  args?: Record<string, unknown>,
): Promise<T> {
  await ready;
  return invoke<T>(name, args);
}

export const api = {
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
  subscribeProgress(listener: (progress: Progress) => void): () => void {
    progressListeners.add(listener);
    return () => progressListeners.delete(listener);
  },
};
