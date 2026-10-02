# WuwaIDMobile

Pendamping lokalisasi **Wuthering Waves dalam Bahasa Indonesia**, dibangun dengan
**Tauri 2** untuk desktop dan Android. Frontend menggunakan TypeScript, HTML, CSS,
dan Vite; shell native menggunakan Rust. Proyek Android berada di
`src-tauri/gen/android/`.

## Frontend dan pemasangan PAK

Frontend menggunakan desain orisinal **Ruang Resonansi**, bukan rekonstruksi
antarmuka APK referensi: tipografi editorial, palet tinta–lavender, ilustrasi
gelombang SVG buatan sendiri, dan tiga ruang kerja:

- **Beranda:** status game, Shizuku, resource, dan rilis; unduh/pasang ulang
  terjemahan dengan progres dan pemeriksaan kompatibilitas.
- **Kustomisasi:** pratinjau custom UID melalui mock IPC di browser. Fitur ini
  tidak diterapkan ke game oleh backend native; UID akun tidak berubah.
- **Perawatan:** pemeriksaan ulang, pemulihan transaksi, ekspor diagnostik JSON,
  dan penghapusan berkas milik aplikasi dengan konfirmasi.
- Navigasi horizontal di desktop dan dock bawah pada layar Android.
- **Terang / Gelap / Sistem** tersedia di header. Default mengikuti tema sistem;
  pilihan disimpan lokal dengan key `wuwaidmobile.theme` dan diterapkan sebelum
  CSS tampil. Mode Sistem mengikuti perubahan tema perangkat secara langsung.
  Pergantian tema tidak mereset isian atau menghentikan operasi.
- Progres hanya memperbarui indikator, bukan membangun ulang navigasi atau kontrol
  tema. Tema juga diterapkan pada dialog, toast, form, dan status error.

Layout dan navigasi ketiga layar diverifikasi di browser pada lebar 320, 390, 768,
1024, dan 1440 px tanpa overflow horizontal. Smoke browser mencakup delapan
skenario perangkat, pemasangan/progres, penerapan UID, pembatalan/konfirmasi
penghapusan, serta persistensi dan perubahan tema sistem. Verifikasi redesign ini
belum dilakukan pada APK/WebView Android.

### Android: pemasangan native

Aplikasi mengambil rilis stabil terbaru secara dinamis dari
[`TitoTFP/WuwaID/releases/latest`](https://github.com/TitoTFP/WuwaID/releases/latest).
Tag, nama aset, ukuran, dan SHA-256 bukan nilai yang dibundel ke aplikasi.
Unduhan memakai HTTPS, streaming ke cache privat aplikasi, serta pemeriksaan
ukuran dan SHA-256 sebelum digunakan. Cache yang dipakai ulang juga diverifikasi.
PAK diperlakukan sebagai berkas opaque: tidak diekstrak atau didecompile.

1. Pasang **Wuthering Waves Global** (`com.kurogame.wutheringwaves.global`).
2. Buka game dan selesaikan unduhan resource, termasuk paket bahasa Inggris.
3. Pasang dan jalankan [Shizuku](https://shizuku.rikka.app/), lalu berikan izin
   ke WuwaIDMobile melalui tombol **Berikan izin Shizuku**.
4. Periksa kembali perangkat, lalu pilih **Unduh & pasang terjemahan**.

Backend menemukan versi resource numerik yang sudah termaterialisasi dan pasangan
PAK/signature resmi, dengan preferensi `Lang_en`. Lini versi game dan resource
harus cocok dengan rilis. Game dihentikan sebelum penulisan. File yang dikelola
berada di bawah `Resources/<versi-resource>/`:

- `wuwaid/WuwaID_99_P.pak`: aset rilis yang sudah diverifikasi.
- `wuwaid/WuwaID_99_P.sig`: salinan signature resmi dari resource game.
- `wuwaid/.owner.json`: identitas dan hash pemasangan.
- `Mount/wuwaid.txt`: mount descriptor dengan prioritas 99.

Field path di descriptor adalah **nama dasar tanpa ekstensi**:

```text
::Mount::
wuwaid/WuwaID_99_P,99,<PAK_SHA1_UPPER>,<SIG_SHA1_UPPER>,,
::Del::
```

Kontrak ini sama dengan [Resource Mount WuwaIDLauncher](https://github.com/TitoTFP/WuwaIDLauncher/blob/676c4a9b3b3f14ccb570d90252dc8972dbac8d13/src-tauri/src/engine/installer.rs#L95-L111)
dan `removeSuffix(".pak")` pada bytecode APK Android asal. Folder `Patch/`
adalah layout launcher PC; layout Android tetap `wuwaid/`.

Pemasangan dari build lama yang mencantumkan `.pak` pada descriptor ditandai
**Perlu pemulihan**, bukan sehat. **Perbaiki pemasangan** memperbarui descriptor
dan hash kepemilikannya secara transaksional. **Hapus terjemahan** juga dapat
menghapus pemasangan lama yang terverifikasi tanpa harus memperbaikinya terlebih
dahulu; format mount yang tidak dikenal tetap ditolak.


PAK, signature, dan mount bawaan game tidak ditimpa. Target tanpa metadata
kepemilikan valid dan custom mount lain dengan prioritas 99 atau lebih tinggi
menghalangi pemasangan. Berkas disiapkan dan diverifikasi sebelum commit;
jurnal dan backup menyimpan snapshot untuk rollback/pemulihan. **Perbaiki**
memulihkan transaksi tertunda tanpa membutuhkan rilis atau koneksi jaringan,
termasuk transaksi pada versi resource lama. Tanpa transaksi tertunda,
perbaikan memasang ulang rilis latest yang kompatibel.
Residu `.new`/`.bak` tanpa jurnal valid tidak dihapus otomatis: aplikasi
menolak menebak kepemilikan berkas dan menampilkan kondisi pemulihan terhalang.

Bridge Kotlin menggunakan **Shizuku UserService + AIDL**, bukan API private
`newProcess`. PAK mengalir melalui pipe `ParcelFileDescriptor`, tanpa base64
atau buffer sebesar PAK. Guard di aplikasi dan service privileged membatasi
mutasi ke path WuwaID pada versi numerik serta menolak traversal/symlink.

**Verifikasi:** unduhan PAK asli dan siklus install/repair/uninstall diuji pada
filesystem terisolasi. Regresi mount memeriksa resolusi berkas PAK/signature,
hash, perbaikan/penghapusan format lama, dan rollback snapshot.
Migrasi melalui tombol **Perbaiki** juga berhasil pada perangkat SM-A525F
melalui backend Android/Shizuku: descriptor tanpa ekstensi dan hash ownership
sesuai, sementara 19 hash baseline tetap sama (16 mount bawaan, `ResManifest`,
signature bahasa Inggris aktif, dan PAK rilis). Build APK debug ARM64 dan
signature APK terverifikasi. Game kemudian berhasil mencapai layar masuk/pilih
server; teks **Keluar**, **Akun**, **Pengaturan**, dan **Ketuk untuk mendarat di
Solaris-3** tampil dalam Bahasa Indonesia tanpa dialog error pada layar tersebut.
**Batas verifikasi:** belum masuk ke gameplay atau memeriksa seluruh konten terjemahan.

### Browser dan desktop

`src/ipc.ts` memilih transport sebelum memuat mock: browser memakai `mockIPC`
di `src/mock-ipc.ts`; shell Tauri memakai perintah Rust dan event `wuwa-progress`.
**Desktop Tauri tidak memasang PAK Android** dan menampilkan batas platform,
bukan status perangkat simulasi.

Di browser, pilih **Perawatan → Skenario simulasi** untuk mencoba delapan kondisi:
siap, terpasang, game tidak ditemukan, versi tidak didukung, resource belum siap,
Shizuku mati, izin belum diberikan, dan pemulihan tertunda. Semua tindakan browser
tetap simulasi; tidak ada penulisan ke game atau peluncuran game.

Custom UID menerima teks/angka maksimal 32 karakter dengan pratinjau langsung.
UID bawaan `900123456`, versi game `3.7.0`, dan konten `3.7.0-R2` adalah data
contoh mock, bukan nilai perangkat. Penerapan/pemulihan UID memerlukan patch sehat;
uninstall mereset tampilannya. Font game tetap bawaan.

State browser disimpan dengan key `wuwaidmobile.mock-state.v3`; skenario mereset
data contoh dan custom UID. Ekspor diagnostik membedakan `mock: true` dan
`mock: false`, serta backend Android atau desktop yang tidak didukung.

### Kontrak IPC

| Perintah | Argumen | Hasil |
|---|---|---|
| `wuwa_load` | — | `Dashboard` |
| `wuwa_set_scenario` | `scenario` | `Dashboard` (browser saja) |
| `wuwa_run` | `operation`, `uidText?` (UID mock saja) | `Dashboard` |
| `wuwa_open_setup` | `target`: `game` / `shizuku` | — (native) |
| `wuwa_diagnostics` | — | `Diagnostics` (`filename`, `content`) |

Operasi native: `install`, `repair`, `uninstall`, `request_permission`,
`launch_game`. `apply_uid` dan `restore_uid` hanya tersedia dalam mock.
Operasi mutasi bersamaan ditolak. Kontrak state berada di `src/model.ts`;
installer/downloader berada di `src-tauri/src/pak/`, orkestrasi di
`src-tauri/src/native.rs`, dan bridge di `src-tauri/src/android.rs`.

## Persiapan

- Node.js 24 dan npm
- Rust stable melalui rustup
- Android Studio, JDK 17, Android SDK, dan NDK untuk pengembangan Android
- Dependensi sistem Tauri untuk target desktop yang digunakan

Panduan lengkap Android ada di [`docs/android.md`](docs/android.md).

## Menjalankan

```sh
npm ci

# Browser: frontend dengan mock IPC, tanpa toolchain Rust/Android
npm run dev

# Desktop
npm run tauri dev

# Android: jalankan emulator atau sambungkan perangkat dengan USB debugging
adb devices
npm run tauri android dev
```

## Pemeriksaan dan build Android

```sh
npm run check
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
npm run tauri android build -- --debug --apk --target aarch64
```

APK tersebut adalah build **debug** untuk verifikasi, bukan paket rilis
bertanda tangan. Konfigurasi penandatanganan Android belum disiapkan.

GitHub Actions menjalankan pemeriksaan frontend/Rust dan mengunggah APK debug
sebagai artifact pada workflow CI.

Panduan kontribusi ada di [`CONTRIBUTING.md`](CONTRIBUTING.md); riwayat perubahan
ada di [`CHANGELOG.md`](CHANGELOG.md).
