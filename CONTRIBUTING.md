# Kontribusi

## Persiapan

Jalankan `npm ci` untuk memasang dependensi yang terkunci.

## Sebelum pull request

Jalankan pemeriksaan yang relevan:

```sh
npm run check
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
```

Untuk perubahan Android, uji di emulator/perangkat atau buat APK debug sesuai [panduan Android](docs/android.md); jalankan tes Gradle pada [bagian Tes host dan instrumentation Android](docs/android.md#tes-host-dan-instrumentation-android). Sertakan langkah reproduksi dan pemeriksaan yang dijalankan dalam PR. Perbarui dokumentasi bila setup atau perilaku berubah.

## Jangan commit

Jangan commit kredensial signing, `local.properties`, keluaran build, atau `node_modules`. Tetap lacak `src-tauri/gen/android/`: direktori itu berisi proyek native yang dikonfigurasi untuk aplikasi ini.
