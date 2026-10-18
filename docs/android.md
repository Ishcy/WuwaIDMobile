# Pengembangan Android

Proyek Android Tauri tersimpan di `src-tauri/gen/android/` dan dikomit. Versi build lokal/CI: Android API 36, Build Tools 36.0.0, NDK 27.3.13750724, JDK 17.

## Persiapan dan build debug

Pasang melalui Android Studio SDK Manager:

- `platforms;android-36`
- `build-tools;36.0.0`
- `platform-tools`
- `cmdline-tools;latest`
- `ndk;27.3.13750724`

Pasang Rust stable dan target yang diperlukan. ARM64:

```sh
rustup target add aarch64-linux-android
```

Tambahkan target berikut bila membangun ABI tersebut atau memakai emulator x86_64:

```sh
rustup target add armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

Atur environment Android/JDK (sesuaikan lokasi mesin):

```sh
export JAVA_HOME="/path/to/jdk-17"
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export NDK_HOME="$ANDROID_HOME/ndk/27.3.13750724"
```

Windows PowerShell:

```powershell
$env:JAVA_HOME = "C:\Program Files\Android\Android Studio\jbr"
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
$env:NDK_HOME = "$env:ANDROID_HOME\ndk\27.3.13750724"
```

Dari root repo:

```sh
npm ci
adb devices
npm run tauri android dev
```

Checkout sudah memuat `src-tauri/gen/android/`; `npm run tauri android init` hanya untuk regenerasi native, misalnya setelah mengganti identifier di `src-tauri/tauri.conf.json`. Setelah regenerasi, pastikan `compileSdk` dan `targetSdk` di `src-tauri/gen/android/app/build.gradle.kts` cocok dengan SDK terpasang (saat ini 36).

Buat APK debug ARM64:

```sh
npm run tauri android build -- --debug --apk --target aarch64
```

Output berada di `src-tauri/gen/android/app/build/outputs/apk/`. CI membangun ARM64 dengan perintah sama dan menyimpan artifact 14 hari. APK debug memakai package `com.titotfp.wuwaid.debug`, versi `0.4.0-debug`; bisa berdampingan dengan release, tetapi bukan upgrade release. Jangan distribusikan signing debug atau jalankan dua installer bersamaan pada resource game yang sama.

### Profile debug

`profile.dev.debug = 1` menyimpan informasi modul/baris dan backtrace, tetapi bukan detail penuh tipe/variabel lokal. Profile dev memakai `opt-level = 2`; debug assertions, overflow checks, dan trace tetap aktif. Atur `CARGO_PROFILE_DEV_DEBUG=2` untuk detail debugging Rust penuh. Hanya task `package*Debug` yang nonincremental; task build lain tetap incremental.

## Ukuran APK dan resource (pengukuran WM-29)

| Pengukuran | Baseline WM-28 | WM-29 |
|---|---:|---:|
| APK debug ARM64 | 479.265.253 byte (457,06 MiB) | 120.178.389 byte (114,61 MiB) |
| `libwuwaidmobile_lib.so` | 236.674.584 byte | 112.417.248 byte |
| ELF section `.debug_*` total | 218.904.759 byte | 94.682.666 byte |
| Kompilasi regex per laporan (20 error) | 41 | 2 |
| Alokasi per panggilan (instance baru + metadata 256 KiB) | 811.348 byte | 283.716 byte |
| Cache fixture (termasuk file asing/staging) | 458.778 byte | 65.562 byte |

APK turun 74,9%; library native 52,5%. ZIP diukur dari central directory dan rentang local header/payload; ELF dengan `llvm-size -A`. Baseline memuat 233.540.556 byte data ZIP usang dari incremental packaging; APK akhir memiliki 929 entry dan hanya gap 4.096 byte untuk signing block. Build debug diulang tanpa `clean`; APK akhir diperiksa signature v2 dan alignment 16 KiB.

Angka alokasi berasal dari harness JVM sementara (5 warmup, 10 panggilan, `ThreadMXBean`, adapter `Os` ke file host); bukan syscall, RSS/PSS atau Binder Android. Regex diukur pada modul frontend yang dipakai UI dengan 20 error dan UID aktif. CPU, baterai, startup, dan RSS/PSS perangkat belum diukur.

## Optimasi storage dan APK release (WM-30)

Build sebelum/sesudah memakai source checkout yang sama selain perubahan WM-30: Linux x86_64, Node 24.20.0, Rust 1.97.1, JDK 17.0.20.1, API/Build Tools 36, NDK 27.3.13750724, ARM64, `npm run tauri android build -- --apk --target aarch64`. Identitas tetap `com.titotfp.wuwaid`, 0.4.0/code 8 dan key rilis asli. APK akhir lolos signature v2 dan `zipalign -c -P 16 4`; bukan bukti upgrade/game pada perangkat.

| Pengukuran (byte) | Sebelum | Sesudah | Penurunan |
|---|---:|---:|---:|
| APK release ARM64 | 10.550.849 | 9.970.529 | 5,50% |
| Library native di APK | 8.451.032 | 7.870.712 | 6,87% |
| Cache PAK setelah operasi pada fixture crash | 12.634.179 | 4.207.297 | 66,70% |
| Total fixture saat UID Apply sebelum jurnal | 25.257.891 | 12.623.712 | 50,02% |
| Total fixture saat UID Reset sebelum jurnal | 21.050.594 | 12.623.712 | 40,03% |
| Cache updater yatim setelah restart | 2.097.154 | 0 | 100% |
| Maksimum cache updater teramati di callback progres | 17.210 | 8.736 | 49,24% |

Metode dan batas:

- APK: ukuran file dan ZIP central directory. Dominan native; Cargo release beralih `opt-level = 3` → `"s"`, tetap LTO, satu codegen unit, abort, dan strip. `"z"` tidak dipilih agar loop vectorization tidak dimatikan. Tidak ada section `.debug_*` pada kedua ELF. R8/resource shrinking, ikon, locale, ABI, dependensi, dan konfigurasi signing tidak diubah. Pengaruh CPU/startup dari profile `"s"` belum diukur di perangkat.
- Rust: harness sementara menjalankan kode installer sebenarnya lewat `LocalStorage`, PAK v12 4.207.297 byte dengan SQLite UID valid dan payload 4 MiB. Awal identik: source terverifikasi, satu partial seukuran source, satu derived PAK seukuran source, database yatim 12 KiB. Install → UID Apply → Reset → uninstall berhasil; source dan file game bawaan fixture tetap utuh.
- Angka native adalah jumlah panjang file reguler pada checkpoint, **bukan high-water mark seluruh syscall**. UID masih memerlukan source, derived PAK, stage, dan pemasangan lama secara bersamaan saat staging; pelepasan derived setelah staging mengurangi footprint saat commit, bukan menghilangkan kebutuhan ruang puncak staging. Cache sumber tunggal tetap tersedia untuk penggunaan offline.
- Updater: Robolectric API 28, TLS localhost, APK ARM64 sungguhan yang ditandatangani key fixture sementara (8.474 byte). Baseline repository sebelum perubahan dan repository akhir menjalankan skenario sama: restart dengan orphan/partial, unduh kandidat, pilih aset dengan digest baru, unduh dan verifikasi lagi. Pengukuran progres tidak mengamati setiap write metadata. APK final, ukuran/SHA-256, identitas dan signing tetap diverifikasi; ini bukan installer Android asli.
- Tabel di atas hanya pengukuran host. Hasil Settings perangkat dicatat terpisah di bawah; alokasi blok filesystem, ruang PackageInstaller, dan puncak storage perangkat belum diukur. Penurunan ukuran APK bukan bukti penurunan total storage terpasang.

Audit artifact pembanding awal oleh subagent mengonfirmasi SHA-256, identitas/signing, ARM64, serta kesamaan library optimized dengan output build pada fase pengukuran itu (`src-tauri/target/wm30/qa/host-audit.txt`). **Alignment ZIP bukan alignment ELF**: kedua library awal memiliki `PT_LOAD.p_align = 0x1000` (4 KiB). Kelulusan `zipalign -P 16` tidak membuktikan loading native pada perangkat dengan page size 16 KiB; dukungan runtime tersebut belum diuji.

Artifact pembanding sebelum koreksi lock berada di `src-tauri/target/wm30/`: `baseline-release.apk`, `optimized-release.apk`, dan `SHA256sums.txt`. **Jangan gunakan `optimized-release.apk` untuk tes lanjutan:** pengguna melaporkan operasi PAK gagal pada API lock Android. APK koreksi memakai nama `cache-lock-fixed-release.apk`; baseline/optimized lama dipertahankan hanya untuk provenance pengukuran. Output build standar tetap `src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk`. Artifact tidak dikomit.

APK koreksi lock: **9.970.593 byte**, SHA-256 `a3f6a06b5ecb01c232fa994cd387427e912f78d3633bf175ca37034913974f79`, package/key/0.4.0/code 8 tetap. Signature v2 dan alignment ZIP 16 KiB lolos; library yang dikemas cocok output build dan mengimpor `flock@LIBC`. Sebanyak 42 tes Rust dan smoke lock dua proses lulus. Pengguna kemudian mengonfirmasi tes perangkat aman setelah investigasi konflik PAK manual di bawah; hasil pengukuran artifact awal tidak diganti.

### Hasil QA perangkat asli (bertahap)

Pada fase awal, subagent DeviceQA menguji Samsung SM-A525F, Android 14/API 34, ARM64; Wuthering Waves Global 3.7.0. Package debug tetap dipertahankan. Package release awalnya belum ada: baseline dipasang fresh, lalu diganti optimized memakai `adb install -r`, tanpa uninstall/clear data/downgrade. Optimized yang ditarik dari perangkat cocok byte/digest/signing dengan artifact fase awal.

Settings → App info → Storage, sesudah launch dan cek update manual pada setiap fase:

| Komponen (unit sesuai UI, dibulatkan) | Baseline fresh | Optimized setelah replacement |
|---|---:|---:|
| App | 12,24 MB | 11,66 MB |
| Data | 4,42 MB | 8,63 MB |
| Cache | 438 KB | 442 KB |
| Total | 17,11 MB | 20,74 MB |

Komponen App turun, tetapi total naik karena Data bertambah. Data dipertahankan dan urutan pengujian tidak mengisolasi pertumbuhan state/WebView; jangan mengatribusikannya pada perubahan kode atau mengklaim total storage turun. Ini bukan pengukuran peak.

- Cek update manual pada kedua APK menampilkan belum ada stable resmi; API latest mengembalikan 404. Download/PackageInstaller belum diuji; kedua artifact code 8 bukan kandidat versi lebih tinggi.
- Pada fase awal, Shizuku 13.6 terpasang tetapi UI manager dan WuwaID menyatakan tidak berjalan setelah startup ADB resmi mengembalikan exit 0. Snapshot proses hanya memuat manager, bukan `shizuku_server`; log terfilter tidak memberi penyebab (`qa/shizuku-start-output.txt`). Grant dan operasi game belum diuji pada fase tersebut; exit starter bukan bukti service siap.
- Patch milik WuwaID dan status adanya UID kustom dipertahankan; teks privat tidak dibaca/disimpan. Empat file resmi sampel (dua mount, satu PAK beserta signature) memiliki SHA-256 sebelum/sesudah yang sama; bukan validasi seluruh tree game.
- Pengamatan log PID/exit-info tidak menemukan crash/ANR optimized pada jendela uji. Bukti: `src-tauri/target/wm30/qa/`, terutama `storage-settings.txt`, `package-identity.txt`, `runtime-observations.txt`, `game-{initial,final}-sha256.txt` dan screenshot `baseline-storage-breakdown/screen.png`, `optimized-storage-breakdown/screen.png`.

Retest berikutnya memakai APK koreksi lock dan Shizuku wireless debugging yang dinyalakan pengguna. Pengguna mengonfirmasi pemasangan dengan UID bawaan aman, lalu melaporkan UID kustom tidak tampil. Pemeriksaan read-only membuktikan PAK installer dan ketiga baris UID sudah berisi teks kustom, tetapi ada PAK terjemahan manual lain di `Saved/Paks` yang hash-nya identik dengan rilis asli. Sesudah temuan dan instruksi menonaktifkan PAK manual, pengguna mengonfirmasi hasil aman. Ini penerimaan pengguna, bukan retest otomatis; lima siklus, recovery setelah interupsi, peak storage, dan PackageInstaller update lebih tinggi belum dikonfirmasi.

### Konflik PAK manual dengan UID kustom

Jika UID kustom tercatat aktif tetapi game tetap menampilkan UID bawaan, periksa apakah ada terjemahan manual di:

```text
Android/data/<package-game>/files/UE4Game/Client/Client/Saved/Paks/
```

Pada perangkat QA ditemukan `pakchunk0-ID-WindowsNoEditor_1000_P.pak` tanpa UID kustom, di luar kepemilikan installer `Resources/<versi>/wuwaid`. PAK manual dapat menimpa database yang sama; status sehat installer hanya memverifikasi berkas miliknya, bukan seluruh urutan mount game.

Tutup paksa game, pindahkan hanya PAK terjemahan manual yang dikenali ke backup di luar direktori game, lalu buka game kembali. Jangan hapus PAK resmi atau berkas `Resources/<versi>/wuwaid`. Jika PAK installer sudah memuat UID kustom, tidak perlu reinstall APK atau mengunduh ulang PAK untuk retest ini. Aplikasi tidak memindahkan/menghapus file manual tersebut otomatis.

### Prosedur QA storage perangkat

1. Catat model/Android, filesystem, package game, metode Root/Shizuku, versi game/PAK, serta kondisi WebView yang sama. Bandingkan kedua APK tanpa uninstall/clear data. Catat ukuran aplikasi/data/cache dari Settings sebelum dan sesudah skenario; ukur puncak dengan tooling yang tersedia dan izin yang sudah diberikan, atau tandai belum terukur.
2. Ulangi install/update, UID Apply/Reset, repair/recovery, dan uninstall. Periksa hash file game bawaan sebelum/sesudah dan hasil UID/terjemahan di game. Backup/jurnal recovery tidak boleh dibersihkan sebagai cache.
3. Putuskan jaringan atau hentikan proses saat unduh PAK/UID; setelah membuka ulang, jalankan operasi cache berikutnya. Partial/UID scratch yang namanya direservasi harus hilang; source terverifikasi dan file asing tetap ada. Kegagalan staging/commit harus tetap dapat diperbaiki lewat recovery.
4. Jalankan [QA updater](#qa-updater-wajib), termasuk versi stabil resmi yang lebih tinggi, installer batal/izin ditolak, retry offline dari APK terverifikasi, dan restart setelah update. Partial/yatim dibersihkan saat restore; APK ready tidak dihapus hanya karena installer batal atau pemeriksaan jaringan gagal.
5. Ulangi sedikitnya lima siklus pada kondisi sama. Catat byte/MiB sebelum/sesudah dan persentase, kegagalan cleanup, ruang puncak yang benar-benar teramati, serta hasil pass/fail. Hasil host di atas tidak menggantikan penerimaan perangkat.


## Nama dan ikon

Nama tampilan **WuwaID Mobile**. Samakan `productName` dan judul window di `src-tauri/tauri.conf.json` dengan `app_name` dan `main_activity_title` di `src-tauri/gen/android/app/src/main/res/values/strings.xml`. `com.titotfp.wuwaid` adalah identifier, bukan nama tampilan; jangan ubah untuk rebranding.

Sumber ikon `src-tauri/app-icon.png` berasal dari [WuwaIDLauncher](https://github.com/TitoTFP/WuwaIDLauncher/blob/676c4a9b3b3f14ccb570d90252dc8972dbac8d13/src-tauri/icons/icon.png). Regenerasi:

```sh
npm run tauri icon -- src-tauri/app-icon.json
```

CLI menghasilkan ikon bundle di `src-tauri/icons/` dan Android langsung di `src-tauri/gen/android/app/src/main/res/`, termasuk adaptive icon Android 8+ dan density mdpi–xxxhdpi. Aset iOS juga dihasilkan, tetapi iOS tidak dikonfigurasi dan aset itu tidak disertakan.

`src-tauri/app-icon.json` memakai `app-icon-android-foreground.svg` untuk adaptive foreground: artwork 72×72 di tengah kanvas 108×108 (padding transparan 18 tiap sisi), sesuai viewport 72 dp. Ukuran awal 66×66 menyisakan bingkai putih; 72×72 memperbesar foto sekitar 9% sambil mempertahankan mask launcher. Jangan ganti dengan foreground penuh, regenerasi langsung dari PNG, atau mengandalkan `android_fg_scale` (CLI yang dipakai tidak mengecilkannya). Untuk xxxhdpi, batas artwork `(72,72)–(360,360)` pada kanvas 432×432.

Periksa APK final—alpha/padding, label aplikasi/activity, dan resource ikon—bukan hanya RGB/sumber PNG. Preview mask lokal bukan screenshot perangkat. Uji update tanpa clear data; launcher dan info aplikasi harus menampilkan nama/ikon baru. Kandidat branding terbaru dan checksum dicatat di README.

## Release dan upgrade

Tujuh APK publik v0.1.0–v0.3.4 memakai package `com.titotfp.wuwaid`, versionCode 1–7, dan sertifikat SHA-256:

```text
2cb1b4fed5446cf52018b5d56d58c379d7328022c1ba17278c19c0002e484df9
```

Konfigurasi rebuild: versionCode **8**, versionName **0.4.0**. Untuk setiap release, naikkan `bundle.android.versionCode` dari code publik terakhir dan samakan versi Tauri, Cargo, dan npm. Jangan turunkan code atau mengganti package/signing key. Jangan memakai `adb install -d` sebagai bukti upgrade normal.

Gunakan private keystore rilis asli. Sediakan dari secret store lokal/CI, bukan Git/chat:

- Local build: `ANDROID_KEYSTORE_PATH`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`.
- Workflow `.github/workflows/release.yml`: `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`.

Build signed release:

```sh
npm run tauri android build -- --apk --target aarch64
```

Task `verifyUpgradeSigning` memeriksa sertifikat alias dan versionCode sebelum build. Secret tidak lengkap atau sertifikat berbeda menggagalkan build; tidak ada fallback debug atau pembuatan key baru. Sertifikat publik dalam APK tidak dapat menggantikan private key.

Workflow release dipicu manual pada tag yang sudah ada. Workflow memeriksa package/version/certificate APK final dan alignment 16 KiB, lalu mengunggah kandidat signed beserta checksum sebagai artifact QA selama 14 hari. `upgrade_qa_completed=false` (default) tidak memublikasikan GitHub Release; set `true` hanya setelah pengguna menguji source/tag yang sama. Keystore sementara dihapus meskipun job gagal. Checklist rinci historis ada di README WM-25.

Release yang disetujui memakai `prerelease: false`, termasuk tag `v0.x`, agar tersedia pada updater kanal stabil. Nama aset wajib `WuwaID-Mobile-v<versi>.apk`, tepat satu aset cocok, dan checksum harus tersedia melalui GitHub digest atau `SHA256sums.txt`; ini kontrak updater, jangan ubah tanpa migrasi updater.

### QA updater wajib

1. Mulai dari APK publik lama; pasang kandidat dengan `adb install -r`, tanpa uninstall/clear data. Uji deteksi, migrasi, recovery, UID, dan game; pastikan mount/signature benar dan hash berkas game bawaan tidak berubah sebelum/sesudah. APK debug hanya menguji migrasi berkas; package berbeda tidak menguji PackageManager/data upgrade in-place.
2. Periksa banner, versi terpasang, dan tombol manual. Jika tidak ada stable yang lebih baru (termasuk hanya ada prerelease), UI harus menyatakan belum ada update stabil, bukan bahwa versi terpasang terbaru.
3. Uji unduh/pasang normal hanya dengan stable resmi yang lebih baru, misalnya `0.4.1`/code 9, package dan key yang sama serta aset/checksum sesuai kontrak. Jangan gunakan metadata palsu atau menurunkan versionCode.
4. Uji perbandingan versi numerik (`0.4.9` → `0.4.10`), progres, ukuran/digest, dan penolakan APK dengan package, signing/tag, versionCode, minSdk, ABI, atau signature tidak sesuai. APK invalid tidak boleh mencapai installer.
5. Tolak izin sumber tak dikenal lalu coba lagi setelah grant; tutup app dari pengaturan izin dan buka lagi untuk memastikan APK cache diverifikasi ulang. Batalkan installer: jangan laporkan sukses dan sediakan retry. Setelah sukses, versi baru harus terbuka, data/PAK bertahan, dan downgrade tidak ditawarkan.
6. Putuskan jaringan saat cek/unduh atau hentikan app saat download. Partial/corrupt cache tidak siap dipasang dan harus dapat diunduh ulang. Background check tidak boleh mencuri fokus/draft UID atau memblokir akses game.

Hasil host/build/metadata bukan bukti penerimaan PackageManager, alur installer, atau game. Catat cakupan uji perangkat yang benar-benar dijalankan; laporan penerimaan pengguna sebelumnya tidak membuktikan semua skenario checklist ini.

## Tes host dan instrumentation Android

Tes host frontend/Rust (sesudah environment Android disiapkan):

```sh
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
```

`ResourceFilesTest` memiliki delapan tes instrumentation: batas teks 256 KiB/file kosong; FD saat sukses/gagal; digest segar setelah mutasi ukuran tetap, pergantian mode, atau deletion; batch 64 vs 65 dan urutan hasil; path invalid, symlink, FIFO, serta direktori non-file. Fixture berada di cache privat app debug, bukan folder game. Build/packaging saja tidak berarti tes dijalankan.

Setelah build debug ARM64 dari bagian persiapan di atas, siapkan JVM tes:

Suite Robolectric yang mencakup API 36 memerlukan **JDK 21** (JDK 17 ditolak sebelum tes kelas dijalankan). Setelah build APK dengan JDK 17, ubah JVM untuk tes; CI/release melakukan pergantian yang sama:

```sh
export JAVA_HOME="/path/to/jdk-21"
export PATH="$JAVA_HOME/bin:$PATH"
```


```sh
./src-tauri/gen/android/gradlew -p src-tauri/gen/android \
  :app:testUniversalDebugUnitTest :app:assembleUniversalDebugAndroidTest \
  -PabiList=arm64-v8a -ParchList=arm64 -PtargetList=aarch64 \
  -x :app:rustBuildArm64Debug --no-configuration-cache --console=plain
```

Pengecualian `rustBuildArm64Debug` hanya memakai library ARM64 yang baru dibangun; Tauri build harus sukses pada source sama terlebih dahulu. ABI properties membatasi packaging ke ARM64. Jangan pakai pengecualian setelah mengubah source native tanpa membangun ulang lewat Tauri CLI.

Pada perangkat ARM64, pengguna dapat memasang dan menjalankan instrumentation:

```sh
adb install -r src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
adb install -r src-tauri/gen/android/app/build/outputs/apk/androidTest/universal/debug/app-universal-debug-androidTest.apk
adb shell am instrument -w \
  -e class com.titotfp.wuwaid.storage.ResourceFilesTest \
  com.titotfp.wuwaid.debug.test/androidx.test.runner.AndroidJUnitRunner
```

Package test `com.titotfp.wuwaid.debug.test`; runner `androidx.test.runner.AndroidJUnitRunner`; target `com.titotfp.wuwaid.debug`, minSdk 24, targetSdk 36. Catat versi Android dan hasil pass/fail/skip. Tes host/instrumentation tidak membuktikan Binder, izin/kematian service Root/Shizuku, scoped storage game, atau interpretasi mount game. QA perangkat juga harus mencakup kedua package game dan metode akses yang tersedia, izin ditolak/diberikan, service terputus, upgrade saat UserService lama masih hidup (pastikan version change membuat service baru), install/repair/uninstall, recovery/migrasi legacy valid, UID Apply/Reset, dan hasil di game.

## Diagnostik dan ekspor

JSON diagnostik `schemaVersion: 1` mencakup `generatedAt`, `mock`, `backend`, `sections`, `privacy`, dan `errors`; bagian application/device/probe/resource/patch/release dikumpulkan independen. `status: ok` hanya berarti pemeriksaan selesai: cek `patch.data.healthy` dan pesan masalah. `error`/`not_checked` memakai `data: null`, bukan `installed: false`.

Ekspor tidak memasang PAK, meminta root grant, atau mengunggah laporan. Release PAK hanya dibaca dari cache sesi. Metadata perangkat tidak memuat serial/ID akun; UID kustom dihilangkan. Teks bebas meredaksi UID aktif dan input operasi yang diketahui. History hanya sesi frontend, maksimal 20 error; pesan yang tersimpan dibatasi 1024 unit UTF-16 setelah redaksi. Reload menghapus history. Logcat/trace tidak disertakan.

Regex UID dibuat sekali per snapshot/serialisasi, longest-first agar UID yang memuat `[REDACTED]` atau karakter regex tidak lolos sebagai substring; pola tidak disimpan lintas panggilan/sesi.

- Android **Simpan** memakai `ACTION_CREATE_DOCUMENT` (`application/json`) dan menulis UTF-8 ke URI `content://` pilihan pengguna. Status `saved` hanya setelah write/flush/close sukses; cancel, URI/hasil dialog invalid, kegagalan write/close, atau lifecycle tertutup bukan sukses. Provider bisa meninggalkan dokumen parsial saat gagal.
- Android **Bagikan** memakai `ACTION_SEND`, chooser, FileProvider dan ClipData dengan grant read-only. Setiap laporan punya URI unik; fitur hanya menambah cache `diagnostics/`, tidak mengubah cakupan updater `app-updates/`. Laporan fitur yang lebih tua dari 24 jam dibersihkan pada share berikutnya. `share_opened` berarti chooser terbuka, bukan penerima berhasil mengirim/menerima.
- Satu ekspor aktif menahan snapshot hingga dialog/worker selesai; lifecycle menolak request tertunda dan mengabaikan callback terlambat. Browser memakai Blob download (`download_started`, bukan `saved`) dan Web Share bila didukung. Desktop native tidak menyediakan dialog Android.

Pemeriksaan perangkat: simpan dan buka JSON UTF-8; batalkan pemilih dokumen dan pastikan tidak ada status sukses. Bagikan lalu buka di penerima dan bandingkan isinya dengan hasil simpan; chooser terbuka bukan bukti pengiriman dan laporan sebelumnya harus tetap immutable. Uji tanpa grant, dengan Root/Shizuku, dan setelah service terputus: bagian yang tak diperiksa harus `null`. Terapkan UID privat dan operasi gagal, lalu pastikan UID tidak muncul dalam laporan/error; history maksimal 20 dan hilang setelah reload. Catat Android, provider penyimpanan, penerima, serta hasil/batal/gagal.

## Retensi cache PAK dan batas keselamatan

Cache privat `release-paks` memakai nama `<tag-rilis>-<nama-asset>.pak`. `release::download` mengembalikan `CachedPak`; caller wajib mempertahankannya hingga instalasi/UID selesai. Handle ini memegang OS file lock `.cache.lock`, sehingga operasi kedua gagal sebelum cleanup, dan kematian proses melepas lock otomatis.

Koreksi regresi Android: Rust 1.97 `std::fs::File::try_lock()` mengembalikan `Unsupported` pada target Android, sehingga tes Linux sebelumnya tidak membuktikan operasi lock pada perangkat. Android dan Linux sekarang berbagi `libc::flock(fd, LOCK_EX | LOCK_NB)`; platform lain tetap memakai API std. `WouldBlock` dilaporkan sebagai kontensi, sedangkan error OS lain ditampilkan sebagai kegagalan lock yang sebenarnya. Tidak ada penghapusan lock atau fallback tanpa lock.

Smoke host dua proses pada jalur produksi memverifikasi penolakan kontensi sebelum cleanup, scratch aktif tetap utuh, lock terlepas setelah proses pemegang dihentikan, serta akuisisi ulang dengan source terverifikasi tetap utuh. [Hasil perangkat](#hasil-qa-perangkat-asli-bertahap) dicatat terpisah dari smoke host. Berkas `.cache.lock` yang tertinggal tidak perlu dihapus karena kepemilikan lock ada pada descriptor/proses, bukan keberadaan nama file.

Setelah metadata rilis valid dan lock eksklusif diperoleh, operasi cache berikutnya membersihkan scratch yatim: regular `.<tag>-<asset>.pak.part-<pid>-<counter>`, regular `uid-pak-<6 alfanumerik>.pak`, dan direktori `uid-work-<6 alfanumerik>` beserta isinya tanpa mengikuti symlink. Prefix/suffix yang tidak cocok, symlink di root cache, file asing, dan direktori lain tidak disentuh. Drop normal tetap membersihkan partial/UID; cleanup sesudah proses mati menunggu operasi cache berikutnya, bukan passive dashboard refresh.

Setelah cache hit terverifikasi atau unduhan dipublikasikan atomik dengan ukuran/SHA-256 benar, source regular lama yang cocok validator tag/asset dapat dihapus; source saat ini dipertahankan. Metadata, transfer, atau integritas gagal sebelum verifikasi/publikasi tidak mengeviksikan source lama. Kegagalan inspeksi/penghapusan cache menghentikan operasi sebelum penulisan game. Eviksi yang sudah selesai tidak dipulihkan bila instalasi berikutnya gagal; rollback game tidak bergantung pada source cache lama.

Derived PAK UID dihapus setelah staging selesai, sebelum penanganan jurnal/commit. Stage, backup dan jurnal milik installer tetap mengikuti pemeriksaan hash/kepemilikan dan recovery; tidak ada perubahan format PAK, mount/signature, IPC, atau akses Root/Shizuku. Satu source tidak membatasi ukuran seluruh direktori; staging UID tetap membutuhkan beberapa salinan.

Cache `app-updates` membersihkan `update.part.apk`, `candidate.json.part`, serta `update.apk` tanpa metadata saat restore pertama. Pemilihan kandidat berbeda yang sudah valid membuang APK/metadata lama; cek gagal/offline mempertahankan kandidat ready. Unduh kandidat ready memverifikasi ulang file tanpa transfer atau partial kedua. File asing tidak dihapus. Cleanup yang gagal dilaporkan, bukan dianggap sukses. Installer batal tetap menyimpan APK untuk retry; versi terpasang yang sudah menyamai kandidat membuang cache saat restore.

`read_text` mengisi satu `ByteArray` sesuai ukuran file yang sudah diperiksa (batas 256 KiB), tanpa scratch buffer atau salinan akhir. Hash mengalokasikan buffer 64 KiB saat diperlukan dan menggunakannya ulang dalam instance service; file kosong/short read, no-follow, identitas file, dan penutupan FD tetap ditangani. Tidak ada polling, retry, telemetry, probe sintetis, atau cache metadata/hash lintas operasi. UID PAK rebuild tetap memakai buffer entry `repak`, bukan constant-memory.

UserService Shizuku harus memulai ulang implementasi setelah upgrade kode: `UserServiceArgs.version` dinaikkan 3 → 4 agar proses service lama tidak dipakai dengan kode baru. Ikuti [kontrak Shizuku untuk version/recreating service](https://github.com/RikkaApps/Shizuku-API/blob/a27f6e4151ba7b39965ca47edb2bf0aeed7102e5/api/src/main/java/rikka/shizuku/Shizuku.java#L624-L633).

## Referensi

- [Prasyarat Tauri](https://v2.tauri.app/start/prerequisites/)
- [Perintah Android Tauri CLI](https://v2.tauri.app/reference/cli/#android-build)
- [Cargo: debug information dan runtime checks](https://doc.rust-lang.org/cargo/reference/profiles.html#debug)
- [Gradle: incremental task](https://docs.gradle.org/current/userguide/custom_tasks.html#incremental_tasks)
