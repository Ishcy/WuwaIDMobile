# Changelog

## Unreleased

- Add restrained 140–180 ms navigation, control, theme, disclosure, dialog,
  and toast transitions; interpolate accessible progress without layout reflow.
  Honor live reduced-motion preferences and avoid replaying page entrances on
  status/progress updates.

- Simplify all three screens: remove promotional copy, decorative artwork,
  numbered labels, and repeated notices; use concise status/action text and
  collapsible technical details. Remove the package detail section from Beranda.
  Preserve themes and operation controls.

- Reimagine the frontend as an original ink/lavender editorial workspace with
  custom resonance artwork, horizontal desktop navigation, a mobile dock,
  a UID studio, and a maintenance control center; no reference APK UI/assets.
- Add persistent light, dark, and system themes with pre-paint initialization,
  live system preference tracking, and themed dialogs, forms, and notifications.
- Keep navigation/theme controls stable during operations and update progress
  in place so interaction and keyboard focus are not discarded by progress events.

- Fix Resource Mount entries to use extensionless PAK base paths, matching the
  launcher and Android bytecode rather than physical filenames.
- Mark old owned descriptors unhealthy and allow transactional repair or safe
  uninstall; preserve previous snapshots on failed format migration.
- Replace the descriptor wording assertion with file-resolution/hash checks and
  add previous-format migration, removal, rollback and unknown-format regression coverage.

- Add latest stable WuwaID GitHub release discovery and streaming PAK downloads
  with size/SHA-256 verification and re-verified private cache.
- Implement Android Shizuku UserService/AIDL storage access and PFD streaming,
  restricting writes to owned paths inside numeric game-resource versions.
- Add resource/signature discovery, separate priority-99 mounts, ownership
  metadata, transactional install/repair/uninstall, and exact snapshot recovery
  across resource updates without requiring release metadata.
- Use native Tauri IPC and progress events in the shell; retain browser-only
  simulation and custom UID previews, with explicit desktop platform limits.

- Build the Indonesian Beranda, Kustomisasi, and Perawatan frontend with responsive
  desktop/sidebar and mobile/bottom navigation.
- Add explicitly simulated Tauri IPC for device scenarios, translation install and
  repair, permission flow, custom UID display, and JSON diagnostics; persist mock state.
- Replace font customization with custom UID input, live preview, apply/restore
  actions, and persisted display state without changing the account UID.
- Add operation progress and gating, and uninstall confirmation with UID restoration.
- Remove the unused greeting form and Rust greeting command.

- Replace the previous Android launcher with a minimal Tauri 2 desktop/Android shell.
- Add GitHub CI for frontend, Rust, and Android debug APK builds.
