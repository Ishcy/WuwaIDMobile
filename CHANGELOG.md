# Changelog

## Unreleased

- Expand current-state regression coverage (WM-27) with 31 frontend, 9 Rust and
  7 Android host tests for access/readiness, UID and maintenance gates, verified
  PAK cache reuse, unsafe metadata, staging cleanup, collisions, fresh hashes,
  first-install rollback and privileged storage boundaries. All 108 host tests
  pass, alongside real browser UI and local PAK-engine CLI smoke. Prepare six
  additional Android instrumentation cases (eight total), compile and package
  their APK against the current ARM64 debug build, and document user-only device
  execution. Device tests are not reported as passed; production behavior and the
  accepted release APK remain unchanged.
  Keep cache regression assertions focused on verified paths and bytes rather
  than filesystem modification timestamps.

- Add the Android application updater (WM-26), separate from translation PAK
  releases: check stable WuwaIDMobile releases on startup or manually, show an
  in-app update banner, stream and verify APK downloads with progress, then open
  the Android installer with explicit user confirmation. Handle unknown-source
  permission return, cancellation, interrupted downloads and verified cache
  reuse without root/Shizuku or game-file mutation. Enforce release identity,
  higher versionCode, pinned signer, supported minSdk/ARM64, bounded HTTPS hosts
  and size/SHA-256 checks. Narrow FileProvider to the updater cache. Disable
  release updates on debug/browser/desktop builds; retain UID focus during
  background check completion. Publish QA-approved 0.x releases as stable so
  the updater can discover them. The user subsequently reported device testing
  safe and accepted the candidate for commit; detailed scenario coverage was not
  supplied.
- Add WM-26 behavior coverage: 27 Kotlin/Robolectric cases with locally served
  TLS downloads and SDK-built, ephemeral-key-signed APKs; 10 type-checked DOM/UI
  cases; CI and release execution. Exercise cache integrity, unsafe metadata,
  APK identity/signatures, permissions, cancellation and lifecycle. Fix the
  Android 9 archive parser rejecting valid APKs by requesting GET_SIGNATURES
  alongside GET_SIGNING_CERTIFICATES. Preserve the failing-before/passing-after
  regression. Browser smoke and 24 existing Rust tests pass; the assistant did
  not run the real Android installer or game tests. Device acceptance is recorded
  from the user's report, separately from host-test evidence.

- Fix the launcher icon appearing over-zoomed on the user's device. Use a separate
  Android foreground SVG with centered 72×72 artwork inside the 108×108 adaptive
  canvas, preserving transparent padding through icon regeneration. Enlarge the
  initial 66×66 correction by about 9% after user feedback about white borders.
  Keep original desktop/legacy artwork and app identity unchanged. Verify padded
  foregrounds in all five densities inside the signed APK and inspect simulated
  masks. The user accepted the final framing after a successful device update.

- Restore the display name **WuwaID Mobile** in Tauri and Android app/activity
  labels. Replace template icons with the pinned WuwaIDLauncher artwork, regenerate
  desktop and Android legacy/adaptive icons, and remove unused default drawables.
  Keep package/version/signing identity unchanged. Build a new signed ARM64 APK;
  verify final labels, all five-density launcher/foreground image pixels, original
  certificate, v2 signing and 16 KiB alignment. Record the user's device acceptance
  for the latest branding APK separately from the preceding WM-25 candidate.

- Preserve the published Android release identity for the Tauri rebuild:
  version 0.4.0 / versionCode 8, original package and pinned release certificate.
  Restore original-keystore signing inputs and reject incompatible release signing.
  Isolate debug APKs with `.debug`; require explicit manual upgrade acceptance and
  APK identity/certificate verification in the signed release workflow.
- Detect pre-Tauri `wuwaindonesia` installations without mutating during refresh.
  Add explicit offline migration using pinned historical release SHA-256/size,
  exact mount/SHA-1 verification and the existing transaction engine. Retain old
  sources until the new owned set commits; journal hash-checked resumable cleanup.
  Refuse ambiguous layouts, unknown payloads and orphan legacy transaction files.
  Remove verified inactive-resource sets and uninstall owned sets across resources.
- Restore Galaxy Store game discovery, storage routing and launch/force-stop,
  preserve Global-first materialized selection, and freeze the selected package
  for the app session across Root/Shizuku switches.
- Compile frontend, Rust (including existing test targets without execution),
  Kotlin/AIDL and debug ARM64 APK; inspect all seven historical published APK
  identities/certificates and verify debug APK v2 signing and 16 KiB alignment.
  Build a release ARM64 upgrade candidate with the original keystore subsequently
  supplied by the user; verify final package/version, the original certificate,
  v2 signing and 16 KiB alignment. No assistant tests were run for that WM-25 handoff.
  The user reported successful release APK installation and completed device QA
  with safe, correct operation; WM-25 is accepted on that confirmation. No
  exhaustive scenario coverage, pre-upgrade version, official-file hash comparison
  or quantitative measurements were supplied. No runtime changes after acceptance.

- Support direct root access through an explicit `su` permission request and a
  non-daemon libsu 6.0.0 Binder service, without requiring Shizuku.
  Reuse the guarded storage engine, PFD streaming, ownership, and transactions.
- Add persistent Root/Shizuku selection in maintenance and selected-access
  readiness across install, UID, repair, uninstall, and diagnostics.
  Passive refresh never requests root; service loss fails without automatic
  reconnect or transport fallback. Release pending binds on lifecycle teardown.
- Verify 24 Rust tests including selected-access transitions and game/resource
  gates, eight host-adapted root lifecycle cases, and root-only browser
  Apply/Reset/removal/switching. Build the ARM64 debug APK and verify its
  v2 signature and 16 KiB alignment. The user completed actual-device testing
  and reported safe, correct operation; no quantitative device measurements
  or exhaustive scenario/hash evidence were supplied.

- Read root probes, directory listings, batched stat metadata, bounded text, and
  streaming hashes directly inside the privileged Shizuku UserService (API 24+).
  Remove their shell scripts, input pipes, and per-request threads; keep mutation
  allowlists, path/symlink guards, ownership, journals, rollback, and recovery.
- Return SHA-256, optional SHA-1, and size from the same verified file stream.
  Batch/deduplicate lifecycle inspection, avoid discarded recovery inspections,
  and remove overlapping committed-state checks without cross-operation caches.
- Close every privileged read/hash descriptor explicitly with `Os.close` in
  `finally`, including rejected reads. Android's borrowed-descriptor
  `FileInputStream` does not own or close caller FDs; retain regression coverage.
- Add native-gated debug phase/correlation markers across frontend busy state,
  permission dialog, service binding, storage, network/cache, UID, and transactions.
  Release builds do not record UID text, payloads, paths, or debug timings.
- Verify real-release host install, UID replacement, Reset, tamper refusal,
  repair/uninstall, Linux-syscall Kotlin metadata safety, and browser busy/idle.
  Actual-device performance/game verification remains with the user.

- Reuse the installer's verified terminal state for successful-operation
  dashboards instead of repeating resource discovery and full PAK inspection;
  keep fresh permission probes, explicit refresh/diagnostics, and next preflight.
- Reuse two Shizuku output workers and per-worker 64 KiB read buffers, serialize
  process execution to prevent paired-reader starvation, and shrink initial
  metadata buffers without changing output limits or storage guards.
- Share the blocking GitHub HTTP client/connection pool without caching release
  metadata or weakening redirect, timeout, size, or digest validation.
- Optimize the dev profile at level 2 for CPU-heavy PAK hashing/repacking in
  debug APKs while retaining debug symbols, assertions, and overflow checks.
  With the same real-release host fixture, debug install drops from 21.283s to
  0.560s and UID Apply from 33.318s to 1.019s; these are not device timings.
- Profile frontend, indexed UID SQLite, and the real 52,871,888-byte release PAK.
  Host smoke verifies UID/Reset/tamper detection and streaming/concurrent storage;
  real HTTPS smoke verifies latest, cache integrity and corrupted-cache re-download.
  Device performance tests remain exclusively with the user.

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
