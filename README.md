# WuwaIDMobile

Pendamping lokalisasi **Wuthering Waves dalam Bahasa Indonesia**, dibangun dengan
**Tauri 2** untuk desktop dan Android. Frontend menggunakan TypeScript, HTML, CSS,
dan Vite; shell native menggunakan Rust. Proyek Android berada di
`src-tauri/gen/android/`.

## Frontend dan pemasangan PAK

Frontend minimalis dengan palet tinta–lavender, tanpa meniru antarmuka APK
referensi. Tiga layar:

- **Beranda:** status game, metode akses (Shizuku/root), resource, dan rilis; unduh/pasang ulang
  terjemahan dengan progres dan pemeriksaan kompatibilitas.
- **Kustomisasi:** UID kustom native di Android dan simulasi di browser, dengan
  penerapan dan Reset tanpa pratinjau. Hanya teks tampilan yang berubah, bukan UID akun.
- **Perawatan:** pilihan Shizuku/root, pemeriksaan ulang, pemulihan transaksi,
  ekspor diagnostik JSON, dan penghapusan berkas milik aplikasi dengan konfirmasi.
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
3. Pilih metode akses di **Perawatan**:
   - **Shizuku** (default): pasang dan jalankan [Shizuku](https://shizuku.rikka.app/),
     lalu tekan **Berikan izin Shizuku**.
   - **Root**: tekan **Gunakan Root**, lalu **Berikan izin root** dan izinkan
     WuwaIDMobile melalui pengelola root. Tidak perlu memasang atau menjalankan Shizuku.
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

Bridge Kotlin memakai **Shizuku UserService + AIDL** atau root service `su` +
Binder melalui [libsu 6.0.0](https://github.com/topjohnwu/libsu/tree/6.0.0).
Keduanya menjalankan `StorageUserService` yang sama, bukan installer terpisah
atau API private `newProcess`. PAK mengalir melalui pipe `ParcelFileDescriptor`,
tanpa base64 atau buffer sebesar PAK. Guard di aplikasi dan service privileged
membatasi mutasi ke path WuwaID pada versi numerik serta menolak traversal/symlink.

Shizuku harus menyediakan server API **12 atau lebih baru**. Permintaan izin
bersifat asinkron: binder mati atau plugin dihancurkan menolak permintaan yang
tertunda, bukan meninggalkan UI menunggu. Penolakan yang memerlukan perubahan
izin manual mengarahkan pengguna ke daftar aplikasi di Shizuku.
Setiap bind mempunyai callback sendiri; callback sesi lama tidak dapat mengubah
sesi baru. Binder mati, disconnect, gagal bind, timeout 15 detik, dan destroy
mengosongkan sesi serta melepas penunggu. Pemeriksaan symlink cache memakai
`Os.lstat` (API 21), sehingga tidak memerlukan API 26 pada aplikasi `minSdk 24`.
Provider, layanan `IBinder`, dan kode AIDL destroy mengikuti
[kontrak resmi Shizuku](https://github.com/RikkaApps/Shizuku-API#userservice).

**Audit WM-21 (2026-10-03):** permintaan izin yang menggantung saat binder mati
direproduksi gagal-sebelum/lulus-sesudah memakai sumber Kotlin bridge di JVM host.
Smoke berikutnya mencakup izin, restart, callback lama, kegagalan bind, timeout,
destroy saat menunggu bind, stream 8 MiB beserta SHA-1/SHA-256, copy/rename,
penolakan overwrite, traversal/symlink, shell metacharacters, dan batas teks/list.
Empat berkas resource fixture tetap identik. Smoke memakai adapter tipis untuk
Android/Tauri/Shizuku; proses dan pipe berjalan di host, dengan `/system/bin/sh`
serta root resource dipetakan ke shell/direktori sementara host. Ini **bukan**
verifikasi IPC binder, UID shell/root, SELinux, atau izin storage Android nyata.

Build frontend, APK debug ARM64, signature v2, serta 20 tes Rust lulus.
Manifest APK hasil build memuat provider Shizuku yang dijaga permission dan
deklarasi API dari dependensi. Dua error `NewApi` pada bridge hilang dari lint;
full lint masih gagal pada `MissingTvBanner` dan `ImpliedTouchscreenHardware`
dari deklarasi Android TV template yang tidak diubah dalam audit ini.
ADB dan mDNS tidak menemukan perangkat; perbaikan lifecycle ini belum diuji
di perangkat. Bukti perangkat pada bagian berikut berasal dari pengujian sebelumnya.

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

### Android: root langsung (WM-24)

Root hanya diminta oleh **Berikan izin root**. Membuka aplikasi, memilih metode,
refresh, dan diagnostik tidak menjalankan `su` atau memunculkan dialog root.
Pilihan metode disimpan; grant root tidak disimpan sebagai boolean permanen.
Setelah aplikasi dibuka ulang atau root service putus, tekan tombol izin lagi.
Pengelola root dapat mengingat keputusan izin sehingga dialognya tidak selalu muncul.

Shell `su` diverifikasi sebagai root, lalu libsu meluncurkan service non-daemon.
Service memeriksa UID 0 sebelum memberikan Binder. Shell permintaan ditutup
setelah bind; service dilepas ketika aplikasi ditutup. Timeout verifikasi shell
60 detik, bind 15 detik. Penolakan izin, `su` tidak tersedia, callback lama,
Binder mati, dan destroy tidak mengaktifkan akses palsu atau pindah otomatis ke
Shizuku. Pergantian metode diblokir selama operasi, tanpa mengubah file game.
Allowlist mutasi, hash, ownership, journal, rollback, dan pemulihan tetap sama.
Tidak ada perubahan SELinux, permission direktori game secara rekursif, atau daemon permanen.

Smoke host-adapted memverifikasi delapan skenario lifecycle memakai sumber
`RootSession`/`RootStorageService`: probe pasif, penolakan/retry izin, Binder mati,
callback lama, destroy, destroy saat bind, UID root, dan pelepasan worker/shell.
UI browser memverifikasi grant root tanpa Shizuku, Apply/Reset, uninstall,
pergantian metode, serta tombol terkunci saat busy. Adapter host dan mock browser
**bukan** bukti `su`, Binder, FUSE, SELinux, atau gameplay Android asli.

**Tes perangkat hanya oleh pengguna:** coba Root dengan Shizuku tidak aktif;
pastikan refresh tidak meminta izin; tolak lalu izinkan root; pasang terjemahan,
Terapkan/Reset UID, Perbaiki/Hapus; buka game; ulangi setelah aplikasi ditutup;
dan kembali ke Shizuku. Periksa hash file resmi sebelum/sesudah. Dukungan profil
tetap menggunakan path game utama `/storage/emulated/0`, seperti backend sebelumnya.

**Verifikasi lokal WM-24:** 24 tes Rust lulus, termasuk transisi grant root,
isolasi izin metode terpilih, dan penolakan ketika game/resource tidak siap.
Build frontend dan APK debug ARM64, signature v2, serta alignment 16 KiB lulus.
APK untuk pengujian pengguna:
`src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`.
SHA-256: `6aa1ee48678687f3b72c881d544a4f795d3c71d0325363a956330b6a0876a64d`.
APK ini belum dipasang atau diuji pada perangkat oleh assistant.

**Verifikasi perangkat WM-24 (2026-10-04, oleh pengguna):** APK di atas berhasil
dipasang dengan `adb install -r`; pengguna menyatakan hasil tes **aman dan bekerja
semestinya**. Ini konfirmasi kualitatif, bukan hasil tes perangkat oleh assistant.
Tidak ada trace baru, angka performa, checklist skenario terperinci, atau hash
resource resmi sebelum/sesudah yang dilaporkan.

### Android: UID kustom

**Kustomisasi → Terapkan** tidak memerlukan pemasangan terjemahan manual
terlebih dahulu. Saat belum terpasang, backend mengambil PAK rilis terverifikasi
(atau cache yang diverifikasi ulang), menyiapkan UID, lalu memasang terjemahan
beserta UID dalam satu transaksi. Saat sudah terpasang, transaksi yang sama
mengganti pilihan UID. Game, izin metode terpilih (Shizuku/root), resource, dan rilis kompatibel tetap
wajib siap; pemasangan rusak atau transaksi tertunda harus diperbaiki lewat
**Perawatan**.

Input teks/angka maksimal 32 unit UTF-16 (sama dengan batas input browser), tanpa
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
SQLite dan pengolahan PAK berjalan di cache privat Rust; bridge privileged hanya
menulis berkas WuwaID yang dijaga oleh aturan kepemilikan.

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

**Verifikasi alur dinamis WM-18–WM-20 (2026-10-03):** smoke browser desktop
dan mobile mencakup penerapan dari kondisi belum terpasang, persistensi pilihan,
Reset, dan penerapan ulang saat sudah terpasang. Pratinjau dihapus; editor satu
kolom tidak menyisakan panel kosong. Smoke disk memakai installer Rust dan PAK
v12/SQLite fixture: ketiga teks target tepat, mount menyelesaikan PAK turunan,
Reset identik byte-per-byte dengan sumber, dan empat berkas resource fixture
tetap identik pada kedua kondisi pemasangan. Build frontend, 20 tes Rust, dan
APK debug ARM64 beserta verifikasi signature berhasil. Perubahan alur dinamis
ini belum diuji ulang di game Android: ADB tidak menemukan perangkat ataupun
layanan mDNS. Bukti gameplay di atas berasal dari alur UID sebelumnya.

### Performa

Optimasi WM-22 mempertahankan pemeriksaan integritas dan transaksi:

- Respons operasi memakai status terminal yang sudah diverifikasi installer,
  bukan langsung mengulang `discover` dan `inspect`. Snapshot hanya berlaku
  untuk respons itu; refresh, diagnostik, dan preflight operasi berikutnya
  tetap membaca serta memverifikasi resource. Probe izin/perangkat tetap dibaca ulang.
- User service memakai dua worker output tetap dengan buffer baca 64 KiB per
  worker. Eksekusi proses diserialkan agar kedua reader selalu tersedia untuk
  stdout/stderr proses yang sama. Penangkapan output tetap dibatasi 256 KiB/16 KiB;
  kapasitas awal metadata diturunkan dari 8 KiB menjadi 256 byte.
- Satu klien HTTP dipakai ulang mengikuti
  [kontrak connection pool reqwest](https://docs.rs/reqwest/0.12.28/reqwest/blocking/struct.Client.html).
  Host/redirect, timeout, metadata terbaru, ukuran, dan hash tetap diperiksa.
- Profil Cargo `dev` memakai `opt-level = 2` karena APK debug juga menjalankan
  hash dan pengemasan PAK penuh. Debug symbols, assertions, overflow checks,
  dan incremental build tetap aktif; tradeoff-nya kompilasi awal lebih mahal.
  Pengaturan ini mengikuti [profil Cargo](https://doc.rust-lang.org/cargo/reference/profiles.html).
- Frontend mempertahankan pembaruan progress in-place. Database rilis memakai
  indeks `MultiText.Id`; query UID dan `PRAGMA integrity_check` tidak dikurangi.

**Pengukuran host (2026-10-03):** input PAK rilis
[`v3.7.1-id.1`](https://github.com/TitoTFP/WuwaID/releases/tag/v3.7.1-id.1),
52.871.888 byte, SHA-256 diverifikasi, dengan 326.830 baris `MultiText`.

| Jalur | Sebelum | Sesudah |
|---|---|---|
| Respons sukses native | Mengulangi penemuan/inspeksi | Memakai hasil terminal installer |
| Thread output baru untuk 120 stat setelah warm-up | 240 | 0; dua worker dipakai ulang |
| Median 200 pengambilan klien HTTP, tanpa request jaringan | 7,176 ms | < 0,001 ms |
| Instalasi PAK pada Rust debug host, satu run | 21,283 detik | 0,560 detik |
| Terapkan UID pada Rust debug host, satu run | 33,318 detik | 1,019 detik |

Pada fixture filesystem host, rangkaian `discover` + `inspect` yang sebelumnya
diulang memerlukan **47 operasi storage**, termasuk **105.743.952 byte** hash
dan sekitar **61 ms**. Penghematan waktu/IPC Android belum diukur.
Durasi batch 120 stat tetap sekitar 0,5 detik sebelum/sesudah; tidak ada klaim
peningkatan wall-time signifikan dari pooling worker pada host ini.
Profil mobile browser mencatat nol long task ≥50 ms pada pemasangan mock,
header/navigasi tetap sama, dan tidak ada overflow horizontal.

Smoke host pada PAK rilis asli membuktikan ketiga target UID tepat, Reset
identik byte-per-byte dengan sumber, perubahan PAK setelah operasi terdeteksi
pada inspeksi baru, dan empat resource resmi fixture tetap identik. Smoke
service mencakup stream 8 MiB, output melebihi batas, penolakan write resmi/
symlink, serta 32 panggilan konkuren dengan hasil tepat. Smoke HTTPS memakai
kode downloader asli: membaca latest dua kali, memakai cache terverifikasi,
menolak cache rusak, mengunduh ulang, dan memverifikasi ukuran/SHA-256.
Build frontend, 20 tes Rust, APK debug ARM64, dan signature v2 berhasil.

**Batas:** pengukuran memakai Rust release dan debug di host x64. Perbandingan
debug menggunakan input dan smoke yang sama, dengan `opt-level` 0 → 2; bukan
waktu APK di perangkat Android. Service JVM memakai adapter Android/AIDL dan
pemetaan root/shell host, bukan Binder/SELinux nyata. Tidak ada APK dipasang
atau aplikasi/game dijalankan di perangkat untuk pekerjaan ini; tes perangkat
asli dilakukan pengguna.

### Browser dan desktop

`src/ipc.ts` memilih transport sebelum memuat mock: browser memakai `mockIPC`
di `src/mock-ipc.ts`; shell Tauri memakai perintah Rust dan event `wuwa-progress`.
**Desktop Tauri tidak memasang PAK Android** dan menampilkan batas platform,
bukan status perangkat simulasi.

Di browser, pilih **Perawatan → Skenario simulasi** untuk mencoba sepuluh kondisi:
siap, terpasang, game tidak ditemukan, versi tidak didukung, resource belum siap,
Shizuku mati, izin Shizuku belum diberikan, root menunggu izin, root siap tanpa
Shizuku, dan pemulihan tertunda. Semua tindakan browser tetap simulasi;
tidak ada penulisan ke game, permintaan root nyata, atau peluncuran game.

Custom UID menerima teks/angka maksimal 32 unit UTF-16 tanpa pratinjau.
UID bawaan `900123456`, versi game `3.7.0`, dan konten `3.7.0-R2` adalah data
contoh mock, bukan nilai perangkat. Terapkan dari kondisi belum terpasang juga
menandai patch terpasang dan sehat, sesuai alur native. Hambatan perangkat dan
pemulihan tetap memblokir tindakan; uninstall mereset tampilannya. Font game tetap bawaan.

State browser disimpan dengan key `wuwaidmobile.mock-state.v3`; skenario mereset
data contoh dan custom UID. Ekspor diagnostik membedakan `mock: true` dan
`mock: false`, serta backend Android atau desktop yang tidak didukung.

### Kontrak IPC

| Perintah | Argumen | Hasil |
|---|---|---|
| `wuwa_load` | `requestId` | `Dashboard` |
| `wuwa_set_scenario` | `scenario` | `Dashboard` (browser saja) |
| `wuwa_run` | `requestId`, `operation`, `uidText?` (`apply_uid`) | `Dashboard` |
| `wuwa_open_setup` | `target`: `game` / `shizuku` | — (native) |
| `wuwa_diagnostics` | `requestId` | `Diagnostics` (`filename`, `content`) |
| `wuwa_trace_enabled` | — | Native debug recording enabled (`boolean`) |
| `wuwa_ui_trace` | `requestId`, `event`, `frontendUs` | —; fixed events, recording disabled by native release builds |

Operasi native: `install`, `repair`, `uninstall`, `request_permission`,
`request_root`, `use_root`, `use_shizuku`, `launch_game`, `apply_uid`, dan `restore_uid`.
Dashboard memisahkan status Shizuku asli (`shizuku`) dari metode terpilih
(`access.method`: `shizuku`/`root`, `access.granted`). Kesiapan pemasangan,
UID, repair, dan uninstall memakai grant metode terpilih, bukan wajib Shizuku.
Diagnostik mencatat `android-root` atau `android-shizuku`, sesuai pilihan.
Operasi mutasi bersamaan ditolak. Kontrak state berada di `src/model.ts`;
installer/downloader berada di `src-tauri/src/pak/`, orkestrasi di
`src-tauri/src/native.rs`, dan bridge di `src-tauri/src/android.rs`.

`src/ipc.ts` membuat correlation ID per aksi UI; caller tidak perlu membuat ID
sendiri. Metadata dan digest dibaca langsung dalam service privileged terpilih,
tanpa shell/helper; mutasi tetap memakai guard dan jurnal yang sama. Panduan
marker, bukti smoke lokal, dan batas verifikasi WM-23 ada di
[`docs/android-traces.md`](docs/android-traces.md#implementasi-wm-23).

## Persiapan

- Node.js 24 dan npm
- Rust stable melalui rustup
- Android Studio, JDK 17, Android SDK, dan NDK untuk pengembangan Android
- Dependensi sistem Tauri untuk target desktop yang digunakan

Panduan lengkap Android ada di [`docs/android.md`](docs/android.md).
Analisis lima rekaman performa perangkat beserta SQL reproduksinya ada di
[`docs/android-traces.md`](docs/android-traces.md).

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
