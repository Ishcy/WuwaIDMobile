# WuwaIDMobile

Pendamping lokalisasi **Wuthering Waves dalam Bahasa Indonesia**, dibangun dengan
**Tauri 2** untuk desktop dan Android. Frontend menggunakan TypeScript, HTML, CSS,
dan Vite; shell native menggunakan Rust. Proyek Android berada di
`src-tauri/gen/android/`.

## Frontend dan pemasangan PAK

Frontend minimalis dengan palet tinta–lavender, tanpa meniru antarmuka APK
referensi. Tiga layar:

- **Beranda:** status game, Shizuku, resource, dan rilis; unduh/pasang ulang
  terjemahan dengan progres dan pemeriksaan kompatibilitas.
- **Kustomisasi:** UID kustom native di Android dan pratinjau mock di browser,
  dengan penerapan dan Reset. Hanya teks tampilan yang berubah, bukan UID akun.
- **Perawatan:** pemeriksaan ulang, pemulihan transaksi, ekspor diagnostik JSON,
  dan penghapusan berkas milik aplikasi dengan konfirmasi.
- Navigasi horizontal di desktop dan dock bawah pada layar Android.
- **Terang / Gelap / Sistem** tersedia di header. Default mengikuti tema sistem;
  pilihan disimpan lokal dengan key `wuwaidmobile.theme` dan diterapkan sebelum
  CSS tampil. Mode Sistem mengikuti perubahan tema perangkat secara langsung.
  Pergantian tema tidak mereset isian atau menghentikan operasi.
- Progres hanya memperbarui indikator, bukan membangun ulang navigasi atau kontrol
  tema. Tema juga diterapkan pada dialog, toast, form, dan status error.
- Copy hanya memuat status, versi, tindakan, dan peringatan. Detail teknis di
  Perawatan dapat dibuka saat diperlukan; slogan dan label dekoratif dihapus.
- Animasi 140–180 ms untuk navigasi, kontrol, tema, progres, dialog, dan toast.
  Animasi layar hanya berjalan saat pindah halaman, bukan setiap pembaruan status.
  Pengaturan `prefers-reduced-motion` menonaktifkan animasi, termasuk saat diubah
  ketika aplikasi terbuka.

Layout dan navigasi ketiga layar diverifikasi di browser pada lebar 320, 390, 768,
1024, dan 1440 px tanpa overflow horizontal. Smoke browser mencakup delapan
skenario perangkat, pemasangan/progres, penerapan UID, pembatalan/konfirmasi
penghapusan, serta persistensi dan perubahan tema sistem. Smoke APK Android
mencakup Beranda dan penerapan/Reset UID native; belum mencakup seluruh skenario
browser atau layar Perawatan.

### Android: pemasangan native

Aplikasi mengambil rilis stabil terbaru secara dinamis dari
[`TitoTFP/WuwaID/releases/latest`](https://github.com/TitoTFP/WuwaID/releases/latest).
Tag, nama aset, ukuran, dan SHA-256 bukan nilai yang dibundel ke aplikasi.
Unduhan memakai HTTPS, streaming ke cache privat aplikasi, serta pemeriksaan
ukuran dan SHA-256 sebelum digunakan. Cache yang dipakai ulang juga diverifikasi.
Pemasangan standar menyalin PAK utuh. UID kustom mengubah database teks dalam
salinan privat lalu mengemas ulang PAK; berkas sumber rilis tetap utuh.

1. Pasang **Wuthering Waves Global** (`com.kurogame.wutheringwaves.global`).
2. Buka game dan selesaikan unduhan resource, termasuk paket bahasa Inggris.
3. Pasang dan jalankan [Shizuku](https://shizuku.rikka.app/), lalu berikan izin
   ke WuwaIDMobile melalui tombol **Berikan izin Shizuku**.
4. Periksa kembali perangkat, lalu pilih **Pasang terjemahan**.

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
Pengujian gameplay untuk UID kustom dijelaskan di bawah; belum memeriksa seluruh
konten terjemahan.

### Android: UID kustom

**Kustomisasi → Terapkan** memerlukan terjemahan terpasang dan sehat. Input
teks/angka maksimal 32 unit UTF-16 (sama dengan batas input browser), tanpa
karakter kontrol atau pemisah baris. Spasi awal/akhir dipangkas; input kosong
ditolak, bukan digunakan untuk menyembunyikan UID.

Mengadaptasi algoritme
[`WuwaIDLauncher`](https://github.com/TitoTFP/WuwaIDLauncher/blob/676c4a9b3b3f14ccb570d90252dc8972dbac8d13/src-tauri/src/engine/patch_asset.rs):
backend membuka `Client/Content/Aki/ConfigDB/en/lang_multi_text.db` dalam PAK
rilis terverifikasi, lalu mengubah kolom `MultiText.Content` untuk
`Text_FriendMyUid_Text`, `Text_UserId_Text`, dan `PrefabTextItem_1341587207_Text`.
Setiap target wajib tepat satu baris. Transaksi SQLite, pembacaan ulang teks,
dan pemeriksaan integritas harus berhasil sebelum pengemasan ulang PAK v12.
Mount point, daftar entry, dan isi setiap entry hasil repack diverifikasi;
entry lain tetap identik.

SHA-256 sumber tetap dibandingkan dengan metadata GitHub. PAK turunan memiliki
SHA-256/ukuran sendiri, dan `SHA1_PAK` descriptor berasal dari PAK yang benar-benar
dipasang. Teks pilihan dan checksum sumber disimpan bersama metadata kepemilikan
dalam transaksi yang sama, sehingga rollback mengembalikan pilihan sebelumnya.
Pasang ulang/perbaikan memakai kembali pilihan UID yang terpasang.
**Reset** memasang kembali PAK rilis asli; **Hapus** menghapus pilihan beserta
berkas milik aplikasi. Pemulihan transaksi tidak memerlukan repack atau jaringan.

Aplikasi tidak membaca nomor UID akun. Android menampilkan **UID asli game**
sebagai label bawaan; nomor contoh hanya dipakai oleh mock browser.
SQLite dan pengolahan PAK berjalan di cache privat Rust; bridge Shizuku tidak
diubah dan tetap hanya menulis berkas WuwaID yang dijaga oleh aturan kepemilikan.

**Verifikasi perangkat (2026-10-03):** APK debug ARM64 dipasang pada SM-A525F
tanpa menghapus data aplikasi, menggunakan rilis `v3.7.1-id.1`.
**Terapkan** menampilkan persis `Rover Indonesia` pada HUD gameplay Android;
pilihan tetap tersimpan setelah aplikasi dibuka kembali. **Reset** mengembalikan
label dan nomor UID semula pada HUD yang sama. SHA-256 PAK yang terpasang setelah
Reset identik dengan PAK sumber rilis terverifikasi.

Seluruh **443 berkas resource resmi** pada `Resources/3.7.0` memiliki SHA-256
identik sebelum dan sesudah skenario; tidak ada berkas resmi ditambah atau hilang.
Artefak WuwaID yang dimiliki aplikasi dikecualikan dari baseline tersebut.
`npm run build` dan 20 tes Rust (`--locked --all-targets`) berhasil.
**Batas verifikasi:** panel teman/profil tidak diuji terpisah; pengujian ini tidak
memverifikasi seluruh konten terjemahan.

### Browser dan desktop

`src/ipc.ts` memilih transport sebelum memuat mock: browser memakai `mockIPC`
di `src/mock-ipc.ts`; shell Tauri memakai perintah Rust dan event `wuwa-progress`.
**Desktop Tauri tidak memasang PAK Android** dan menampilkan batas platform,
bukan status perangkat simulasi.

Di browser, pilih **Perawatan → Skenario simulasi** untuk mencoba delapan kondisi:
siap, terpasang, game tidak ditemukan, versi tidak didukung, resource belum siap,
Shizuku mati, izin belum diberikan, dan pemulihan tertunda. Semua tindakan browser
tetap simulasi; tidak ada penulisan ke game atau peluncuran game.

Custom UID menerima teks/angka maksimal 32 unit UTF-16 dengan pratinjau langsung.
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
| `wuwa_run` | `operation`, `uidText?` (`apply_uid`) | `Dashboard` |
| `wuwa_open_setup` | `target`: `game` / `shizuku` | — (native) |
| `wuwa_diagnostics` | — | `Diagnostics` (`filename`, `content`) |

Operasi native: `install`, `repair`, `uninstall`, `request_permission`,
`launch_game`, `apply_uid`, dan `restore_uid`.
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
