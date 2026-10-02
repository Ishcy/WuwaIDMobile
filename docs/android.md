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

APK debug bukan rilis distribusi. Keystore dan konfigurasi signing rilis belum
tersedia; jangan gunakan signing debug untuk distribusi publik.

Ikon di `src-tauri/icons/` masih aset bawaan template; ganti sebelum distribusi.

## Referensi

- [Prasyarat Tauri](https://v2.tauri.app/start/prerequisites/)
- [Perintah Android Tauri CLI](https://v2.tauri.app/reference/cli/#android-build)
