# 02 — Arsitektur

Nama file Kotlin asli dipulihkan dari metadata `SMAP` di
`reverse_engineer/out/raw/DebugProbesKt.bin`.

## Peta modul

```
vn.wahu.vhwuwa                 UI Compose + orkestrasi
├── MainActivity               entry point, portrait, setContent { FullFeatureMobileScreen }
├── WuWaMobileController       orchestrator: refresh / install / repair / uninstall / font
├── FullFeatureMobileScreen     root composable, 3 section, primary action, readiness banner
├── PremiumInstallerUi.kt       shared UI kit (WahuButtonKind, UiTone, header, card, status chip)
├── FontGalleryPanel.kt         katalog font + preview + import TTF/OTF/TTC
├── FontCatalogRepository       baca/tulis assets/font/catalog.json
├── DiagnosticsExporter        export JSON diagnostik
├── DebugInstallReceiver        receiver broadcast debug
│
├── core/                      domain murni, tanpa Android
│   ├── Models.kt              ContentBaseline (3.7.0-R2 + 2 SHA-256)
│   ├── ContentManifest.kt     ContentManifest, ContentArtifact, ContentChannel, TranslationVariant
│   ├── Compatibility.kt       ClientCompatibility, CompatibilityRules, InstallationIntegrityRules
│   ├── Digests.kt             DigestAlgorithm, DigestUtils, DigestValue
│   ├── InstallMarker.kt       marker kepemilikan file yang diinstall
│   ├── ManagedFiles.kt        ManagedFileRecord, ManagedFileOwnership, ManagedFileRole
│   └── SemanticVersion, GameClientInfo, CompatibilityResult/Issue, IntegrityResult/Issue
│
├── storage/                   akses ke direktori data game
│   ├── GameStorage            interface: read/write/copy/move/delete/list/stat/exists/digest/findFirst
│   ├── NormalGameStorage      implementasi proses biasa (root storage read-only → gagal di API 30+)
│   ├── ShizukuStorageAdapter  implementasi lewat Shizuku newProcess + shell
│   ├── ShizukuGameStorage     adapter
│   ├── StoragePathGuard       tolak path di luar root
│   └── ResourceVersionDiscovery  cari folder resource version yang sudah termaterialisasi
│
├── installer/                 mesin install 3 paket (translation / font / edits)
│   ├── TransactionalPatchEngine  state machine + rollback
│   ├── InstallTransaction.kt  InstallTransactionLayout/Codec, TransactionMarker, TransactionOperation
│   ├── AndroidPatchMount      descriptor+paths untuk paket lokalisasi
│   ├── AndroidFontMount       descriptor+paths untuk font
│   ├── AndroidEditsMount      descriptor+paths untuk edits (UI reskin)
│   ├── AndroidPatchInstaller / AndroidFontInstaller / AndroidEditsInstaller
│   ├── NativeV12PakBuilder    penulis .pak V12 untuk font hasil import
│   ├── FontFileCodec          baca TTF/OTF/TTC, cek glyph Vietnam
│   └── AndroidFontPack, FileSnapshot, InstallPhase/Progress, *Status, *Paths
│
└── updater/                   konten jarak jauh (belum dipakai di build ini)
    ├── SignedManifestLoader   parse + validasi JSON
    ├── ManifestSignature.kt   DetachedManifestSignature + Ed25519ManifestVerifier
    ├── ContentStager          unduh artefak (ContentDownloader.kt)
    ├── HttpUrlConnectionTransport
    └── UpdatePolicy, RetrySleeper, CancellationToken, StageResult, TranslationContentVersions
```

## File sumber asli yang berhasil dipulihkan

| File Kotlin | Class utama |
|---|---|
| `Models.kt` | `core.ContentBaseline` |
| `ContentManifest.kt` | `core.ContentManifest`, `core.ClientCompatibility`, `updater.AppVersion` |
| `Compatibility.kt` | `core.CompatibilityRules`, `core.InstallationIntegrityRules` |
| `Digests.kt` | `core.DigestUtils`, `core.DigestValue` |
| `InstallMarker.kt` | `core.InstallMarker` |
| `ManagedFiles.kt` | `core.ManagedFileOwnership` |
| `ResourceVersionDiscovery.kt` | `storage.ResourceVersionDiscoverer` |
| `NormalGameStorage.kt` | `storage.NormalGameStorage` |
| `ShizukuStorageAdapter.kt` | `storage.ShizukuStorageAdapter`, `storage.AndroidShizukuCommandRunner`, `storage.AndroidShizukuBackendState` |
| `StoragePathGuard.kt` | `storage.StoragePathGuard` |
| `TransactionalPatchEngine.kt` | `installer.TransactionalPatchEngine` |
| `InstallTransaction.kt` | `installer.InstallTransactionCodec`, `installer.InstallTransactionLayout` |
| `AndroidPatchMount.kt` | `installer.AndroidPatchMount` |
| `AndroidFontMount.kt` | `installer.AndroidFontMount` |
| `AndroidEditsMount.kt` | `installer.AndroidEditsMount`, `installer.EditsInstallIdentity` |
| `AndroidPatchInstaller.kt` | `installer.AndroidPatchInstaller` |
| `AndroidFontInstaller.kt` | `installer.AndroidFontInstaller` |
| `AndroidEditsInstaller.kt` | `installer.AndroidEditsInstaller` |
| `AndroidFontPack.kt` | `installer.FontInstallIdentity` |
| `FontFileCodec.kt` | `installer.FontFileCodec` |
| `NativeV12PakBuilder.kt` | `installer.NativeV12PakBuilder` |
| `MainActivity.kt` | `MainActivity` |
| `WuWaMobileController.kt` | `WuWaMobileController` |
| `FullFeatureMobileScreen.kt` | `FullFeatureMobileScreenKt` |
| `Composables.kt` | `FullFeatureMobileScreenKt`, `FontGalleryPanelKt`, `PremiumInstallerUiKt` |
| `PremiumInstallerUi.kt` | `PremiumInstallerUiKt` |
| `FontGalleryPanel.kt` | `FontGalleryPanelKt` |
| `FontCatalogRepository.kt` | `FontCatalogRepository` |
| `DiagnosticsExporter.kt` | `DiagnosticsExporter` |
| `DebugInstallReceiver.kt` | `DebugInstallReceiver` |
| `ManifestSignature.kt` | `updater.DetachedManifestSignature`, `updater.Ed25519ManifestVerifier` |
| `ContentDownloader.kt` | `updater.ContentStager` |

## Navigasi

Tiga section, enum `MobileSection` (label Vietnam, sublabel Vietnam):

| Enum | Label | Sublabel |
|---|---|---|
| `HOME` | Trang chủ | Cài nhanh · setup · mở game |
| `CUSTOMIZE` | Tùy biến | Font và hiển thị |
| `TOOLS` | Bảo trì | Kiểm tra · repair · chẩn đoán |

## State dashboard

`MobileDashboardState` dipakai Compose; `installStatus` dan `fontStatus` memuat
`installed`, `healthy`, `recoveryPending`, dan `message` (status font juga memuat
nama font).

| Field | Tipe / isi |
|---|---|
| `game` | `GameClientInfo?`: installed, packageName, versionName, versionCode |
| `shizukuAvailable`, `shizukuGranted`, `busy` | `Boolean` |
| `installStatus` | `AndroidInstallStatus` |
| `fontStatus` | `AndroidFontStatus` |
| `selectedVariant` | `TranslationVariant` |
| `progress` | `InstallProgress` |
| `error` | `String?` |

`MobileDashboardLogic.present(state, variant)` menurunkan:

| `MobileUiPresentation` | Isi |
|---|---|
| `readiness` | `MobileReadiness` |
| `compatibilityLabel`, `compatibilityDetail`, `setupHint` | String UI |
| `primaryAction` | `MobilePrimaryAction` |
| `primaryLabel` | String UI |
| `primaryEnabled`, `showProgress` | Boolean |


Enum turunan yang harus diport:

```kotlin
enum class TranslationVariant(val label: String, val payloadFileName: String) {
    ENGLISH_NAMES("Tên Anh",     "WuWaVH_EN_99_P.pak"),
    HAN_VIET    ("Hán Việt",    "WuWaVH_HanViet_99_P.pak"),
}

enum class MobileReadiness { INITIALIZING, READY, GAME_MISSING, GAME_VERSION_UNSUPPORTED,
                             RESOURCE_NOT_READY, RESOURCE_VERSION_UNSUPPORTED,
                             SHIZUKU_NOT_RUNNING, SHIZUKU_PERMISSION_REQUIRED }

enum class MobilePrimaryAction { NONE, INSTALL_AND_OPEN, OPEN_GAME, OPEN_GAME_STORE,
                                 OPEN_SHIZUKU, REQUEST_SHIZUKU_PERMISSION }

enum class InstallPhase { IDLE, PREFLIGHT, BACKUP, STAGE, INSTALL, VERIFY_SOURCE,
                          VERIFY_TARGET, COMMIT, ROLLBACK, VERIFY_ROLLBACK }

enum class ManagedFileRole { TRANSLATION_PAK, CLONED_SIGNATURE, MOUNT_DESCRIPTOR }
```

## Alur utama

```
MainActivity
  └─ FullFeatureMobileScreen(controller = WuWaMobileController(context))
       ├─ refresh(gameInfo, variant)      → poll: PackageManager + ResourceVersionDiscoverer
       │                                   + Shizuku binder + installer.inspect()
       ├─ install(variant)                → AndroidPatchInstaller.install()
       ├─ uninstall()                     → ownership check → hapus 3 file terkelola
       ├─ repair()                        → pulihkan transaksi terputus / reinstall
       ├─ installFont / recoverFont / uninstallFont / importFont / deleteImportedFont
       └─ exportDiagnostics()             → DiagnosticsExporter → JSON → FileProvider share
```

State dispersal: `SharedPreferences` bernama `wahu_vhwuwa_preferences` dengan key
`preferred_translation_variant`, `preferred_font_pack`, `preferred_font_id`
(default `be_vietnam_pro`). `AppEventLog` menulis ke file `recent-app.log`; pesan controller memakai tag `WAHU-VHWUWA`.

## Aturan kompatibilitas (`core.CompatibilityRules.evaluate`)

Masukan: `GameClientInfo`, resourceVersion, `ContentManifest`, `TranslationVariant`.
Keluar: `CompatibilityResult(issue, message)`; issue = `NONE` bila kompatibel.

Daftar validasi dari konstanta `CompatibilityRules`; urutan aktual terverifikasi
per-instruksi di `08-discovery-compat-design.md` §2:
1. package harus `com.kurogame.wutheringwaves.global` → `PACKAGE_MISMATCH`
2. `versionName` harus `^\d+\.\d+\.\d+$` → `GAME_VERSION_MISSING` / `GAME_VERSION_MALFORMED`
3. `versionCode` harus ada → `VERSION_CODE_MISSING`
4. harus ada entry `compatibility` di manifest → `RESOURCE_VERSION_MISSING` / `RESOURCE_VERSION_MALFORMED`
5. resourceVersion harus cocok entry → `RESOURCE_VERSION_UNSUPPORTED`
6. artifact untuk variant harus ada di manifest → `VARIANT_ARTIFACT_MISSING`
7. `versionCode` harus cocok → `VERSION_CODE_MISMATCH`

Aturan yang di-hardcode di `core.ContentBaseline`: game version `3.7.0`,
content version `3.7.0-R2`. Pesan UI yang dihasilkan:

- `"Bản hiện tại cần resource 3.7.0."`
- `"Bản nội dung hiện tại được khóa cho game 3.7.0."`

## Manifest content

`core.ContentManifest` — schema `VHWUWA_CONTENT_MANIFEST_V1`:

Berikut **skema tipe**, bukan JSON valid — `|` menyatakan union dan `…` adalah placeholder.
Nilai yang terverifikasi dari `core.ContentBaseline` sudah ditulis apa adanya: `contentVersion`
`3.7.0-R2`, game version `3.7.0`, dan kedua digest payload. `gameVersionCode` tidak ada di
baseline, jadi nilainya tidak diketahui dari APK ini.

```jsonc
{
  "schema": "VHWUWA_CONTENT_MANIFEST_V1",
  "contentVersion": "3.7.0-R2",
  "channel": "STABLE" | "BETA",
  "compatibility": [ { "gameVersion": "3.7.0", "resourceVersion": "3.7.0",
                       "gameVersionCode": … } ],
  "artifacts":  [ { "variant": "ENGLISH_NAMES" | "HAN_VIET",
                    "fileName": "WuWaVH_EN_99_P.pak" | "WuWaVH_HanViet_99_P.pak",
                    "sha256": "8114f9e7…" | "ce8ec3b1…" } ]
}
```

Validasi: tepat satu artifact per variant, tanpa duplikat, `compatibility` tidak boleh kosong,
schema harus dikenali.

## Manifest updater (JSON, belum terpasang di build ini)

`updater.UpdaterManifestJson` — `requireExactKeys` ketat (kunci asing → error):

Sama seperti di atas, ini **skema tipe** — bukan JSON valid, dan **tidak ada instance nyata
di APK ini**. Yang terverifikasi hanya nama kuncinya (dari `UpdaterManifestJson` +
`requireExactKeys`) dan string schema `VHWUWA_ANDROID_UPDATER_MANIFEST_V1`. Semua nilai
lain harus kamu tentukan sendiri.

```jsonc
{
  "schema": "VHWUWA_ANDROID_UPDATER_MANIFEST_V1",
  "appVersion": …,
  "minAppVersion": …,
  "channel": …,
  "translationContentVersion": …,
  "compatibility": [ { "gameVersion": …, "resourceVersion": …,
                       "packageName": …, "gameVersionCode": … } ],
  "artifacts": [ { "variant": …, "fileName": …, "url": …,
                   "sha256": …, "sizeBytes": … } ],
  "bytes": <base64 manifest>,
  "manifest": <base64 signed detached manifest>
}
```

Signature scheme `DetachedManifestSignature` diverifikasi `Ed25519ManifestVerifier`;
kegagalan → `ManifestSignatureException`, JSON rusak → `ManifestFormatException`.
Tidak ada URL yang di-hardcode, jadi channel update harus kamu isi sendiri.