// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AppUpdateStatus, Progress } from "../src/model";
import type * as AppUpdateModule from "../src/app-update";

const transport = vi.hoisted(() => vi.fn());
vi.mock("../src/ipc", () => ({ api: { appUpdate: transport } }));

const installed: AppUpdateStatus = {
  supported: true, currentVersion: "0.4.0", currentVersionCode: 8,
  availableVersion: null, size: 0, ready: false, checked: false, message: "",
};
const available: AppUpdateStatus = {
  ...installed, availableVersion: "0.4.1", size: 10 * 1024 * 1024, checked: true,
};
const cached: AppUpdateStatus = { ...available, ready: true };
let updater: typeof AppUpdateModule;
let render: () => void;

beforeEach(async () => {
  vi.resetModules();
  transport.mockReset();
  // Each case exercises initialization of fresh module-scoped state; a static import cannot reset it.
  updater = await import("../src/app-update");
  document.body.innerHTML = '<div id="updates"></div>';
  render = () => { document.querySelector("#updates")!.innerHTML = updater.renderAppUpdate(false, true); };
  transport.mockImplementation((action: string) => Promise.resolve(action === "status" ? installed : available));
});

describe("updater state presented to the user", () => {
  it("keeps a background check non-blocking and reveals a newly available update", async () => {
    const check = Promise.withResolvers<AppUpdateStatus>();
    transport.mockImplementation((action: string) => action === "status" ? Promise.resolve(installed) : check.promise);
    const initialization = updater.initAppUpdates(render);
    await vi.waitFor(() => expect(document.querySelector<HTMLButtonElement>("[data-action=app-update-check]")!.disabled).toBe(true));
    expect(updater.appUpdateBusy()).toBe(false);
    expect(updater.renderAppUpdate(false, false)).toBe("");
    check.resolve(available);
    await initialization;
    expect(updater.renderAppUpdate(false, false)).not.toBe("");
    expect(document.querySelector<HTMLButtonElement>("[data-action=app-update-download]")!.disabled).toBe(false);
  });

  it("does not offer install or make network checks on an unsupported build", async () => {
    transport.mockResolvedValue({ ...installed, supported: false });
    await updater.initAppUpdates(render);
    expect(document.querySelector<HTMLButtonElement>("[data-action=app-update-check]")!.disabled).toBe(true);
    expect(document.querySelector("[data-action=app-update-install]")).toBeNull();
    expect(transport).toHaveBeenCalledTimes(1);
    expect(updater.appUpdateBusy()).toBe(false);
  });

  it("blocks overlapping downloads and PAK activity until a verified APK is ready", async () => {
    const download = Promise.withResolvers<AppUpdateStatus>();
    transport.mockImplementation((action: string) => action === "download" ? download.promise
      : Promise.resolve(action === "status" ? installed : available));
    await updater.initAppUpdates(render);
    const downloading = updater.handleAppUpdate("app-update-download", false);
    expect(updater.appUpdateBusy()).toBe(true);
    expect(document.querySelector<HTMLButtonElement>("[data-action=app-update-download]")!.disabled).toBe(true);
    await updater.handleAppUpdate("app-update-download", false);
    await updater.handleAppUpdate("app-update-check", false);
    expect(transport.mock.calls.filter(([action]) => action === "download")).toHaveLength(1);
    download.resolve(cached);
    await downloading;
    expect(updater.appUpdateBusy()).toBe(false);
    expect(document.querySelector("[data-action=app-update-download]")).toBeNull();
    expect(document.querySelector<HTMLButtonElement>("[data-action=app-update-install]")!.disabled).toBe(false);
  });

  it("ignores updater actions during a game operation and disables the visible actions", async () => {
    await updater.initAppUpdates(render);
    document.querySelector("#updates")!.innerHTML = updater.renderAppUpdate(true, true);
    expect([...document.querySelectorAll<HTMLButtonElement>("button")].every(button => button.disabled)).toBe(true);
    await updater.handleAppUpdate("app-update-download", true);
    expect(transport.mock.calls.filter(([action]) => action === "download")).toHaveLength(0);
    expect(updater.appUpdateBusy()).toBe(false);
    expect(await updater.handleAppUpdate("apply_uid", false)).toBe(false);
  });

  it("updates accessible progress in place without replacing focused DOM nodes", async () => {
    const download = Promise.withResolvers<AppUpdateStatus>();
    let progress!: (value: Progress) => void;
    transport.mockImplementation((action: string, callback?: (value: Progress) => void) => {
      if (action === "download") { progress = callback!; return download.promise; }
      return Promise.resolve(action === "status" ? installed : available);
    });
    await updater.initAppUpdates(render);
    const downloading = updater.handleAppUpdate("app-update-download", false);
    const card = document.querySelector("#app-update-progress")!;
    const input = document.createElement("input");
    document.body.append(input);
    input.focus();
    input.value = "123456789";
    progress({ percent: 49, label: "<img src=x onerror=alert(1)>" });
    expect(document.querySelector("#app-update-progress")).toBe(card);
    expect(document.activeElement).toBe(input);
    expect(input.value).toBe("123456789");
    expect(card.querySelector("strong")!.textContent).toBe("<img src=x onerror=alert(1)>");
    expect(card.querySelector("img")).toBeNull();
    expect(card.querySelector("[role=progressbar]")!.getAttribute("aria-valuenow")).toBe("49");
    expect(card.querySelector<HTMLElement>(".progress-fill")!.style.transform).toBe("scaleX(0.49)");
    download.resolve(cached);
    await downloading;
    expect(document.querySelector("#app-update-progress")).toBeNull();
  });

  it("releases the busy lock after failure, refreshes readiness and permits another download", async () => {
    transport.mockImplementation((action: string) => action === "download"
      ? Promise.reject(new Error("Network interrupted")) : Promise.resolve(available));
    await updater.initAppUpdates(render);
    await updater.handleAppUpdate("app-update-download", false);
    expect(updater.appUpdateBusy()).toBe(false);
    expect(document.querySelector("[role=alert]")!.textContent).toContain("Network interrupted");
    expect(document.querySelector<HTMLButtonElement>("[data-action=app-update-download]")!.disabled).toBe(false);
    transport.mockResolvedValue(cached);
    await updater.handleAppUpdate("app-update-download", false);
    expect(document.querySelector("[role=alert]")).toBeNull();
    expect(document.querySelector("[data-action=app-update-install]")).not.toBeNull();
  });

  it("keeps a cached APK available after denied permission or installer cancellation", async () => {
    transport.mockResolvedValue(cached);
    await updater.initAppUpdates(render);
    transport.mockImplementation((action: string) => action === "install"
      ? Promise.reject(new Error("Permission denied")) : Promise.resolve(cached));
    await updater.handleAppUpdate("app-update-install", false);
    expect(updater.appUpdateBusy()).toBe(false);
    expect(document.querySelector<HTMLButtonElement>("[data-action=app-update-install]")!.disabled).toBe(false);
    transport.mockResolvedValue({ ...cached, message: "Cancelled" });
    await updater.handleAppUpdate("app-update-install", false);
    expect(document.querySelector("[role=alert]")).toBeNull();
    expect(document.querySelector("[data-action=app-update-install]")).not.toBeNull();
  });

  it("retains the original error when reading post-failure status also fails", async () => {
    await updater.initAppUpdates(render);
    transport.mockRejectedValue(new Error("Offline"));
    await updater.handleAppUpdate("app-update-download", false);
    expect(updater.appUpdateBusy()).toBe(false);
    expect(document.querySelector("[role=alert]")!.textContent).toContain("Offline");
    expect(document.querySelector<HTMLButtonElement>("[data-action=app-update-download]")!.disabled).toBe(false);
  });

  it("escapes release, installed-version, status and failure strings", async () => {
    const injected = '<img src=x onerror="alert(1)">';
    transport.mockResolvedValue({ ...available, currentVersion: injected, availableVersion: injected, message: injected });
    await updater.initAppUpdates(render);
    expect(document.querySelector("#updates img")).toBeNull();
    expect(document.querySelector("#updates")!.textContent).toContain(injected);
    transport.mockRejectedValue(new Error(injected));
    await updater.handleAppUpdate("app-update-check", false);
    expect(document.querySelector("[role=alert]")!.textContent).toBe(injected);
    expect(document.querySelector("#updates img")).toBeNull();
  });

  it("allows a manual check after initialization failed", async () => {
    transport.mockRejectedValue(new Error("Initial status unavailable"));
    await updater.initAppUpdates(render);
    expect(document.querySelector("[role=alert]")).not.toBeNull();
    transport.mockResolvedValue(available);
    await updater.handleAppUpdate("app-update-check", false);
    expect(document.querySelector("[role=alert]")).toBeNull();
    expect(document.querySelector("[data-action=app-update-download]")).not.toBeNull();
  });
});
