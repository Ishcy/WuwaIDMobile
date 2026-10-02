# Kontribusi

1. Jalankan `npm ci` untuk memasang dependensi yang terkunci.
2. Sebelum membuka pull request, jalankan pemeriksaan yang relevan:

   ```sh
   npm run check
   npm run build
   cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
   cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
   cargo test --manifest-path src-tauri/Cargo.toml --all-targets
   ```

3. Untuk perubahan Android, jalankan aplikasi di emulator/perangkat atau buat APK
   debug sesuai [`panduan Android`](docs/android.md).
4. Sertakan langkah reproduksi atau verifikasi pada deskripsi pull request dan
   perbarui dokumentasi bila instruksi setup atau perilaku berubah.

Jangan commit kredensial signing, `local.properties`, keluaran build, atau
`node_modules`. Folder sumber Android Tauri `src-tauri/gen/android/` tetap dilacak
karena berisi proyek native yang dikonfigurasi untuk aplikasi ini.
