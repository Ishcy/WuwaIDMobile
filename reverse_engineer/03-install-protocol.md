# 03 — Protokol Install (WAHU)

Menulis payload ke data game Wuthering Waves Global (`com.kurogame.wutheringwaves.global`) melalui Shizuku, lalu mendaftarkannya dengan *mount descriptor* engine.

## 1. Root dan layout

Dihasilkan `AndroidPatchMount.resourcesRoot(packageName)`:

```
/storage/emulated/0/Android/data/{packageName}/files/UE4Game/Client/Client/Saved/Resources
```

Untuk game: `/storage/emulated/0/Android/data/com.kurogame.wutheringwaves.global/files/UE4Game/Client/Client/Saved/Resources`

Isi resource yang diharapkan (`ResourceVersionDiscoverer`, regex `^\d+\.\d+\.\d+$`):

```
Resources/
└── 3.7.0/                     ← resourceVersion (dari versi game, bukan nama folder sembarang)
    ├── ResManifest
    ├── Launcher/
    ├── Lang_en/
    └── Mount/
        ├── MountResource.txt  ← milik game
        └── wahuviethoa.txt    ← milik WAHU
```

`ResourceVersionDiscoverer` hanya mempertimbangkan folder semver yang sudah termaterialisasi; urutan seleksi ada di [08 §1](08-discovery-compat-design.md#1-resource-version-discovery). Jika tak ada kandidat: `"No materialized resource-version directory was found"`. String fallback ter-hardcode di kode: `/sdcard/Android/data/com.kurogame.wutheringwaves.global/files/UE4Game/Client/Client/Saved/Resources/3.7.0/Lang_en`.

## 2. Tiga paket yang diinstall

| Paket | Folder di resource root | PAK | Descriptor | Mount file |
|---|---|---|---|---|
| **Translation** | `wahuviethoa/` | `WuWaVH_99_P.pak` | `Mount/wahuviethoa.txt` | priority **99** |
| **Font** | `wahufont/` | `WahuFont_100_P.pak` | `Mount/wahu-font.txt` | priority ≥ 0 |
| **Edits** | `wahuedits/` | `WahuEdits_101_P.pak` | `Mount/wahu-edits.txt` | priority ≥ 0 |

Constante `AndroidPatchMount`: `MOUNT_FILENAME="wahuviethoa.txt"`,
`PATCH_FILENAME="WuWaVH_99_P.pak"`, `PATCH_FOLDER="wahuviethoa"`, `PRIORITY=99`.

## 3. Format mount descriptor

Dihasilkan `AndroidPatchMount.descriptor(pakSha1, signatureSha1, priority)`:

```
::Mount::
wahuviethoa/WuWaVH_99_P,99,<PAK_SHA1_UPPER>,<SIG_SHA1_UPPER>,,
::Del::
```

Kontrak terverifikasi dari bytecode `AndroidPatchMount.descriptor`:

- Baris mount punya 6 field koma: `[0]=folder/PAK_BASE` tanpa `.pak`, `[1]=priority`, `[2]=pakSha1`, `[3]=signatureSha1`, `[4]=""`, `[5]=""`; dua field akhir ditulis `,,\n`.
- `AndroidPatchMount.smali:220-241` memanggil `removeSuffix(".pak")`: nama fisik tetap `WuWaVH_99_P.pak` dan `WuWaVH_99_P.sig`, sedangkan field mount memakai base tanpa ekstensi. [Implementasi WuwaIDLauncher](https://github.com/TitoTFP/WuwaIDLauncher/blob/676c4a9b3b3f14ccb570d90252dc8972dbac8d13/src-tauri/src/engine/installer.rs#L95-L111) memakai format sama.
- Parser WAHU sendiri (`detectConflictingMounts`) memecah baris dengan `split(',')` lalu
  membaca **index 1** sebagai priority (`toIntOrNull`), dipakai untuk mendeteksi mount
  pihak lain dengan priority ≥ 99.
- `pakSha1` dan `signatureSha1` harus cocok regex `^[0-9a-fA-F]{40}$` (SHA-1, **bukan** SHA-256).
- Kedua hash ditulis `toUpperCase(Locale.ROOT)`.
- Priority harus `0 <= p < 1000`; di luar itu → `IllegalArgumentException("Invalid mount priority: N")`.
- Baris `::Mount::` dan `::Del::` selalu diakhiri newline (`\n`); pembacaan di sisi
  installer juga menoleransi `\r\n`.
- Installer menolak mount milik pihak lain: `"Phát hiện custom mount ưu tiên >= 99: …"` →
  `"Phát hiện mod/mount khác có thể xung đột. WAHU không ghi đè tự động."`

Font dan Edits memakai format identik dengan folder/filename masing-masing, plus file
pendamping: `wahufont/WahuFont_100_P.sig` (salinan `.sig` resmi game) dan marker
`/.font-owner`, `/.font-transaction` (edits: `/.edits-owner`, `/.edits-transaction`).

## 4. Hash: dua tingkat

- **Mount descriptor** memakai SHA-1 → harus hitung SHA-1 file `.pak` dan file `.sig` yang
  ditulis, bukan SHA-256 dari `ContentBaseline`.
- **Integritas aplikasi** memakai SHA-256 → `core.ContentBaseline.enSha256` /
  `hanVietSha256` dibandingkan dengan isi asset sebelum commit
  (`VARIANT_DIGEST_MISMATCH`).

`WuWaMobileController.sha256(File)` untuk cek cepat payload Android sebelum commit,
dan pesan gagalnya: `"<file> trong APK không đúng SHA-256"`.

## 5. Backend storage

`GameStorage` interface (dipakai installer):

```
readBytes / writeBytes / readText / copy / move / delete / list / exists / stat / digest / findFirst / mkdirs
```

Dua implementasi:

- `NormalGameStorage` — proses app biasa. Melarang diri sendiri dengan pesan eksplisit:
  `"Normal app process cannot read the storage root"`,
  `"Normal app process has read-only access to the storage root"`,
  `"Normal file backend cannot force-stop applications"`.
  Praktisnya **tidak berguna di Android 11+**; minSdk app 29.
- `ShizukuStorageAdapter` + `AndroidShizukuCommandRunner` — satu-satunya yang bekerja.
  Backend name yang diekspor di diagnostik: `shizuku-safe-storage`.

`StoragePathGuard` menolak path di luar root → `StorageErrorCode.PATH_OUTSIDE_ROOT`.
Kode error lain: `BACKEND_UNAVAILABLE`, `INVALID_TARGET`, `IO_ERROR`, `NOT_FOUND`,
`NOT_READABLE`, `NOT_WRITABLE`, `PERMISSION_REQUIRED`.

## 6. State machine transaksi

`installer.InstallPhase`:

```
IDLE → PREFLIGHT → BACKUP → STAGE → INSTALL
     → VERIFY_SOURCE → VERIFY_TARGET → COMMIT
     (gagal) → ROLLBACK → VERIFY_ROLLBACK
```

`TransactionalPatchEngine` + `InstallTransactionLayout`/`InstallTransactionCodec` +
`TransactionMarker`/`TransactionOperation`. Detail lengkap di
[`07-transaction-engine.md`](07-transaction-engine.md).

Marker transaksi `.wahu-transaction.json` memakai `schema: Int`; `TransactionalPatchEngine.ownership()` memeriksa isi file, bukan marker (lihat §7).

- `"Có transaction WAHU gián đoạn cần recovery"` → status `Cần phục hồi`; UI menyediakan
  `Repair bản cài WAHU` sebelum install ulang.
- Biner harus binary-safe: `"Storage backend không hỗ trợ transaction binary-safe"`.

`core.InstallMarker` tidak perlu diport: tidak ada referensi di luar `core/`. Model terpisah ini memakai schema String `VHWUWA_INSTALL_MARKER_V1`, mensyaratkan tepat tiga file kanonik tanpa path/role duplikat dan `gamePackageName = com.kurogame.wutheringwaves.global`; nilai `gameVersionCode` tidak diketahui karena tidak ada di `ContentBaseline`.

## 7. Ownership

### 7.1 `ManagedFileOwnership` — registry path kanonik (live)

`DiagnosticsExporter` adalah satu-satunya pemanggil di luar `core/` (`pathFor` saja); registry ini murni resolver, tanpa I/O atau pembacaan marker:

| Kontrak | Nilai |
|---|---|
| `OWNER_ID` | `vn.wahu.vhwuwa` |
| `PATCH_DIRECTORY` | `wahuviethoa` |
| `TRANSLATION_PAK` | `wahuviethoa/WuWaVH_99_P.pak` |
| `CLONED_SIGNATURE` | `wahuviethoa/WuWaVH_99_P.sig` |
| `MOUNT_DESCRIPTOR` | `Mount/wahuviethoa.txt` |

`pathFor(role)`, `roleFor(path)`, `isOwnedCommittedFile(path)`, `isOwnedDirectory(path)`,
`isOwnedLifecycleFile(path)`, `getCommittedRelativePaths()`; `.new`/`.bak` lifecycle files
dan direktori `wahuviethoa` juga dikenali sebagai milik WAHU.

### 7.2 Gate di jalur install — dari isi file

`TransactionalPatchEngine.ownership()` tidak membaca marker: `isWahuDescriptor()` memeriksa isi descriptor (`.pak`, `WuWaVH_99_P.pak`, `wahuviethoa/`, dipisah `::`) dan PAK yang ada harus cocok SHA-256 `core.ContentBaseline`.

Penolakan:

- `"Có file cùng tên WAHU nhưng thiếu PAK baseline; từ chối ghi/xóa"`
- `"WuWaVH_99_P.pak hiện tại không khớp baseline WAHU; từ chối ghi đè/xóa"`
- `"Mount/wahuviethoa.txt không được xác nhận là WAHU-owned; từ chối ghi đè/xóa"`
- `repair()` tanpa patch: `"Chưa có patch WAHU để sửa"`.
- Konflik mount pihak lain: `"Phát hiện mod/mount khác có thể xung đột. WAHU không ghi đè tự động."`

UI menyatakan tidak ada instalasi buta: `"Gói cài không vượt qua kiểm tra toàn vẹn. Không có file nào được cài mù."`

## 8. Font: alur import & glyph check

`FontFileCodec` + `NativeV12PakBuilder`:

1. User pilih TTF/OTF/TTC (`importFont(bytes, name)`).
2. Batas ukuran: `"Font quá lớn (>30MB)"`.
3. Cek glyph terhadap **134 codepoint** yang dibentuk `FontFileCodec.vietnameseCharset()`:
   67 huruf kecil Vietnam di-hardcode sebagai `vietnameseLower`
   (`àáảãạăằắẳẵặâầấẩẫậèéẻẽẹêềếểễệìíỉĩịòóỏõọôồốổỗộơờớởỡợùúủũụưừứửữựỳýỷỹỵđ`),
   lalu tiap huruf ditambahkan versi kapitalnya lewat `toUpperCase(Locale.ROOT)`
   → 67 + 67 = 134, sesuai `viTotal` di `catalog.json`.
   Kalau kurang → `"Font thiếu glyph tiếng Việt (…). Có thể xem trước nhưng WAHU sẽ không cho cài."`
4. Kalau cukup → tulis `.pak` versi **V12** via `NativeV12PakBuilder`, lalu pasang
   sebagai `wahufont/WahuFont_100_P.pak` + descriptor `Mount/wahu-font.txt`.
5. Uninstall font → `"Đã khôi phục font mặc định của game."`

Preview memakai `assets/font-preview/*.ttf` (8 file) via `fontPreviewBytes()`;
`importLauncher` menampilkan jumlah glyph: `"Đã import … glyph tiếng Việt."`.

Catalog font (`FontCatalogRepository`) dibaca/tulis `assets/font/catalog.json` di app storage,
dengan item:

```json
{ "id": "...", "label": "...", "category": "...",
  "pakSha256": "...", "source": "font-preview/x.ttf", "sourceSha256": "...",
  "family": "...", "viHave": 134, "viTotal": 134,
  "license": "font/licenses/x-OFL.txt", "sample": "...", "builtin": true }
```

## 9. Diagnostik
Key `DiagnosticsExporter` diverifikasi dari string konstan; `appVersion`/`versionCode` (`0.2.0-alpha.9`/`9`) diverifikasi dengan `aapt2 dump badging`. Tidak ada JSON diagnostik yang dicapture dari perangkat.

`schemaVersion`, `generatedAt`, `appVersion`, `versionCode`, `gameInstalled`, `gameVersion`,
`gameVersionCode`, `resourceVersion`, `selectedVariant`, `installedVariant`,
`translationContentVersion`, `storageBackend`, `storageAvailable`, `permissionGranted`,
`installed`, `healthy`, `recoveryPending`, `statusMessage`, `fontInstalled`, `fontPack`,
`fontHealthy`, `fontRecoveryPending`, `fontStatusMessage`, `technical-proof`, `unsupported`,
`compatibilityLevel`, `deviceApiLevel`, `managedFiles`.

Dikirim lewat `FileProvider` authority `vn.wahu.vhwuwa.files` dengan `@xml/diagnostic_paths`.

## 10. Receiver debug (tidak dikecualikan di produksi, tapi berguna saat rebuild)

`vn.wahu.vhwuwa.DebugInstallReceiver`, exported, menerima:

| Action | Arti |
|---|---|
| `vn.wahu.vhwuwa.DEBUG_INSPECT` | periksa status install |
| `vn.wahu.vhwuwa.DEBUG_INSTALL_EN` | install varian ENGLISH_NAMES |
| `vn.wahu.vhwuwa.DEBUG_INSTALL_HAN` | install varian HAN_VIET |
| `vn.wahu.vhwuwa.DEBUG_UNINSTALL` | hapus patch |
| `vn.wahu.vhwuwa.DEBUG_FONT_INSPECT` | periksa status font |
| `vn.wahu.vhwuwa.DEBUG_FONT_INSTALL_BEVN` | pasang `be_vietnam_pro` |
| `vn.wahu.vhwuwa.DEBUG_FONT_IMPORT_TEST` | uji import font |
| `vn.wahu.vhwuwa.DEBUG_FONT_UNINSTALL` | hapus font |

## 11. Perubahan yang diperlukan untuk varian baru

- Bridge Shizuku native memeriksa Binder, meminta `API_V23`, lalu menjalankan proses lewat
  `Shizuku.newProcess`; refleksi di APK bukan syarat port.
- Payload `.pak` Indonesia harus dibangun tersendiri (container dan isi SQLite/JS game).
  `core.ContentBaseline` saat ini meng-hardcode SHA-256 EN/HanViet; `AndroidPatchInstaller`
  menolak mismatch dengan `"PAK cùng tên không khớp baseline WAHU"`.
- Tambahkan nilai Indonesia pada `core.TranslationVariant` dengan
  `payloadFileName = WuWaVH_ID_99_P.pak`; sambungkan ke `ContentManifest.artifactFor`,
  `UpdatePolicy`, dan field variant marker transaksi.