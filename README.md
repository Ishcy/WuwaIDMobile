# WuwaID Mobile

Pendamping lokalisasi **Wuthering Waves ke Bahasa Indonesia**, dibuat dengan Tauri 2 untuk Android dan desktop. Frontend memakai TypeScript/Vite; backend native memakai Rust dan Kotlin.

## Fitur

- **Beranda:** status game, metode akses, status terjemahan, dan pemasangan atau perbaikan terjemahan.
- **Kustomisasi:** terapkan teks UID kustom atau pulihkan teks bawaan. Ini mengubah teks tampilan game, bukan UID akun.
- **Perawatan:** pilih Shizuku atau root, periksa ulang, pulihkan transaksi, kelola pembaruan aplikasi, dan simpan/bagikan diagnostik.
- Navigasi horizontal di desktop dan dock bawah di Android. Tema **Terang / Gelap / Sistem** disimpan lokal; Sistem mengikuti perubahan tema perangkat. Pergantian tema tidak menghapus isian atau menghentikan operasi.
- Progres diperbarui tanpa membangun ulang navigasi. Animasi 140–180 ms mengikuti `prefers-reduced-motion`.

## Android

### Memasang terjemahan

1. Pasang **Wuthering Waves Global** (`com.kurogame.wutheringwaves.global`) atau **Galaxy Store** (`com.kurogame.wutheringwaves.samsung`), lalu selesaikan unduhan resource game termasuk paket bahasa Inggris.
2. Di **Perawatan**, pilih akses:
   - **Shizuku** (default): [pasang dan jalankan Shizuku](https://shizuku.rikka.app/), lalu pilih **Berikan izin Shizuku**. Server Shizuku API 12 atau lebih baru diperlukan.
   - **Root**: pilih **Gunakan Root**, lalu **Berikan izin root** dan setujui permintaan pada pengelola root. Membuka aplikasi/refresh tidak menjalankan `su`. Pilihan metode disimpan, tetapi grant tidak dianggap permanen; minta izin lagi setelah aplikasi/service dimulai ulang (pengelola root dapat mengingat keputusan). Tidak ada fallback otomatis, dan pergantian metode diblokir selama operasi.
3. Periksa perangkat, lalu pilih **Pasang terjemahan**. Game dihentikan sebelum perubahan berkas.

Rilis PAK diambil dinamis dari [rilis stabil terbaru WuwaID](https://github.com/TitoTFP/WuwaID/releases/latest), bukan dari rilis aplikasi; metadata tag, aset, ukuran, dan hash tidak dibundel. Unduhan HTTPS diverifikasi dengan ukuran dan SHA-256; cache pakai ulang juga diverifikasi. Rilis dipasang hanya jika lini game/resource cocok, memakai pasangan PAK/signature resmi pada resource materialized (prefer `Lang_en`). Pemasangan standar menyalin PAK utuh; UID kustom mengubah salinan privat lalu mengemas ulang PAK. Reset memulihkan PAK asli.

Pemasangan Android berada di `Resources/<versi-resource>/`: `wuwaid/WuwaID_99_P.pak`, `wuwaid/WuwaID_99_P.sig` (signature resmi), `wuwaid/.owner.json` (identitas/hash kepemilikan), dan `Mount/wuwaid.txt` (prioritas 99). Descriptor memakai nama dasar tanpa ekstensi: `wuwaid/WuwaID_99_P`; entri mount berbentuk `wuwaid/WuwaID_99_P,99,<PAK_SHA1_UPPER>,<SIG_SHA1_UPPER>,,`. Layout Android ini berbeda dari layout `Patch/` launcher PC.

Berkas resmi game tidak ditimpa. Mutasi dibatasi pada path WuwaID di versi resource numerik; traversal, symlink, kepemilikan tidak dikenal, dan mount lain berprioritas 99 atau lebih tinggi ditolak. Berkas diverifikasi sebelum commit; jurnal/backup mendukung rollback dan pemulihan transaksi tertunda tanpa jaringan. Residu `.new`/`.bak` tanpa jurnal valid tidak dihapus otomatis.

Aplikasi mendukung Global dan Galaxy Store. Jika keduanya terpasang, pemilihan mengikuti resource Global yang sudah siap, lalu Galaxy Store, lalu paket terpasang pertama; pilihan dibekukan selama sesi. Migrasi versi sebelum Tauri bersifat offline: SHA-256 dan ukuran PAK harus cocok dengan katalog rilis lama, signature tidak boleh kosong, dan descriptor harus cocok persis dengan PAK/signature terpasang dalam format lama. Berkas ambigu/tidak dikenal memblokir mutasi dan berkas asing tidak dihapus. Langkah migrasi, pemulihan, instalasi, dan penghapusan ada di [panduan Android](docs/android.md).

### UID kustom

Masukkan teks maksimal **32 unit UTF-16**. Spasi tepi dipangkas; input kosong, karakter kontrol, dan pemisah baris ditolak. Android menampilkan UID asli game sebagai nilai bawaan; nomor contoh browser hanyalah simulasi. Reset memasang kembali PAK rilis asli. Rincian format PAK dan prosedur Android ada di [panduan Android](docs/android.md).

### Pembaruan aplikasi

Updater memeriksa [rilis stabil WuwaID Mobile](https://github.com/TitoTFP/WuwaIDMobile/releases) saat aplikasi dibuka dan melalui **Perawatan → Pembaruan aplikasi → Periksa pembaruan**. Jika tersedia, unduh APK, periksa hasil verifikasi, lalu konfirmasikan pemasangan pada dialog Android. Aplikasi tidak memasang update diam-diam, tidak menghapus data atau PAK, dan tidak memerlukan root/Shizuku. Pembatalan atau unduhan gagal bukan update berhasil; update tidak berjalan bersamaan dengan mutasi PAK. Browser, desktop, dan build debug tidak menyediakan updater rilis.

APK diperiksa terhadap nama aset, ukuran dan SHA-256, package `com.titotfp.wuwaid`, versi/tag, versionCode yang lebih tinggi, signer rilis, minSdk, dan ABI ARM64 sebelum installer Android dibuka. Verifikasi signature akhir tetap dilakukan Android.

**Kompatibilitas rilis:** identitas aplikasi adalah `com.titotfp.wuwaid`, versi rebuild `0.4.0` / versionCode **8**. Tujuh APK publik v0.1.0–v0.3.4 memakai package yang sama dan sertifikat SHA-256 `2cb1b4fed5446cf52018b5d56d58c379d7328022c1ba17278c19c0002e484df9`. Upgrade in-place memerlukan keystore rilis asli; jangan uninstall atau menghapus data untuk upgrade. APK debug memakai `com.titotfp.wuwaid.debug` dan bukan pengganti APK rilis. Rilis berikutnya harus menaikkan versionCode serta menyamakan versi npm, Cargo, Tauri, dan tag. Workflow rilis memerlukan persetujuan QA manual dan memeriksa identitas/sertifikat APK. Detail build dan upgrade ada di [panduan Android](docs/android.md).

### Upgrade dari aplikasi sebelum Tauri (WM-25)

- APK publik v0.1.0–v0.3.4 memakai package dan sertifikat yang tercantum di atas. Upgrade in-place memakai APK release dengan keystore asli; jangan uninstall atau hapus data. APK debug bukan jalur upgrade.
- Setelah memasang release, pilih metode akses dan jalankan **Perawatan → Migrasikan**. Migrasi offline mencocokkan hash/ukuran PAK historis, signature, dan descriptor; sumber lama dipertahankan sampai set baru terverifikasi. Berkas ambigu/tidak dikenal memblokir mutasi.
- Jika lini game/resource tidak cocok setelah migrasi, pasang rilis PAK stabil yang kompatibel. Terapkan UID setelah pemasangan sehat. [Panduan Android](docs/android.md) memuat prosedur dan QA lengkap.

## Privasi dan batas platform

Diagnostik `schemaVersion: 1` berisi bagian `application`, `device`, `probe`, `resource`, `patch`, dan `release`; tiap bagian berstatus `ok`, `error`, atau `not_checked` dengan `data`, `error`, dan `reason`. Bagian yang gagal/tidak diperiksa memakai `data: null`, bukan klaim bahwa terjemahan tidak terpasang. UID kustom tidak disertakan; `uidCustomized` hanya dicatat setelah inspeksi pemasangan berhasil, dan error yang mengetahui nilainya meredaksi teks UID. History hanya untuk sesi frontend, maksimal 20 error dengan pesan maksimal 1024 unit UTF-16 setelah redaksi, dan hilang saat frontend dimuat ulang.

Android **Simpan** baru berstatus `saved` setelah tulis, flush, dan tutup berhasil; batal dan gagal dibedakan. **Bagikan** membuka chooser read-only, tetapi `share_opened` bukan bukti laporan diterima. Laporan tidak berisi logcat/trace atau payload PAK dan tidak diunggah otomatis. Di browser, Simpan hanya mengonfirmasi unduhan dimulai; Bagikan memakai Web Share, atau gagal bila tidak tersedia. Ekspor native hanya tersedia di Android; desktop menandai status game `not_checked`.
Browser memiliki 11 skenario simulasi: siap, terpasang, game tidak ditemukan, versi tidak didukung, resource belum siap, Shizuku mati/izin belum diberikan, root menunggu izin/siap tanpa Shizuku, pemulihan tertunda, dan instalasi sebelum Tauri. Semua akses, instalasi, UID, penghapusan, dan diagnostik adalah mock—tanpa root nyata, penulisan game, atau peluncuran game. UID `900123456`, versi `3.7.0`, dan konten `3.7.0-R2` adalah data contoh. State memakai `wuwaidmobile.mock-state.v5`; memilih skenario mereset data contoh dan UID. Desktop Tauri tidak memasang PAK Android dan menampilkan batas platform.

## Browser dan pengembangan

Persyaratan umum: Node.js 24 dan npm, Rust stable melalui rustup. Untuk Android, gunakan Android Studio, JDK 17, Android SDK, dan NDK yang sesuai; lihat [panduan Android](docs/android.md). Target desktop juga memerlukan dependensi sistem Tauri.

```sh
npm ci

# Frontend browser (mock IPC)
npm run dev

# Desktop Tauri
npm run tauri dev

# Android: sambungkan perangkat dengan USB debugging atau jalankan emulator
adb devices
npm run tauri android dev
```

Pemeriksaan dan APK debug ARM64:

```sh
npm run check
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
npm run tauri android build -- --debug --apk --target aarch64
```

APK tersebut untuk verifikasi, bukan paket rilis dengan keystore asli. GitHub Actions menjalankan pemeriksaan frontend/Rust dan mengunggah APK debug sebagai artifact. Cara berkontribusi ada di [`CONTRIBUTING.md`](CONTRIBUTING.md); perubahan tercatat di [`CHANGELOG.md`](CHANGELOG.md).

## Kontrak IPC

State frontend didefinisikan di `src/model.ts`; implementasi installer/downloader di `src-tauri/src/pak/`, orkestrasi di `src-tauri/src/native.rs`, dan bridge Android di `src-tauri/src/android.rs`.

| Perintah | Argumen | Hasil |
|---|---|---|
| `wuwa_load` | `requestId` | `Dashboard` |
| `wuwa_set_scenario` | `scenario` | `Dashboard` (browser saja) |
| `wuwa_run` | `requestId`, `operation`, `uidText?` (`apply_uid`) | `Dashboard` |
| `wuwa_open_setup` | `target`: `game` / `shizuku` | — (native) |
| `wuwa_diagnostics` | `requestId` | `DiagnosticReport` (`schemaVersion`, `sections`, `privacy`, `errors`) |
| `wuwa_export_diagnostics` | `action`: `save` / `share`; `content`: JSON teredaksi | `saved` / `cancelled` / `failed` / `share_opened` (Android) |
| `wuwa_trace_enabled` | — | Status recording debug native (`boolean`) |
| `wuwa_ui_trace` | `requestId`, `event`, `frontendUs` | Event tetap; recording dinonaktifkan di release native |

Operasi native: `install`, `repair`, `uninstall`, `request_permission`, `request_root`, `use_root`, `use_shizuku`, `launch_game`, `apply_uid`, `restore_uid`. Status Shizuku (`shizuku`) terpisah dari metode akses terpilih (`access.method`, `access.granted`); kesiapan memakai grant metode terpilih. `game.packageName` berisi paket aktif. `patch.legacyPending` menandai instalasi sebelum Tauri; `repair` memilih migrasi offline atau pemulihan jurnal sesuai status. Mutasi bersamaan ditolak. `src/ipc.ts` membuat correlation ID; caller tidak perlu membuatnya. Detail trace dan analisis performa ada di [`docs/android-traces.md`](docs/android-traces.md).

## Referensi

- Implementasi terdahulu: [Resource Mount](https://github.com/TitoTFP/WuwaIDLauncher/blob/676c4a9b3b3f14ccb570d90252dc8972dbac8d13/src-tauri/src/engine/installer.rs#L95-L111), [algoritme UID](https://github.com/TitoTFP/WuwaIDLauncher/blob/676c4a9b3b3f14ccb570d90252dc8972dbac8d13/src-tauri/src/engine/patch_asset.rs), [ikon](https://github.com/TitoTFP/WuwaIDLauncher/blob/676c4a9b3b3f14ccb570d90252dc8972dbac8d13/src-tauri/icons/icon.png), dan [kontrak berkas v0.3.4](https://github.com/TitoTFP/WuwaIDMobile/blob/v0.3.4/app/src/main/java/com/titotfp/wuwaid/GamePaths.kt).
- [Shizuku UserService API](https://github.com/RikkaApps/Shizuku-API#userservice), [libsu 6.0.0](https://github.com/topjohnwu/libsu/tree/6.0.0), dan [reqwest connection pool](https://docs.rs/reqwest/0.12.28/reqwest/blocking/struct.Client.html).
- Rilis PAK historis: [daftar rilis](https://github.com/TitoTFP/WuwaID/releases) dan [v3.7.1-id.1](https://github.com/TitoTFP/WuwaID/releases/tag/v3.7.1-id.1).
