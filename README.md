# WuwaID Mobile

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
  penyimpanan/berbagi diagnostik JSON teredaksi, dan penghapusan berkas milik aplikasi dengan konfirmasi.
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

1. Pasang **Wuthering Waves Global** (`com.kurogame.wutheringwaves.global`) atau
   **Galaxy Store** (`com.kurogame.wutheringwaves.samsung`).
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

Pemasangan dari build Tauri lama yang mencantumkan `.pak` pada descriptor ditandai
**Perlu pemulihan**, bukan sehat. **Perbaiki pemasangan** memperbarui descriptor
dan hash kepemilikannya secara transaksional. **Hapus terjemahan** juga dapat
menghapus pemasangan lama yang terverifikasi tanpa harus memperbaikinya terlebih
dahulu; format mount yang tidak dikenal tetap ditolak.


PAK, signature, dan mount bawaan game tidak ditimpa. Target tanpa metadata
kepemilikan valid dan custom mount lain dengan prioritas 99 atau lebih tinggi
menghalangi pemasangan. Berkas disiapkan dan diverifikasi sebelum commit;
jurnal dan backup menyimpan snapshot untuk rollback/pemulihan. **Perbaiki**
memulihkan transaksi tertunda tanpa membutuhkan rilis atau koneksi jaringan,
termasuk transaksi pada versi resource lama. Pemasangan sebelum rebuild Tauri
memakai alur **Migrasikan** berikut; tanpa transaksi/migrasi tertunda, perbaikan
memasang ulang rilis latest yang kompatibel.
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

### Pengujian lintas fitur (WM-27)

Suite saat ini mencakup **108 tes host yang lulus**: 41 frontend, 33 Rust, dan
34 Kotlin. WM-27 menambahkan **47 tes host** di luar cakupan updater sebelumnya:
aturan kesiapan/akses, UID dan Perawatan, metadata/cache PAK, cleanup staging,
collision filesystem, rollback pemasangan pertama pada empat tahap commit,
serta batas mutasi/pemilihan package storage Android.

Smoke Chromium menjalankan 11 skenario Perawatan, izin root simulasi tanpa
Shizuku, UID pertama/busy/persistensi/Reset, pergantian tema, pembatalan dan
konfirmasi penghapusan, serta lebar 320/412/1024 tanpa overflow horizontal.
Smoke CLI menjalankan engine PAK sebenarnya pada fixture lokal: cache
terverifikasi, install, repair dan uninstall sambil mempertahankan berkas
resmi, berkas tak dimiliki aplikasi, dan sumber unduhan. Ini bukan bukti game.

**8 tes instrumentasi Android** (6 baru) sudah dikompilasi dan APK tes dibuat,
tetapi **belum dijalankan pada perangkat**. APK debug dan APK instrumentasi
memakai package berbeda dari release; kandidat release WM-26 tidak diganti.
[Perintah, APK/checksum dan checklist perangkat](docs/android.md#regresi-lintas-fitur-wm-27)
dicatat terpisah. Implementasi WM-27 semula diselesaikan tanpa commit atau push;
pengguna kemudian mengizinkan pre-commit review dan commit. Review menghapus
assertion `mtime` yang tidak termasuk kontrak cache. Tidak ada perubahan perilaku
produksi, push atau tes perangkat oleh asisten.

### Pembaruan aplikasi Android (WM-26)

Updater **aplikasi** memakai rilis stabil
[`TitoTFP/WuwaIDMobile`](https://github.com/TitoTFP/WuwaIDMobile/releases),
bukan repository PAK `TitoTFP/WuwaID`. Pemeriksaan otomatis dilakukan saat
aplikasi dibuka; **Perawatan → Pembaruan aplikasi → Periksa pembaruan** tersedia
untuk pemeriksaan manual. Notifikasi berupa banner di dalam aplikasi, bukan
layanan background atau notifikasi Android ketika aplikasi ditutup.

Jika ada versi lebih baru:

1. Banner menampilkan versi terpasang, versi baru dan ukuran APK.
2. Tekan **Unduh APK**; progres unduhan dan verifikasi ditampilkan.
3. Tekan **Pasang update**. Jika Android meminta izin pemasangan dari sumber
   ini, izinkan WuwaID Mobile lalu kembali; pemasang dilanjutkan setelah izin
   aktual diperiksa. Jika ditolak, APK tetap tersimpan dan dapat dicoba lagi.
4. Konfirmasikan pemasangan pada layar Android. Aplikasi tidak memasang update
   diam-diam dan tidak menghapus data atau PAK. Pembatalan pemasang tidak
   dianggap sebagai update yang berhasil.

Unduhan hanya menerima nama aset `WuwaID-Mobile-v<versi>.apk` dari repository
resmi melalui HTTPS dengan redirect host GitHub yang dibatasi. Ukuran dan
SHA-256 diverifikasi (digest aset GitHub, atau `SHA256sums.txt` pada rilis yang
sama). Sebelum membuka pemasang, cache diperiksa ulang: package harus
`com.titotfp.wuwaid`, versionName sesuai tag, versionCode lebih tinggi,
sertifikat signer sesuai signing asli dan aplikasi terpasang, minSdk didukung,
serta terdapat pustaka native ARM64. Verifikasi signature final dan pemasangan
tetap dilakukan oleh Android. URI `FileProvider` hanya mengekspos subfolder
cache `app-updates/`, dengan grant baca sementara.

APK yang sudah diverifikasi dapat dipakai kembali setelah aplikasi ditutup
atau kembali dari pengaturan izin. Unduhan terputus tidak dianggap siap;
file parsial dibersihkan ketika unduhan dicoba kembali. Cache yang gagal
verifikasi ditolak dan perlu diunduh ulang. Kegagalan jaringan updater tidak
menghalangi status atau operasi game; updater tidak meminta root/Shizuku.
Download/pemasangan update dan mutasi PAK tidak dijalankan bersamaan dari UI.
Browser, desktop, APK debug dan signing yang tidak cocok menampilkan batas
dukungan, bukan update simulasi.

Kanal hanya menerima rilis stabil, bukan draft/prerelease. Workflow rilis
menandai publikasi yang sudah disetujui QA sebagai stabil, termasuk versi
`0.x`; kandidat yang belum disetujui tetap hanya artifact QA. Bila belum ada
rilis stabil, UI menyatakan itu—bukan mengklaim sudah memakai versi terbaru.
Rilis berikutnya wajib menaikkan versionCode dari **8**, mempertahankan package
dan signing, serta menyamakan versi npm/Cargo/Tauri dan tag.

**Status WM-26:** implementasi, 37 tes perilaku host/UI dan smoke browser telah
diverifikasi. Pengguna kemudian melaporkan **“sudah kuuji aman”** dan meminta
commit. Penerimaan perangkat dicatat berdasarkan laporan tersebut, tanpa
mengklaim seluruh checklist QA telah dijalankan atau cakupan yang tidak dilaporkan.
Kandidat baru tetap `0.4.0` / versionCode **8**; memasang kandidat ini pertama kali masih
manual karena APK branding sebelumnya belum memiliki updater. Bukti kandidat
dan checklist QA dicatat di work item; kandidat tidak dipublikasikan.

**Kandidat QA updater WM-26:** APK release ARM64 berhasil dikompilasi dengan
signing asli; package `com.titotfp.wuwaid`, versi `0.4.0` / code 8, signature v2
dan alignment 16 KiB telah diperiksa. Ukuran **10.506.333 byte**.

```text
src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk
SHA-256: f5a089582f140d0eef19c5ab4c4a6a692f2351b2fa74a579441d424eefa85565
```

Pengujian yang dilakukan setelah pengguna mengizinkan tes oleh asisten:

- **27 tes Kotlin/Robolectric:** rilis/versi numerik, fallback checksum, HTTPS
  dan redirect, unduhan terputus/ukuran/digest, identitas dan signer APK,
  signature rusak meskipun checksum cocok, minSdk/ABI, cache/restore, izin,
  pembatalan, hasil pemasang dan lifecycle. APK fixture dibuat dengan SDK/NDK
  dan signing key sementara; HTTP benar-benar melewati server TLS lokal.
- **10 tes Vitest/jsdom**, dengan pemeriksaan tipe: state/busy, overlap aksi,
  progres aksesibel, kegagalan/pemulihan, batas dukungan dan escaping.
- **24 tes Rust yang sudah ada**, build frontend, serta parsing workflow berhasil.
- Smoke Chromium pada 412×915: fallback browser, banner/progres/siap pasang,
  pembatalan, penguncian/pemulihan aksi UID, serta draf/fokus/seleksi UID.
  Transport updater pada smoke ini disimulasikan, bukan instalasi Android.

Tes APK valid menemukan regresi parser arsip **Android 9/API 28**: flag
`GET_SIGNING_CERTIFICATES` saja tidak mengumpulkan sertifikat. Updater kini juga
menetapkan `GET_SIGNATURES`; tes gagal-sebelum/lulus-sesudah dipertahankan.
[Perintah, adaptasi JVM dan batas bukti](docs/android.md#tes-host-updater-wm-26)
didokumentasikan; CI dan workflow rilis menjalankan kedua suite updater.

APK di atas sudah dibangun ulang setelah perbaikan. Warning Rust/API legacy,
deprecation AGP dan configuration-cache pada `verifyUpgradeSigning` masih ada;
tes/build tidak gagal. Workflow GitHub belum dijalankan oleh asisten. Asisten
tidak menguji installer Android nyata, ADB atau game. Penerimaan updater dari
pengguna dicatat terpisah dari penerimaan branding sebelumnya.

Pre-commit review tidak mengubah perilaku kandidat yang diterima. Format bridge
baru sudah sesuai `rustfmt`; `cargo fmt --check` seluruh repository masih gagal
pada enam berkas baseline yang tidak diubah WM-26. Rincian batas verifikasi ini
dicatat pada dokumentasi Android; tidak ada reformat massal dalam commit updater.

### Upgrade dari aplikasi sebelum Tauri (WM-25)

Sumber tag dan APK publik **v0.1.0, v0.2.0, v0.3.0–v0.3.4** telah diperiksa,
bukan APK Vietnam yang dipakai sebagai referensi reverse engineering:
[rilis WuwaIDMobile](https://github.com/TitoTFP/WuwaIDMobile/releases),
[kontrak berkas v0.3.4](https://github.com/TitoTFP/WuwaIDMobile/blob/v0.3.4/app/src/main/java/com/titotfp/wuwaid/GamePaths.kt).
Ketujuh APK mempunyai package `com.titotfp.wuwaid`, versionCode 1–7 dan
sertifikat SHA-256 yang sama:

```text
2cb1b4fed5446cf52018b5d56d58c379d7328022c1ba17278c19c0002e484df9
```

Identitas rilis rebuild adalah **0.4.0 / versionCode 8**, dengan package yang
sama. **Update tanpa uninstall/clear data memerlukan keystore rilis asli.**
Build rilis menolak kredensial yang tidak lengkap, sertifikat berbeda, atau
versionCode <= 7. Versi berikutnya wajib menaikkan versionCode dari rilis terakhir,
bukan hanya mengganti versionName. APK debug memakai `com.titotfp.wuwaid.debug`
dan bukan pengganti APK publik; build debug Tauri terdahulu yang memakai package
rilis dan sertifikat debug juga bukan jalur upgrade publik yang kompatibel.

Alur pengguna:

1. Perbarui memakai APK **release bertanda tangan asli**, tanpa menghapus aplikasi
   maupun datanya. Tutup installer lama; jangan jalankan dua installer bersamaan.
2. Buka aplikasi dan pilih/berikan akses Shizuku atau Root bila diperlukan. Izin
   root tidak dianggap tersimpan hanya karena versi sebelumnya mempunyai akses.
3. Aplikasi mendeteksi `wuwaindonesia/WuWaID_99_P.pak`, signature saudara, dan
   `Mount/wuwaindonesia.txt` pada semua versi resource numerik. Membuka aplikasi
   dan refresh **tidak memindahkan/menghapus** berkas.
4. Pilih **Perawatan → Migrasikan**. Tindakan ini tidak mengunduh PAK atau
   membutuhkan metadata latest: hash SHA-256 **dan ukuran** PAK harus cocok dengan
   katalog aset historis [TitoTFP/WuwaID](https://github.com/TitoTFP/WuwaID/releases)
   yang dipin pada 2026-10-04, signature harus tidak kosong, dan descriptor harus
   persis menunjuk pasangan SHA-1 terpasang dalam format yang ditulis aplikasi lama.
5. Game dihentikan. Salinan PAK/signature, mount baru, dan ownership disiapkan di
   `wuwaid/` lalu dipublikasikan melalui jurnal transaksi yang sama. Berkas lama
   baru dihapus setelah seluruh hasil baru lolos verifikasi. Jurnal menyimpan
   bukti hash berkas lama agar cleanup terputus dapat dilanjutkan; rollback sebelum
   hasil lengkap tidak menghapus sumber lama.
6. Pemasangan lama pada resource tidak aktif diproses melalui ownership/transaksi
   yang sama lalu dihapus, bukan dipromosikan menjadi terjemahan resource aktif.
   **Hapus terjemahan** juga menerima pemasangan lama terverifikasi dan menghapus
   seluruh set yang dimiliki pada versi resource paket game terpilih.
7. Setelah migrasi, pasang latest bila versi terjemahan belum sesuai dengan game.
   UID kustom baru dapat diterapkan setelah status pemasangan sehat.

Hash tidak dikenal, berkas tidak lengkap/berubah, mount berbeda, pemasangan lama
dan baru berdampingan, konflik mod, atau sisa `.new`/`.new.tmp`/`.bak` lama
**memblokir mutasi**. Installer lama tidak menyimpan jurnal durable; aplikasi baru
tidak menebak rollback sisa transaksi tersebut. Berkas tidak dikenal di folder
lama tidak ikut dihapus. Katalog bukan allowlist untuk semua rilis masa depan:
sebelum distribusi berikutnya, perbarui hanya dari digest/ukuran aset publik yang
dapat ditelusuri, bukan hash perangkat atau cache preferensi pengguna.

Galaxy Store tidak kehilangan dukungan: pemilihan mengikuti prioritas versi lama,
Global yang resource-nya materialized terlebih dahulu, lalu Galaxy Store,
kemudian paket terpasang pertama. Pilihan dibekukan untuk sesi aplikasi agar
refresh atau pergantian Shizuku/Root tidak mengalihkan transaksi ke game lain.
Detail teknis menampilkan paket aktual; force-stop dan guard storage hanya
menerima kedua package tersebut. Penghapusan tidak menyentuh paket game lain.
Cache `patch_release` lama tidak dihapus dan tidak dipakai sebagai bukti ownership.

**Penerimaan sebelum rilis — pengujian oleh pengguna:**

- Upgrade in-place dari setiap APK publik v0.1.0–v0.3.4 dengan signing asli;
  pastikan tidak perlu uninstall/clear data dan hanya ada satu aplikasi release.
- Ulangi untuk belum pernah memasang terjemahan, pemasangan sehat, resource lama,
  serta game Global/Galaxy Store; jika keduanya ada, periksa paket yang dipilih.
- Periksa ulang tanpa internet: deteksi/migrasi/pemulihan/penghapusan lama tetap
  tersedia. Setelah migrasi, unduh latest yang kompatibel dan uji dalam game.
- Coba Shizuku diizinkan/ditolak/restart dan Root diizinkan/ditolak; tidak boleh
  ada prompt root otomatis atau fallback transport diam-diam.
- Putus proses saat staging, publikasi, dan cleanup migrasi; setelah dibuka
  ulang, **Perbaiki** harus mempertahankan sumber lama atau menyelesaikan hasil
  baru terverifikasi, tanpa dua mount aktif setelah pemulihan selesai.
- PAK rusak/tidak dikenal, descriptor berubah, file hilang, legacy `.bak/.new`,
  dan dua layout berdampingan harus ditolak tanpa mengubah berkas yang ambigu.
- Setelah migrasi: install/update, UID Apply/Reset, uninstall, buka ulang aplikasi
  dan game. Bandingkan hash PAK/signature/mount resmi sebelum/sesudah.

Pengujian upgrade dan perilaku migrasi **belum dijalankan oleh asisten**, sesuai
instruksi pengguna. Keystore rilis asli kemudian disediakan pengguna dan APK
release untuk pengujian in-place berhasil dibuat. Build debug tetap hanya untuk
pemeriksaan migrasi berkas, bukan bukti upgrade APK.
Workflow rilis memerlukan pengakuan QA manual dan memverifikasi identitas serta
sertifikat APK sebelum publikasi; lihat [`docs/android.md`](docs/android.md).

**Pemeriksaan non-tes WM-25:** frontend TypeScript/Vite, Rust Android, Kotlin/AIDL
dan APK debug ARM64 berhasil dikompilasi; `cargo check --all-targets` juga selesai
(tidak menjalankan tes). Metadata APK menunjukkan
`com.titotfp.wuwaid.debug`, versionCode 8, `0.4.0-debug`; signature v2 dan
alignment native 16 KiB valid. SHA-256 APK debug:

```text
a997596c94157bd3b43a38ee9aea3d5b4ab0d167da064575f03475eb0ee1f2a8
```

Pemeriksaan metadata/build tersebut bukan bukti migrasi berjalan, upgrade
in-place diterima Android, atau game bekerja; penerimaan perangkat dicatat terpisah.

**Kandidat release WM-25 yang diterima sebelum perubahan branding:** APK ARM64 `0.4.0` / versionCode **8** berhasil
dibangun memakai keystore asli pada 2026-10-04, dengan package
`com.titotfp.wuwaid`. Pemeriksaan APK final menunjukkan satu signer dengan
sertifikat yang sama dengan ketujuh APK publik lama; signature v2 dan alignment
native 16 KiB valid. Lokasi:

```text
src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk
```

Ukuran **10.070.581 byte**; SHA-256:

```text
eab8430cf4bcbc734b81e98406e6bdb7474b644e625dd49f410e5f05d30f6f18
```

Signing/build bukan bukti update atau migrasi berhasil pada perangkat. APK tidak
dipasang atau diuji oleh asisten.

**Penerimaan perangkat WM-25 — 2026-10-04:** pengguna memberikan output
`adb install -r` untuk kandidat release di atas dengan hasil **Success**, lalu
menyatakan pengujian sudah dilakukan dan hasilnya **aman dan berfungsi semestinya**.
WM-25 ditutup berdasarkan penerimaan pengguna tersebut. Pada penutupan itu,
asisten tidak mengubah kode runtime/APK, menjalankan tes ulang, commit, push atau
publikasi rilis. Kandidat branding berikutnya dicatat terpisah di bawah.

Konfirmasi ini bersifat kualitatif. Versi aplikasi sebelum update, daftar skenario
yang dijalankan, cakupan seluruh matriks di atas, fase crash recovery, hash berkas
resmi sebelum/sesudah dan metrik performa tidak diberikan; tidak diklaim sebagai
bukti exhaustive atau angka terukur. Checklist tetap menjadi panduan pengujian
upgrade untuk rilis berikutnya.

### Nama dan ikon aplikasi

Nama aplikasi/launcher adalah **WuwaID Mobile**, bukan `WuwaIDMobile`.
`productName`, judul window, label aplikasi Android dan label activity memakai
nama yang sama. Package tetap `com.titotfp.wuwaid`; perubahan branding tidak
mengganti identitas aplikasi, versi `0.4.0` / code 8 atau kunci signing.

Art ikon berasal dari [ikon WuwaIDLauncher](https://github.com/TitoTFP/WuwaIDLauncher/blob/676c4a9b3b3f14ccb570d90252dc8972dbac8d13/src-tauri/icons/icon.png),
dipin pada commit tersebut dan disimpan sebagai `src-tauri/app-icon.png`.
PNG/ICO/ICNS dan aset launcher Android dibuat dengan `tauri icon`, termasuk
legacy PNG semua density serta adaptive foreground/background. Drawable default
Android yang tidak lagi dipakai dihapus. Cara regenerasi ada di
[`docs/android.md`](docs/android.md).

Pengguna melaporkan kandidat branding sebelumnya terpasang dengan sukses, tetapi
ikon launcher terlalu zoom. Foreground adaptive sebelumnya memenuhi kanvas
108×108; sekarang `src-tauri/app-icon.json` memilih SVG khusus yang menempatkan
artwork 72×72 di tengahnya, dengan padding transparan 18 di setiap sisi.
Koreksi awal 66×66 sudah diuji pengguna tetapi masih menyisakan bingkai putih;
framing diperbesar sekitar 9% untuk memenuhi viewport launcher.
Art asli untuk desktop/legacy tidak diganti.

**Kandidat release perbaikan skala ikon:** build ARM64 berhasil; metadata APK
final menampilkan `WuwaID Mobile` untuk aplikasi dan launcher activity. Kelima
foreground PNG pada density mdpi–xxxhdpi cocok pixel-for-pixel dengan sumber
berpadding; untuk xxxhdpi, artwork 288×288 berada di pusat kanvas 432×432.
Preview mask bulat/kotak-membulat menunjukkan framing lebih lebar, tetapi
merupakan simulasi, bukan screenshot perangkat. Signature v2, sertifikat rilis
asli dan alignment native 16 KiB valid. Pengguna kemudian melaporkan update
perangkat berhasil dan menerima framing terbaru dengan “Nice! Oke!”.

Kandidat branding tersebut memakai lokasi output yang sama dengan kandidat
WM-25, dan kemudian digantikan kandidat updater WM-26. Bukti branding tetap
merujuk APK berukuran **10.476.933 byte** dengan SHA-256:

```text
6710ce66966220ff5fad823534407c4d33b31593bc2d601eaa764fe225659a99
```

Penerimaan perangkat WM-25 di atas berlaku untuk kandidat dengan checksum
sebelumnya, bukan bukti pengujian tampilan nama/ikon yang baru.

Penerimaan branding dicatat terpisah: APK dengan checksum branding di atas
berhasil dipasang dengan `adb install -r`, kemudian pengguna menyetujui hasilnya.
Review pre-commit tidak mengubah kode aplikasi atau APK yang telah diterima;
asisten tidak menjalankan ulang tes perangkat.

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

Di browser, pilih **Perawatan → Skenario simulasi** untuk mencoba sebelas kondisi:
siap, terpasang, game tidak ditemukan, versi tidak didukung, resource belum siap,
Shizuku mati, izin Shizuku belum diberikan, root menunggu izin, root siap tanpa
Shizuku, pemulihan tertunda, dan pemasangan sebelum Tauri. Semua tindakan browser tetap simulasi;
tidak ada penulisan ke game, permintaan root nyata, atau peluncuran game.

Custom UID menerima teks/angka maksimal 32 unit UTF-16 tanpa pratinjau.
UID bawaan `900123456`, versi game `3.7.0`, dan konten `3.7.0-R2` adalah data
contoh mock, bukan nilai perangkat. Terapkan dari kondisi belum terpasang juga
menandai patch terpasang dan sehat, sesuai alur native. Hambatan perangkat dan
pemulihan tetap memblokir tindakan; uninstall mereset tampilannya. Font game tetap bawaan.

State browser disimpan dengan key `wuwaidmobile.mock-state.v5`; skenario mereset
data contoh dan custom UID. Ekspor diagnostik membedakan `mock: true` dan
`mock: false`, serta backend Android atau desktop yang tidak didukung.

### Diagnostik dan ekspor (WM-28)

**Perawatan → Simpan diagnostik / Bagikan diagnostik** menghasilkan laporan
`schemaVersion: 1`. Bagian `application`, `device`, `probe`, `resource`, `patch`
dan `release` memakai status `ok`, `error`, atau `not_checked`, beserta `data`,
`error` dan `reason`. Bagian gagal/belum diperiksa memiliki `data: null`;
ini tidak berarti terjemahan tidak terpasang. Kegagalan probe, metadata,
discovery atau inspeksi tidak membuang bagian lain yang masih tersedia.

Android mencatat identitas/versi aplikasi, versionCode/build type, Android SDK,
versi OS, manufacturer/model dan ABI. Pemeriksaan resource/PAK dibaca ulang
jika akses tersedia; rilis PAK memakai cache sesi tanpa meminta jaringan.
Teks UID kustom tidak dimasukkan; laporan hanya mencatat `uidCustomized`
ketika pemasangan berhasil diperiksa. Pesan error juga meredaksi UID kustom
aktif dan input UID operasi yang diketahui. History menyimpan maksimal
20 kegagalan IPC/ekspor terbaru dalam sesi frontend. Pesan history disimpan
maksimal 1024 unit UTF-16 setelah redaksi; penanda redaksi saat serialisasi
dapat sedikit memperpanjang pesan ekspor. History hilang ketika frontend dimuat ulang.

**Simpan** di Android membuka pemilih dokumen sistem. Status `saved` hanya
diberikan setelah penulisan, flush dan penutupan stream berhasil; pembatalan
dan kegagalan berbeda. **Bagikan** membuka chooser dengan URI read-only untuk
snapshot JSON unik di cache privat. `share_opened` tidak membuktikan pengiriman
ke aplikasi penerima. Cache laporan milik fitur yang berumur lebih dari
24 jam dibersihkan pada aksi berbagi berikutnya.

Browser tetap diberi label `mock: true`; Simpan hanya mengonfirmasi unduhan
dimulai, bukan berkas sudah tersimpan. Bagikan memakai Web Share jika tersedia;
jika tidak, tampilkan kegagalan dan gunakan Simpan. Native desktop menghasilkan
metadata dan bagian game `not_checked`; dialog ekspor native hanya untuk Android.

Laporan tidak menyertakan logcat/trace, payload PAK atau material signing,
dan tidak diunggah otomatis. **131 tes host lulus** (49 frontend, 39 Rust,
43 Android), smoke Chromium dan harness Robolectric sudah dijalankan.
Pengguna melaporkan tes perangkat asli aman pada APK sebelum review pre-commit.
Review memperbaiki redaksi UID yang diawali penanda `[REDACTED]`; APK setelah
perbaikan diverifikasi lokal, tanpa mengulang tes perangkat.
[APK, bukti perangkat dan checklist](docs/android.md#diagnostik-dan-ekspor-wm-28).

### Kontrak IPC

| Perintah | Argumen | Hasil |
|---|---|---|
| `wuwa_load` | `requestId` | `Dashboard` |
| `wuwa_set_scenario` | `scenario` | `Dashboard` (browser saja) |
| `wuwa_run` | `requestId`, `operation`, `uidText?` (`apply_uid`) | `Dashboard` |
| `wuwa_open_setup` | `target`: `game` / `shizuku` | — (native) |
| `wuwa_diagnostics` | `requestId` | `DiagnosticReport` (`schemaVersion`, `sections`, `privacy`, `errors`) |
| `wuwa_export_diagnostics` | `action`: `save` / `share`, `content`: JSON teredaksi | `DiagnosticExportResult` (`saved` / `cancelled` / `failed` / `share_opened`; Android) |
| `wuwa_trace_enabled` | — | Native debug recording enabled (`boolean`) |
| `wuwa_ui_trace` | `requestId`, `event`, `frontendUs` | —; fixed events, recording disabled by native release builds |

Operasi native: `install`, `repair`, `uninstall`, `request_permission`,
`request_root`, `use_root`, `use_shizuku`, `launch_game`, `apply_uid`, dan `restore_uid`.
Dashboard memisahkan status Shizuku asli (`shizuku`) dari metode terpilih
(`access.method`: `shizuku`/`root`, `access.granted`). Kesiapan pemasangan,
UID, repair, dan uninstall memakai grant metode terpilih, bukan wajib Shizuku.
`game.packageName` adalah paket Global/Galaxy Store aktual. `patch.legacyPending`
menandakan berkas sebelum Tauri; `repair` memilih migrasi offline untuk kondisi
ini, atau pemulihan jurnal bila `recoveryPending` aktif. Browser juga menyediakan
skenario `legacy`; state simulasi bukan sumber migrasi data Android.
Diagnostik mencatat `android-root` atau `android-shizuku` saat probe berhasil;
`android` ketika probe gagal, dan `desktop-unsupported` di desktop. Helper
frontend menambahkan history/redaksi lalu menghasilkan `filename`/`content`.
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
