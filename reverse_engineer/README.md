# Reverse Engineering — VHWuWa → rebuild Indonesia

Target APK: `VHWuWa-Android-0.2.0-alpha.9-3.7.0-R2.apk`, package
`vn.wahu.vhwuwa`. This reference set documents APK analysis; no rebuild code is
included.

## Ringkasan

- Native Android Kotlin + Jetpack Compose Material 3; not Tauri, WebView, or
  React Native. The app installs localization assets for Wuthering Waves Global
  through Shizuku and registers them with the game resource mount descriptor.
  It does not overwrite original game files.
- The app has no configured update endpoint. Its only APK URL is the Google
  Play listing; the updater implementation is not wired into this build.
- UI copy is hardcoded in Kotlin. The complete 393-string corpus is
  [`strings/vi-strings.json`](strings/vi-strings.json) and
  [`strings/vi-strings.tsv`](strings/vi-strings.tsv).
- `core.InstallMarker` and `core.InstallationIntegrityRules` have no references
  outside `core/`; install ownership is enforced by
  `TransactionalPatchEngine.ownership()` using descriptor contents and baseline
  hashes. Do not port the unread `VHWUWA_INSTALL_MARKER_V1` format.

## Referensi

| File | Fokus |
|---|---|
| [`01-apk-facts.md`](01-apk-facts.md) | APK, manifest, permissions, dex, fonts, payload hashes |
| [`02-architecture.md`](02-architecture.md) | Modul, state, navigasi, kompatibilitas, manifest |
| [`03-install-protocol.md`](03-install-protocol.md) | Game paths, payloads, mount descriptor, ownership, transaksi |
| [`04-ui-and-strings.md`](04-ui-and-strings.md) | Perilaku layar, label penting, port lokalisasi |
| [`06-shizuku-backend.md`](06-shizuku-backend.md) | Argumen shell, backend storage, path guard, penulisan atomik |
| [`07-transaction-engine.md`](07-transaction-engine.md) | Marker JSON, staging, commit, rollback, verifikasi |
| [`08-discovery-compat-design.md`](08-discovery-compat-design.md) | Resource discovery, urutan kompatibilitas, design token |
| [`tools.md`](tools.md) | Prasyarat dan perintah reproduksi analisis |
| [`strings/vi-strings.json`](strings/vi-strings.json) | Korpus Vietnam, terstruktur |
| [`strings/vi-strings.tsv`](strings/vi-strings.tsv) | Korpus Vietnam, TSV |

## Jalur cepat port

| Perlu | Baca |
|---|---|
| Path game, file terkelola, mount descriptor | `03-install-protocol.md` §1–3 |
| State install, ownership, rollback | `03-install-protocol.md` §6; `07-transaction-engine.md` |
| Struktur layar, navigasi, teks UI | `02-architecture.md` §Navigasi; `04-ui-and-strings.md`; string corpus |
| Model domain dan aturan versi | `02-architecture.md` §Aturan kompatibilitas |
| Perintah Shizuku dan path guard | `06-shizuku-backend.md` |
| Pemilihan folder resource dan warna | `08-discovery-compat-design.md` §1, §4 |

## Batas analisis dan hal yang belum diketahui

- Isi `.pak` tidak dianalisis. Format V12 dari `installer.NativeV12PakBuilder`
  hanya dipetakan lewat signature dan nama artifact, bukan struktur payload.
- Modul `updater` memiliki verifikasi Ed25519 tetapi tidak punya call site
  `SignedManifestLoader.<init>` atau base URL pada APK ini; endpoint update tidak
  dapat dipulihkan.
- `WuWaMobileController` menggunakan `ShizukuStorageAdapter` tanpa kondisi;
  Shizuku wajib untuk operasi storage sejak awal.
- Tipografi/shape Compose (08 §4) dan tabel `readinessTone` untuk
  `MobileReadiness` belum dibongkar.