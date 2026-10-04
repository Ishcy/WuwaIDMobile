# Pengembangan Android

Proyek Android Tauri berada di `src-tauri/gen/android/` dan disimpan di Git.
Build lokal memakai Android API 36, Android Build Tools 36.0.0, NDK 27.3.13750724,
dan JDK 17; samakan versi paket SDK dengan konfigurasi CI.

## Prasyarat

Pasang Android Studio dan komponen berikut melalui SDK Manager:

- Android SDK Platform 36 (`platforms;android-36`)
- Android SDK Build Tools 36.0.0 (`build-tools;36.0.0`)
- Android SDK Platform-Tools (`platform-tools`)
- Android SDK Command-line Tools (`cmdline-tools;latest`)
- NDK 27.3.13750724 (`ndk;27.3.13750724`)

Pasang Rust stable dan target yang akan dibangun. Untuk perangkat ARM64:

```sh
rustup target add aarch64-linux-android
```

Untuk juga menjalankan emulator x86_64 atau membangun ABI tambahan:

```sh
rustup target add armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

Atur `JAVA_HOME`, `ANDROID_HOME`, dan `NDK_HOME`. Contoh Linux/macOS:

```sh
export JAVA_HOME="/path/to/jdk-17"
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export NDK_HOME="$ANDROID_HOME/ndk/27.3.13750724"
```

Di Windows PowerShell, gunakan lokasi JDK dan SDK yang dipasang pada mesin:

```powershell
$env:JAVA_HOME = "C:\Program Files\Android\Android Studio\jbr"
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
$env:NDK_HOME = "$env:ANDROID_HOME\ndk\27.3.13750724"
```

## Menjalankan di perangkat atau emulator

Pasang dependensi frontend, lalu pastikan perangkat terhubung:

```sh
npm ci
adb devices
npm run tauri android dev
```

Tauri menggunakan `src-tauri/gen/android/` yang sudah tersedia di checkout. Perintah
`npm run tauri android init` hanya diperlukan untuk menghasilkan ulang proyek native,
misalnya setelah mengubah identifier Android di `src-tauri/tauri.conf.json`.

Setelah regenerasi, periksa nilai `compileSdk` dan `targetSdk` di
`src-tauri/gen/android/app/build.gradle.kts`; samakan dengan platform SDK yang
terpasang (saat ini API 36).

## Membuat APK debug

```sh
npm run tauri android build -- --debug --apk --target aarch64
```

APK berada di bawah `src-tauri/gen/android/app/build/outputs/apk/`. CI menjalankan
build ARM64 yang sama dan menyimpan hasilnya sebagai artifact selama 14 hari.

APK debug memakai `com.titotfp.wuwaid.debug` dan `0.4.0-debug`; dapat dipasang
berdampingan dengan APK publik, tetapi **tidak** meng-upgrade APK release.
Jangan gunakan signing debug untuk distribusi publik. Jangan mengoperasikan
dua installer bersamaan pada resource game yang sama.

## Nama dan ikon aplikasi

Nama tampilan adalah **WuwaID Mobile**. Pertahankan `productName` dan judul
window di `src-tauri/tauri.conf.json`, serta `app_name`/`main_activity_title`
di `src-tauri/gen/android/app/src/main/res/values/strings.xml` agar konsisten.
Identifier `com.titotfp.wuwaid` bukan nama tampilan dan tidak boleh diganti
untuk sekadar mengganti branding.

Sumber ikon `src-tauri/app-icon.png` adalah salinan dari
[WuwaIDLauncher](https://github.com/TitoTFP/WuwaIDLauncher/blob/676c4a9b3b3f14ccb570d90252dc8972dbac8d13/src-tauri/icons/icon.png).
Regenerasi dari root repository:

```sh
npm run tauri icon -- src-tauri/app-icon.json
```

CLI memperbarui ikon bundle di `src-tauri/icons/` dan launcher Android langsung
di `src-tauri/gen/android/app/src/main/res/`, termasuk adaptive icon Android 8+
dan PNG density mdpi–xxxhdpi. CLI juga membuat aset iOS; proyek ini tidak
mengonfigurasi iOS, sehingga aset iOS hasil generasi tidak disertakan.

Manifest `src-tauri/app-icon.json` mempertahankan artwork default dan memilih
`app-icon-android-foreground.svg` khusus untuk adaptive foreground. SVG menaruh
gambar 72×72 pada pusat kanvas 108×108, dengan padding transparan 18 pada setiap
sisi, mengisi viewport 72 dp dalam [adaptive icon Android](https://developer.android.com/develop/ui/compose/system/icon_design_adaptive).
Ukuran awal 66×66 menyisakan bingkai putih pada launcher pengguna; framing
72×72 memperbesar foto sekitar 9% sambil tetap mengikuti mask launcher.
Mengisi seluruh foreground membuat launcher memperbesar/memotong artwork.

Jangan regenerasi langsung dari PNG atau mengandalkan `android_fg_scale`: pada
CLI yang digunakan, parameter itu tidak mengecilkan adaptive foreground; SVG
berpadding mempertahankan geometri yang sama saat seluruh density dibuat ulang.
Untuk xxxhdpi, kanvas tetap 432×432 dan batas artwork adalah
`(72, 72)–(360, 360)` / 288×288. Periksa alpha/padding pada APK final, bukan
hanya kesamaan RGB dengan sumber. Preview mask lokal bukan screenshot perangkat.

Setelah build, periksa label aplikasi/activity dan resource ikon dalam APK final,
bukan hanya sumber PNG. Kandidat branding terbaru beserta checksum ada di README.
Pengguna perlu memasang update tanpa clear data dan memastikan launcher serta
halaman info aplikasi menampilkan nama/ikon baru.

## Membuat APK release yang dapat meng-upgrade versi lama

Gunakan **keystore rilis asli** dari workflow sebelum rebuild Tauri. Ketujuh APK
publik v0.1.0–v0.3.4 memakai `com.titotfp.wuwaid`, versionCode 1–7 dan sertifikat:

```text
2cb1b4fed5446cf52018b5d56d58c379d7328022c1ba17278c19c0002e484df9
```

Konfigurasi rebuild menetapkan versionCode **8** dan versionName **0.4.0**.
Untuk rilis berikutnya, naikkan `bundle.android.versionCode` dari nilai publik
terakhir dan samakan versi Tauri, Cargo, dan npm. Jangan menurunkan versionCode
atau mengganti identifier, signing key, maupun memakai `adb install -d` sebagai
bukti bahwa update normal bekerja.

Sediakan environment berikut dari penyimpanan rahasia lokal/CI, bukan dari Git
atau pesan chat:

- `ANDROID_KEYSTORE_PATH`: path keystore rilis asli.
- `ANDROID_KEYSTORE_PASSWORD`.
- `ANDROID_KEY_ALIAS`.
- `ANDROID_KEY_PASSWORD`.

Kemudian:

```sh
npm run tauri android build -- --apk --target aarch64
```

Task `verifyUpgradeSigning` berjalan sebelum build release, memeriksa sertifikat
alias dan versionCode. Kredensial yang tidak lengkap atau sertifikat berbeda
membatalkan build; tidak ada fallback debug atau pembuatan kunci baru. Private
key asli diperlukan—sertifikat publik APK tidak dapat menggantikannya.
Keystore asli telah disediakan pengguna setelah implementasi WM-25. Kandidat
release ARM64 `0.4.0` / versionCode 8 berhasil dibangun dengan signing asli;
package dan sertifikat APK final, signature v2, serta alignment 16 KiB telah
diperiksa. Lokasi APK/checksum ada pada bagian WM-25 README. Pengguna kemudian
melaporkan `adb install -r` berhasil dan hasil pengujian aman serta berfungsi
semestinya; WM-25 ditutup berdasarkan penerimaan itu. Rincian seluruh skenario
tidak diberikan, sehingga tidak diklaim sebagai cakupan matriks lengkap.
Asisten tidak menjalankan pengujian perangkat atau membangun ulang APK pada
penutupan WM-25 tersebut. Build berikutnya untuk branding dan WM-26 dicatat
terpisah, bukan dianggap sudah diterima oleh pengujian WM-25.

Workflow `.github/workflows/release.yml` menggunakan nama secret lama:
`ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`,
dan `ANDROID_KEY_PASSWORD`. Workflow hanya dipicu manual pada tag yang sudah ada,
memeriksa package/version/certificate APK final serta alignment 16 KiB, lalu
mengunggah kandidat signed beserta checksum sebagai artifact QA selama 14 hari.
Dengan `upgrade_qa_completed=false` (default), tidak ada publikasi GitHub Release.
Publikasi baru dijalankan dengan `upgrade_qa_completed=true` setelah pengguna
menguji kandidat dari tag/source yang sama. Keystore sementara dihapus walaupun
job gagal. Daftar penerimaan ada di bagian WM-25 README.
Publikasi yang telah disetujui ditandai `prerelease: false`, termasuk tag `v0.x`,
agar ditemukan updater kanal stabil. Draft/prerelease tidak ditawarkan updater.
Nama APK wajib `WuwaID-Mobile-v<versi>.apk`, dengan satu aset yang cocok serta
digest GitHub atau `SHA256sums.txt`; jangan mengganti nama aset tanpa memigrasikan
kontrak updater.

Untuk pengujian upgrade oleh pengguna: mulai dengan salah satu APK publik lama,
pasang kandidat release dengan `adb install -r` tanpa uninstall/clear data,
lalu jalankan deteksi, migrasi, recovery, UID dan game. APK debug yang dihasilkan
lokal hanya berguna untuk QA migrasi berkas; suffix berbeda tidak membuktikan
kompatibilitas PackageManager atau data aplikasi pada upgrade in-place.

## Tes host updater (WM-26)

Pengguna kemudian mengizinkan asisten membuat/menjalankan tes host dan smoke UI,
tanpa perangkat asli. Dari root repository, setelah prasyarat dan `npm ci`:

```sh
# Pada checkout baru, build ini juga menghasilkan path dependency Gradle Tauri.
npm run tauri android build -- --debug --apk --target aarch64
npm test
./src-tauri/gen/android/gradlew -p src-tauri/gen/android \
  :app:testUniversalDebugUnitTest --no-configuration-cache --console=plain
```

`npm test` memeriksa tipe source/tes lalu menjalankan **10 tes Vitest/jsdom**.
Task Gradle menjalankan **27 tes Kotlin/Robolectric**: 18 repository dan 9 plugin.
Fixture resource-only APK memakai binary manifest `aapt2`, pustaka ARM64 yang
dikompilasi NDK dan signing key sementara, bukan keystore rilis. MockWebServer
melayani byte APK dan metadata melalui TLS lokal; factory koneksi internal hanya
mengalihkan transport setelah validasi URL resmi. Pin produksi tidak berubah.

Cakupan meliputi versi numerik, draft/prerelease/aset ambigu, digest/fallback
checksum, HTTP gagal/redirect tidak aman, batas ukuran, unduhan terputus,
APK salah package/versi/code/signer/multi-signer/minSdk/ABI, signature rusak,
cache rusak/restored/kedaluwarsa, izin ditolak/grant aktual, pembatalan/gagal
pemasang, request tumpang tindih, destroy/callback terlambat dan retry manual.
UI mencakup busy, progres aksesibel, pemulihan error dan escaping.

JVM menjalankan jalur utama dengan framework **API 28** serta smoke metadata
API 36. Adaptor tes mengganti mmap libcore dengan pembacaan file JVM, mengakses
getter `SigningInfo` framework dan menyediakan rename atomik yang tidak
diimplementasikan shadow bawaan. Kriptografi parser API 28 tetap dijalankan.
Pada **API 24**, parser arsip eksternal adalah boundary host yang dimodelkan
memakai binary manifest asli dan `apksig`, karena resource legacy Robolectric
tidak memuat manifest APK eksternal. Ini bukan bukti PackageManager perangkat.

Regresi APK valid API 28 membuktikan perlunya
`GET_SIGNING_CERTIFICATES | GET_SIGNATURES` ketika membaca arsip APK:
[parser Android 9 hanya mengumpulkan sertifikat jika GET_SIGNATURES ditetapkan](https://github.com/aosp-mirror/platform_frameworks_base/blob/android-9.0.0_r1/core/java/android/content/pm/PackageManager.java#L4815-L4817).
Tes gagal sebelum perbaikan dan lulus setelahnya; APK bertanda tangan yang
dirusak tetap ditolak walaupun checksum metadata cocok dengan byte yang rusak.

Smoke Chromium 412×915 menjalankan UI sebenarnya dengan transport updater
simulasi: cek selesai mempertahankan draf/fokus/seleksi UID, progres mengunci
aksi game, unduhan selesai membuka aksi lagi, dan pembatalan tetap menyediakan
Pasang update. Mode browser normal tetap tidak menawarkan updater release.
CI dan workflow rilis menjalankan tes setelah bootstrap/build Android; workflow
GitHub belum dieksekusi oleh asisten. Host callback dan `RESULT_OK` tidak
membuktikan APK terpasang: checklist perangkat berikut tetap diperlukan.

Pada pre-commit review, `npm ci`, 10 tes frontend, 27 tes host Android
(dieksekusi ulang, tanpa skip/failure/error), 24 tes Rust dan build frontend
lulus. `rustfmt --check` untuk bridge baru `src-tauri/src/app_update.rs` lulus
setelah perbaikan format tanpa perubahan perilaku. `cargo fmt --check` seluruh
repository gagal pada format baseline `android.rs`, `native.rs`,
`pak/installer.rs`, `pak/legacy.rs`, `pak/local_storage.rs` dan `trace.rs`;
keenam berkas tersebut tidak diubah dalam WM-26. Gate format CI tetap memiliki
batas ini; tidak dilakukan reformat berkas yang tidak terkait updater.

## QA updater aplikasi (WM-26; dilakukan pengguna)

- Pasang kandidat release `0.4.0` / code 8 yang memiliki updater secara manual
  di atas APK sebelumnya, tanpa uninstall/clear data. APK debug tidak dapat
  menguji jalur update release dan harus menampilkan alasan batas dukungan.
- Buka aplikasi dan periksa banner; di Perawatan, periksa versi terpasang dan
  tombol manual. Bila hanya ada prerelease/belum ada stable, UI harus mengatakan
  belum ada rilis stabil, bukan menyatakan versi terpasang paling baru.
- Untuk menguji unduh/pasang end-to-end, diperlukan **rilis stabil resmi yang
  benar-benar lebih baru**, misalnya `0.4.1` / code 9, dengan package/signing yang
  sama dan nama aset/checksum sesuai kontrak. Jangan memakai data update palsu
  atau menurunkan versionCode untuk menyatakan jalur normal lulus.
- Periksa deteksi versi numerik (mis. `0.4.9 → 0.4.10`), progres, ukuran/digest,
  serta penolakan APK dengan package/signing/tag/versionCode/minSdk/ABI tidak
  sesuai. Tidak ada APK invalid yang boleh diteruskan ke pemasang.
- Tolak izin sumber tidak dikenal, kembali, lalu coba lagi dan izinkan;
  pemasang harus terbuka setelah grant. Tutup aplikasi di pengaturan izin
  kemudian buka kembali: APK cache harus diverifikasi ulang sebelum dipasang.
- Batalkan pemasang: jangan menampilkan update berhasil; APK tetap dapat dicoba
  lagi. Jika update berhasil, aplikasi yang dibuka ulang harus menampilkan versi
  baru, mempertahankan data/PAK dan tidak menawarkan downgrade.
- Putuskan jaringan ketika cek/unduh, hentikan aplikasi saat unduhan, lalu
  coba kembali. Partial APK tidak boleh dianggap siap; cache yang rusak harus
  ditolak dan menyediakan unduhan ulang. Pemeriksaan background tidak boleh
  mencuri fokus/draf UID atau memblokir akses game.
- Hasil tes host/build/metadata bukan bukti penerimaan alur pemasangan Android.
  Asisten telah menjalankan tes otomatis dan smoke browser di atas, tetapi tidak
  menjalankan installer Android nyata, ADB atau pengujian game untuk WM-26.
  Pengguna kemudian melaporkan **“sudah kuuji aman”** dan meminta pre-commit review
  serta commit. Ini dicatat sebagai penerimaan pengguna, bukan bukti bahwa semua
  skenario checklist di atas telah dijalankan; rincian cakupan tidak dilaporkan.

## Referensi

- [Prasyarat Tauri](https://v2.tauri.app/start/prerequisites/)
- [Perintah Android Tauri CLI](https://v2.tauri.app/reference/cli/#android-build)
