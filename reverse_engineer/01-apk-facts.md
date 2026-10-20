# 01 — Fakta APK

Sumber: `VHWuWa-Android-0.2.0-alpha.9-3.7.0-R2.apk` (158.736.615 B).

## Identitas dan stack

| Field | Nilai |
|---|---|
| Package / label | `vn.wahu.vhwuwa` / `WAHU Việt Hóa WuWa` |
| versionName / versionCode | `0.2.0-alpha.9` / `9` |
| minSdk / targetSdk / compileSdk | 29 / 36 / 37 (Android 17) |
| BuildConfig | Tidak ada label konfigurasi lain |

Native Android dengan Kotlin + Jetpack Compose Material 3; bukan Tauri, Flutter,
React Native, atau WebView. Tidak ada aset web atau `.so` milik aplikasi.

## Manifest

### Permissions

- `android.permission.INTERNET`
- `moe.shizuku.manager.permission.API_V23` — Shizuku root bridge
- `vn.wahu.vhwuwa.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION`
  (protection level `signature`, AndroidX Core)

Tidak ada permission storage/location/camera; akses storage game dilakukan
lewat Shizuku.

### Components dan aplikasi

| Tipe | Nama | Catatan |
|---|---|---|
| Activity | `vn.wahu.vhwuwa.MainActivity` | `LAUNCHER`, portrait, exported |
| Receiver | `vn.wahu.vhwuwa.DebugInstallReceiver` | exported; 8 debug actions |
| Provider | `androidx.core.content.FileProvider` | authority `vn.wahu.vhwuwa.files`, `@xml/diagnostic_paths` |
| Provider | `rikka.shizuku.ShizukuProvider` | authority `vn.wahu.vhwuwa.shizuku` |
| Provider | `androidx.startup.InitializationProvider` | EmojiCompat, ProcessLifecycle, ProfileInstaller |

Tidak ada service atau worker. `<queries>` mencantumkan
`com.kurogame.wutheringwaves.global` dan `moe.shizuku.privileged.api`.
Application flags: `debuggable=true` (debug-signed), `allowBackup=false`,
`extractNativeLibs=false`, theme `Theme.VHWuWa`.

## Isi paket dan distribusi class

```text
AndroidManifest.xml
classes.dex … classes10.dex          10 dex, 27.552 class total
lib/{arm64-v8a,armeabi-v7a,x86,x86_64}/libandroidx.graphics.path.so
assets/font/catalog.json
assets/font/font-preview/*.ttf       8 fonts
assets/font/licenses/*.txt          8 licenses
assets/font/BeaufortBold_100_P.pak
assets/font/WahuFont_100_P.pak
assets/payload/WuWaVH_EN_99_P.pak
assets/payload/WuWaVH_HanViet_99_P.pak
res/                                 values + ~100 values-XX; AndroidX strings only
resources.arsc
```

`vn.wahu.*` berjumlah 486 class pada 486 file top-level/inner.

| dex | Total class | `vn.wahu.*` | Isi utama |
|---|---:|---:|---|
| `classes.dex` | 17.865 | 0 | Compose/AndroidX/Kotlin stdlib |
| `classes2.dex` | 34 | 34 | `updater.*` |
| `classes3.dex` | 166 | 166 | `installer.*` |
| `classes4.dex` | 37 | 37 | `core.*` |
| `classes5.dex` | 69 | 69 | `storage.*` |
| `classes6.dex` | 105 | 10 | Shizuku (`rikka.*`) |
| `classes7.dex` | 170 | 170 | UI + controller (`vn.wahu.vhwuwa.*`) |
| `classes8-10.dex` | 9.106 | 0 | Dependencies lain |

## Integrasi dan batas paket

- Satu-satunya URL APK adalah
  `https://play.google.com/store/apps/details?id=` (tombol “Tải Wuthering
  Waves”). `updater.*` memiliki verifikasi Ed25519, tetapi tidak dikonfigurasi
  dengan URL; konten build ini berasal dari `assets/`.
- Package `vn.wahu.*` hampir tidak di-obfuscate: class, field, dan nama file
  Kotlin asli tersedia lewat `SMAP`. Pengecualian: `storage.AndroidShizukuCommandRunner`
  memanggil `Shizuku.newProcess` reflektif (`const-string "newProcess"` lalu
  `Class.getDeclaredMethod(...)`), jadi shell root tidak tampak sebagai call
  simbol biasa.

## Font catalog

`assets/font/catalog.json` berisi tepat 8 font, masing-masing sejalan dengan satu
`.ttf` preview dan satu lisensi OFL:

| id | label | category |
|---|---|---|
| `be_vietnam_pro` | Be Vietnam Pro | Hiện đại |
| `inter` | Inter | Gọn dễ đọc |
| `roboto` | Roboto | Cân bằng |
| `montserrat` | Montserrat | Mạnh rõ |
| `open_sans` | Open Sans | Dễ đọc |
| `nunito` | Nunito | Bo tròn |
| `noto_sans` | Noto Sans | Unicode rộng |
| `lora` | Lora | Serif |

Semua mencatat `viHave: 134`, `viTotal: 134` dan sample
`Tiếng Việt: Ă Â Đ Ê Ô Ơ Ư – Cộng Hưởng 1234567890`. `FontFileCodec.vietnameseCharset()`
menghasilkan 134 codepoint dari 67 huruf kecil `vietnameseLower` ditambah huruf
kapital melalui `toUpperCase(Locale.ROOT)`. Font yang dipasang ke game:
`BeaufortBold_100_P.pak` (default) dan `WahuFont_100_P.pak` (WahuFont).

## Baseline content

`core.ContentBaseline` menetapkan content version `3.7.0-R2`; kedua SHA-256
berikut cocok dengan payload dalam APK:

```text
8114f9e79aba08ff3cc3ef1a6bdc1499782725b0547627c06856eb14e31f4725  WuWaVH_EN_99_P.pak       (enSha256)
ce8ec3b1cc7dbbb6ccc7a4fe330fd20e598f249729dc3d2e554ab8f9d49ef061  WuWaVH_HanViet_99_P.pak  (hanVietSha256)
```

Port yang mengganti payload harus mengganti kedua hash di baseline.