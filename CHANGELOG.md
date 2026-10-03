# Changelog

## Unreleased

- Fix pending Shizuku permission requests hanging when the server binder dies;
  invalidate cached user-service sessions and wake blocked binds on lifecycle loss.
- Isolate callbacks per bind attempt so stale connection/disconnection events
  cannot replace a newer session; clean failed, timed-out, and destroyed sessions.
- Require Shizuku server API 12+, handle permission rationale with actionable
  guidance, and replace API-26 cache symlink checks with API-21 `Os.lstat`.
- Verify Kotlin bridge behavior using a host-adapted process/pipe smoke,
  Android APK build/signature, packaged provider manifest, and 20 Rust tests.
  Bridge `NewApi` lint errors are resolved; existing TV manifest lint errors remain.
  Real-device Shizuku/Binder/SELinux verification was unavailable without ADB.

- Remove the installation safety copy and custom UID preview, including its
  update handlers and unused styles; keep a single-column UID editor.
- Allow custom UID Apply before a manual translation install: reuse the verified
  release/cache and transactional installer to install translation plus UID in
  one operation. Keep device, permission, release, resource, and recovery gates.
  Align browser simulation with the native fresh-install transition.
- Verify fresh/already-installed Apply and Reset in browser and a disk-backed
  PAK v12/SQLite smoke fixture; build and signature-check the ARM64 debug APK.
  No connected ADB device was available to repeat Android gameplay verification.

- Implement native Android custom UID and Reset using verified release PAKs,
  SQLite updates to the three launcher UID targets, and verified PAK v12 repacking.
  Preserve other translations, original release integrity, and official game files.
- Track source/derived hashes and UID selection in transactional ownership metadata;
  retain selections on repair/reinstall and restore previous selections on rollback.
  Reuse the existing Shizuku bridge without changing account/server UID.
- Enable the native UID editor and keep browser/native validation aligned with
  the 32-unit UTF-16 input limit; reject blank and control-character input.
- Verify native Apply and Reset in Android gameplay on SM-A525F: exact
  `Rover Indonesia` HUD text, restored original UID/release PAK, and identical
  SHA-256 checksums for all 443 baseline official resource files.

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
  simulation and custom UID actions, with explicit desktop platform limits.

- Build the Indonesian Beranda, Kustomisasi, and Perawatan frontend with responsive
  desktop/sidebar and mobile/bottom navigation.
- Add explicitly simulated Tauri IPC for device scenarios, translation install and
  repair, permission flow, custom UID display, and JSON diagnostics; persist mock state.
- Replace font customization with custom UID input, apply/restore
  actions, and persisted display state without changing the account UID.
- Add operation progress and gating, and uninstall confirmation with UID restoration.
- Remove the unused greeting form and Rust greeting command.

- Replace the previous Android launcher with a minimal Tauri 2 desktop/Android shell.
- Add GitHub CI for frontend, Rust, and Android debug APK builds.
