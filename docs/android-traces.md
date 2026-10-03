# Analisis performa trace Android

## Kesimpulan

**Bottleneck bersama adalah inspeksi/verifikasi storage serial melalui Shizuku
UserService yang menjalankan shell dan banyak helper, bukan komputasi Rust atau
penulisan PAK semata.** Temuan ini berlaku untuk kelima rekaman di
`android_traces/` (dataset lokal), bukan hanya pemberian izin.

- Pemberian izin dan pembukaan setelah izin masing-masing menjalankan **49 call
  storage / 361 proses helper**, tanpa mutasi atau hash. Window storage memakan
  **15,461 s** dan **13,570 s**.
- Pemasangan dan UID masing-masing menjalankan **411 call / 3.157 helper**.
  Jumlah durasi call hampir identik: **145,699 s** dan **145,786 s**.
- Penghapusan menjalankan **290 call / 2.170 helper**, dengan **99,832 s** di
  dalam call storage. Bukan operasi `rm` saja yang membuatnya lama.
- Penulisan PAK `.new` hanya **0,852 s** pada pemasangan dan **0,913 s** pada UID.
  Semua mutasi filesystem digabung memakan **6,164 s** dan **6,013 s**.
- Pemasangan juga memiliki jeda **27,764 s** tanpa call storage. Berdasarkan
  urutan kode dan aktivitas thread HTTP, ini konsisten dengan pengambilan rilis /
  unduhan / validasi cache, tetapi trace tidak memberi durasi jaringan terpisah.
- UID memiliki jeda **2,669 s** sebelum `force_stop`, konsisten dengan hash lokal
  dan pembuatan/verifikasi PAK turunan. Ini bukan penyebab utama window 154 s.

Prioritas: operasi filesystem langsung di **UserService ber-UID shell**, lalu
batch metadata dan evaluasi verifikasi berulang dengan batas invalidasi yang
jelas. Jangan mengganti pemeriksaan integritas dengan cache status lintas operasi
atau menghapus guard path demi mempercepat hasil.

Analisis ini **tidak mengubah implementasi aplikasi**, tidak menjalankan ulang
aksi pada perangkat, dan tidak membuktikan keberhasilan pemasangan/UID di game.

## Dataset dan lingkungan

Lima skenario mengikuti nama berkas yang diberikan pengguna. Identitas perangkat
berasal dari metadata kelima trace dan semuanya sama:

```text
Android SDK: 34 (Android 14)
Arsitektur: aarch64
Kernel: 4.14.190-perf-28693687-abA525FXXSCFYF1
Fingerprint: samsung/a52qnsxx/a52q:14/UP1A.231005.007/A525FXXSCFYF1:user/release-keys
```

Ini adalah keluarga Samsung A52 / SM-A525F. Fingerprint `user/release-keys`
menjelaskan **OS perangkat**, bukan jenis build APK WuwaIDMobile. Debug/release,
versi APK, kondisi termal, keadaan cache unduhan, dan teks UID yang dipakai tidak
teridentifikasi oleh metadata ini.

| Berkas di `android_traces/` | Ukuran, byte | Durasi rekaman, s | PID app / service |
|---|---:|---:|---|
| `berikan-izin-shizuku.pftrace` | 67.650.378 | 47,639 | 18619 / 18801 |
| `buka-setelah-izin-shizuku-diberikan.pftrace` | 54.521.381 | 23,267 | 19562 / 19719 |
| `pasang-terjemahan.pftrace` | 310.181.861 | 187,918 | 19562 / 19719 |
| `hapus-terjemahan.pftrace` | 175.493.453 | 116,716 | 19562 / 19719 |
| `terapkan-uid-kustom.pftrace` | 264.890.747 | 168,401 | 19562 / 19719 |

SHA-256 sumber, untuk mengidentifikasi baseline tanpa bergantung pada nama:

```text
f305cd9d6021994884dbe9797f72f443f55cce15c4b001edbbf18ffb42689899  berikan-izin-shizuku.pftrace
f46d30423c9e5e69533dba1363f58bbaa6efc00185928d983a657bd45b13d91b  buka-setelah-izin-shizuku-diberikan.pftrace
035cb4f2f400db6284dd07b0f3322ae088065c9d6caee30aa8ddebd8e44c407e  pasang-terjemahan.pftrace
e0b1cd5df4f921a3b96534a0d5fa392e9fa1640728bfe77e262952721ec3bba0  hapus-terjemahan.pftrace
f8408c4ffcd7f440b7e12aec2ddd417de2dd65b4127f7e8a8b7b9536402dc980  terapkan-uid-kustom.pftrace
```

Berkas trace hanya dibaca. Rekaman izin ini berbeda dari trace sebelumnya di
`Downloads/`; angka di sini dihitung ulang dari dataset di atas.
Trace mentah tidak masuk commit: ukurannya >800 MiB dan dapat berisi metadata
perangkat/proses. Untuk reproduksi, sediakan lima berkas lokal dengan hash di atas.

## Metode dan batas pengukuran

Alat: **Perfetto Trace Processor v58.2-add693d8b**, RPC API 14. SQL pendamping:
[`android-traces.sql`](android-traces.sql).

1. Tentukan `upid` app `com.titotfp.wuwaid` dan service
   `com.titotfp.wuwaid:wuwaid-storage`. `upid` bersifat lokal per trace; PID yang
   sama pada rekaman berbeda tidak berarti `upid` sama.
2. Ambil slice caller `binder transaction` dari thread app dengan
   `destination process` = PID service. Thread caller bernama `wuwaid-user-ser`
   (nama kernel terpotong), berasal dari `wuwaid-user-service-call` di Kotlin.
3. Cocokkan waktu lahir shell anak langsung service dengan interval call.
   Klasifikasikan operasi dari script di `process.cmdline`, bukan dari tebakan
   durasi. Semua **1.210 call** cocok satu-ke-satu dengan **1.210 shell langsung**;
   tidak ada operasi yang tidak terklasifikasi.
4. Telusuri keturunan service melalui `process.parent_upid` untuk menghitung
   helper, termasuk helper yang diluncurkan oleh shell/helper lain.
5. Hitung CPU dari `sched.dur`. CPU helper pada tabel di bawah adalah akumulasi
   seluruh trace. CPU thread app dipotong ke window storage atau jeda yang
   sedang dianalisis.
6. Periksa `stats`: kelima trace tidak memiliki entri bernilai positif dengan
   severity `error` atau `data_loss`. Ini bukan jaminan bahwa semua kategori
   profiling yang diinginkan ikut direkam.

**Definisi:**

- Semua timestamp relatif terhadap `trace_bounds.start_ts`, bukan sejak tombol
  ditekan atau sejak aplikasi dibuka.
- **Window storage** = awal call pertama sampai akhir call terakhir. Mencakup
  jeda antarcall; tidak mencakup seluruh startup, dialog, atau idle di rekaman.
- **Jumlah durasi call** = penjumlahan durasi caller Binder. Semua call storage
  pada lima trace ini serial dan tidak overlap, sehingga jumlahnya juga sama
  dengan union interval call.
- **Occupancy** = jumlah durasi call / window storage.
- Durasi call mencakup pekerjaan remote, penantian, scheduling, dan overhead
  bridge; bukan pengukuran overhead Binder murni.
- CPU akumulatif dan wall time adalah ukuran berbeda; **jangan dijumlahkan**.
- Nama fase yang dipetakan dari urutan kode diberi `[INFERENCE]`. Belum ada
  penanda eksplisit awal/akhir operasi frontend, download, repack, dan respons.

## Perbandingan utama

| Skenario | Call | Awal → akhir storage, s | Window, s | Jumlah durasi call, s | Occupancy | Helper |
|---|---:|---|---:|---:|---:|---:|
| Berikan izin | 49 | 25,725 → 41,185 | 15,461 | 14,851 | 96,06% | 361 |
| Buka setelah izin | 49 | 6,040 → 19,610 | 13,570 | 12,936 | 95,32% | 361 |
| Pasang terjemahan | 411 | 3,813 → 183,010 | 179,198 | 145,699 | 81,31% | 3.157 |
| Hapus terjemahan | 290 | 5,509 → 108,681 | 103,172 | 99,832 | 96,76% | 2.170 |
| Terapkan UID | 411 | 8,944 → 163,030 | 154,086 | 145,786 | 94,61% | 3.157 |

Ini **satu sampel per skenario**, bukan median/p95, benchmark debug-versus-release,
atau bukti bahwa UID secara umum lebih cepat daripada pemasangan.

### Jenis call

Setiap sel adalah **jumlah call / jumlah durasi call dalam detik**. Pembulatan
per sel dapat membuat jumlah tabel sedikit berbeda dari angka total.

| Operasi | Izin | Buka | Pasang | Hapus | UID |
|---|---|---|---|---|---|
| `root_readable` | 1 / 0,143 | 1 / 0,182 | 2 / 0,347 | 2 / 0,383 | 2 / 0,426 |
| `root_writable` | 1 / 0,111 | 1 / 0,126 | 2 / 0,283 | 2 / 0,301 | 2 / 0,296 |
| `list` | 6 / 3,313 | 6 / 3,131 | 15 / 11,367 | 11 / 7,728 | 15 / 10,411 |
| `stat` | 28 / 6,861 | 28 / 6,072 | 252 / 79,151 | 188 / 57,631 | 252 / 80,146 |
| `read_text` | 13 / 4,423 | 13 / 3,425 | 62 / 22,574 | 43 / 15,370 | 62 / 22,806 |
| `hash` | 0 | 0 | 65 / 25,651 | 32 / 13,185 | 65 / 25,537 |
| `force_stop` | 0 | 0 | 1 / 0,162 | 1 / 0,157 | 1 / 0,152 |
| `mkdir` | 0 | 0 | 1 / 0,303 | 0 | 1 / 0,276 |
| `write` | 0 | 0 | 4 / 2,246 | 1 / 0,444 | 4 / 2,093 |
| `copy` | 0 | 0 | 1 / 0,662 | 0 | 1 / 0,629 |
| `move` | 0 | 0 | 5 / 2,604 | 5 / 2,933 | 5 / 2,674 |
| `remove` | 0 | 0 | 1 / 0,350 | 5 / 1,702 | 1 / 0,340 |

Probe, list, stat, dan read_text saja menghabiskan **113,722 s** pada pemasangan,
**81,411 s** pada penghapusan, dan **114,085 s** pada UID. Ini target bersama
sebelum mengoptimalkan transfer besar atau repack.

### Ledakan proses dan CPU

| Nama proses helper | Izin | Buka | Pasang | Hapus | UID |
|---|---:|---:|---:|---:|---:|
| `readlink` | 229 | 229 | 2.015 | 1.388 | 2.015 |
| `/system/bin/sh` | 49 | 49 | 412 | 291 | 412 |
| `printf` | 56 | 56 | 408 | 290 | 408 |
| `stat` | 0 | 0 | 118 | 69 | 118 |
| `wc` | 13 | 13 | 62 | 43 | 62 |
| `cat` | 13 | 13 | 66 | 44 | 66 |
| `sha256sum` / `sha1sum` | 0 / 0 | 0 / 0 | 54 / 11 | 26 / 6 | 54 / 11 |
| Helper lainnya | 1 | 1 | 11 | 13 | 11 |
| **Total, tanpa UserService** | **361** | **361** | **3.157** | **2.170** | **3.157** |

Jumlah shell rekursif pada aksi mutasi satu lebih banyak dari shell langsung
per call; total helper tidak boleh disamakan dengan jumlah request. Kolom
"lainnya" mencakup `ls`, `mkdir`, `cp`, `mv`, `rm`, dan `cmd` sesuai operasi.

| CPU akumulatif, s | Izin | Buka | Pasang | Hapus | UID |
|---|---:|---:|---:|---:|---:|
| Helper `readlink` | 8,483 | 7,563 | 81,131 | 55,930 | 81,029 |
| Helper shell | 2,144 | 1,822 | 20,218 | 14,372 | 20,219 |
| Helper `printf` | 1,991 | 1,859 | 16,261 | 11,710 | 16,592 |
| Seluruh helper | 13,593 | 12,010 | 132,600 | 91,172 | 132,753 |
| Semua `tokio-rt-worker` app dalam window storage | 0,020 | 0,016 | 3,302 | 0,108 | 2,822 |

Rust banyak menunggu bridge; menaikkan optimisasi compiler Rust tidak menghilangkan
CPU helper ini. CPU `readlink` mencakup seluruh eksekusi proses tersebut, termasuk
startup/runtime dan syscall; trace ini tidak mengisolasi biaya `readlink(2)` saja.

## Temuan per skenario

### 1. Berikan izin Shizuku

- Activity dialog izin `performResume` pada **23,969 s**, `performPause` pada
  **25,047 s**: selisih **1,077 s**, termasuk waktu pengguna memilih.
- Service muncul pada **25,021 s**. Call storage pertama pada **25,725 s**;
  terakhir selesai **41,185 s**. Dari pause dialog ke akhir storage sekitar
  **16,139 s**. Pause activity bukan timestamp pasti callback izin diterima.
- Semua 49 call adalah probe/discovery/inspection. Tidak ada write, remove,
  force-stop, atau hash dalam rekaman ini.
- `list("3.7.0/Mount")` mulai **35,096 s**, berlangsung **1,508 s**, kemudian
  13 descriptor mount dibaca satu per satu. Total `read_text` **4,423 s**.

**Diagnosis:** lambat setelah memilih izin terutama adalah pembacaan status game
pada `load_dashboard`, bukan dialog Shizuku atau timeout bind 15 detik. Trace
menunjukkan kemajuan call serial selama penantian, bukan satu bind yang macet.

### 2. Buka setelah izin diberikan

- Activity app resume pada **3,902 s**; storage berjalan **6,040–19,610 s**.
- Proses app dan service baru muncul dalam rekaman ini: izin sudah diberikan,
  tetapi service belum tentu sudah warm. Jangan menyamakan "izin tersedia"
  dengan "tidak perlu startup/bind".
- Pola operasi identik dengan izin: **49 call, 361 helper**, tanpa hash/mutasi.
- Listing Mount **1,264 s**, lalu 13 pembacaan descriptor **3,425 s**.

**Diagnosis:** pembukaan tetap menunggu pemeriksaan resource/status lengkap.
Startup WebView ada, tetapi pekerjaan storage berikutnya sendiri membutuhkan
window **13,570 s**. Tidak ada bukti bahwa login atau game sedang dijalankan.

### 3. Pasang terjemahan

Anchor di bawah berasal dari awal/akhir call dan urutan script. Nama fungsi/fase
adalah `[INFERENCE]` dari kode saat analisis, bukan span yang diinstrumentasi.

| Rentang relatif, s | Durasi, s | Anchor / pemetaan |
|---|---:|---|
| 3,813–20,827 | 17,014 | Probe/discover/inspect awal; 49 call |
| 20,827–48,591 | 27,764 | Tanpa call storage; `[INFERENCE]` latest/download/validasi cache |
| 48,591–83,239 | 34,648 | Recovery/preflight tambahan sampai awal force-stop |
| 83,239–162,573 | 79,334 | Force-stop, staging, jurnal, commit, verifikasi, cleanup |
| 162,573–182,672 | 20,099 | `[INFERENCE]` inspect terminal, dimulai dari listing root |
| 182,672–183,010 | 0,338 | Probe readable/writable untuk respons dashboard |

Pada jeda 27,764 s, CPU thread `reqwest-interna` **4,106 s** dan worker Tokio
**2,707 s**. Kode menjalankan `fetch_release` dan `release::download` pada posisi
ini. Tabel packet network kosong dan tidak ada event ftrace network; **tidak
bisa membagi jeda menjadi DNS/TLS/transfer/hash, menentukan cache hit, atau
mengklaim bandwidth unduhan**.

PAK `.new` ditulis dalam **0,852 s**. Call terbanyak adalah `stat` (**252**).
Canonical PAK di-hash SHA-256 **10 kali**, signature **10 kali**, mount descriptor
**10 kali**, owner **9 kali**. Hash canonical PAK SHA-1 juga **4 kali**. Ini
pengukuran request hash, bukan bukti setiap request aman dihapus.

### 4. Hapus terjemahan

| Rentang relatif, s | Durasi, s | Anchor / pemetaan `[INFERENCE]` |
|---|---:|---|
| 5,509–27,722 | 22,213 | Probe/discover/inspect awal; 64 call, termasuk hash milik app |
| 27,735–66,331 | 38,596 | Recovery, pemeriksaan kepemilikan, snapshot sebelum force-stop |
| 66,331–94,338 | 28,007 | Jurnal, rename target menjadi backup, verifikasi, penghapusan |
| 94,338–108,274 | 13,936 | Inspect terminal |
| 108,274–108,681 | 0,407 | Probe respons |

Ada gap kecil **0,013 s** antara dua rentang awal. Tidak ada gap antarcall >100 ms
pada seluruh window, sehingga penantian terutama mengikuti storage serial.

Lima call `remove` hanya **1,702 s** gabungan. Sebelum `force_stop` pertama,
sekitar **60,822 s** sudah berlalu sejak awal storage. PAK dipindah ke `.bak`
pada **70,754 s**, bukan langsung dihapus tanpa ownership/jurnal. PAK canonical
SHA-256 dibaca **5 kali**, SHA-1 **3 kali**; backup PAK SHA-256 **2 kali**.

**Diagnosis:** validasi kepemilikan, recovery, snapshot, dan cleanup transaksi
mendominasi. Mengganti lima `rm` tidak menyelesaikan 103 detik penantian.

### 5. Terapkan UID kustom

| Rentang relatif, s | Durasi, s | Anchor / pemetaan `[INFERENCE]` |
|---|---:|---|
| 8,944–25,790 | 16,846 | Probe/discover/inspect awal; 49 call |
| 25,790–26,828 | 1,038 | Tanpa storage; latest/cache menurut urutan kode |
| 26,828–62,847 | 36,019 | Recovery/preflight tambahan dan persiapan payload |
| 62,847–141,780 | 78,933 | Force-stop sampai selesai cleanup transaksi |
| 141,780–162,638 | 20,858 | Inspect terminal |
| 162,638–163,030 | 0,392 | Probe respons |

Di dalam rentang persiapan, gap **60,178–62,847 s** berlangsung **2,669 s**,
dengan worker Tokio memakai **2,261 s CPU**. `[INFERENCE]` Ini konsisten dengan
`hash_local_file`, `uid::prepare`, lalu hash payload turunan sebelum force-stop.
Tidak ada span yang memisahkan ekstraksi SQLite, patch database, repack, sync,
dan verifikasi entry; jangan menyebut seluruh 2,669 s sebagai waktu patch SQLite.

Pola **411 call / 3.157 helper** sama dengan pemasangan biasa, termasuk jumlah
hash per target. Penulisan PAK **0,913 s**, bukan 154 detik. Trace memperlihatkan
publish `.new` ke canonical tanpa rename canonical ke `.bak` untuk keempat
slot payload. `[INFERENCE]` Sampel ini konsisten dengan memasang UID saat belum
ada pemasangan, **bukan benchmark mengganti UID pada pemasangan yang sudah ada**.
Trace tidak memuat bukti tampilan UID dalam game.

## Mengapa inspeksi kecil menjadi mahal

Sumber yang dibaca untuk memetakan hasil:

- [`StorageUserService.kt`](../src-tauri/gen/android/app/src/main/java/com/titotfp/wuwaid/storage/StorageUserService.kt):
  `execute`, `ROOT_CHECK`, `CHECK_PATH_FUNCTION`, dan script operasi.
- [`GameStoragePlugin.kt`](../src-tauri/gen/android/app/src/main/java/com/titotfp/wuwaid/storage/GameStoragePlugin.kt):
  `probeDevice`, `connectedUserService`, dan `runShell`.
- [`native.rs`](../src-tauri/src/native.rs): `wuwa_run`, `load_dashboard`.
- [`installer.rs`](../src-tauri/src/pak/installer.rs): `inspect`, `recover`,
  `checked_hash`, `apply_journal`, `verify_committed_install`, `finalize_journal`.
- [`release.rs`](../src-tauri/src/pak/release.rs) dan
  [`uid.rs`](../src-tauri/src/pak/uid.rs): unduhan/cache dan PAK turunan.

### Fan-out shell

Setiap `execute` memulai `/system/bin/sh -c ...`. Script probe meresolve root
resource dan root `files` game. Script path meresolve root tersebut lagi lalu
memeriksa setiap komponen yang ada dengan `readlink -f`. Listing memanggil
`readlink` untuk **setiap entry**, lalu mencetak respons per entry. `read_text`
memanggil `wc` dan `cat`. Di perangkat ini proses helper `printf` juga muncul,
bukan hanya operasi builtin tanpa proses baru.

Keamanan path tersebut dibutuhkan; yang mahal adalah implementasinya melalui
banyak proses kecil untuk setiap request. Jumlah dan CPU helper terukur
menjelaskan mengapa reuse HTTP client, buffer stream, dan optimisasi Rust tidak
menyelesaikan bottleneck bersama.

### Verifikasi berulang

- `inspect` membaca lifecycle lewat `has_lifecycle_files`, kemudian memeriksa
  artifact lifecycle lagi, lalu owner dan konflik mount.
- `checked_hash` memanggil `stat` sebelum request `hash`. Caller seperti
  `verify_owned_content`, `capture_snapshots`, dan `fully_applied` juga melakukan
  `stat`, sehingga satu pemeriksaan dapat menyeberangi bridge beberapa kali.
- `install`/`uninstall` memanggil `recover`, yang berakhir dengan `inspect`,
  meskipun orchestrator sudah melakukan inspect awal.
- `apply_journal`, pemanggilnya, dan `finalize_journal` menjalankan pemeriksaan
  keadaan/isi yang beririsan, kemudian operasi mengembalikan `inspect` terminal.
- Optimisasi `completed` pada `load_dashboard` memang menghindari inspect
  tambahan setelah transaksi sukses; dua probe respons terakhir masih ada.
  Trace tidak menunjukkan discovery/inspect baru setelah probe respons itu.

| Sampel | Path `stat` yang berulang | Call setelah stat pertama per path | Jumlah durasi call ulang, s |
|---|---:|---:|---:|
| Izin | 10 | 10 | 2,440 |
| Buka | 10 | 10 | 1,968 |
| Pasang | 18 | 233 | 73,820 |
| Hapus | 16 | 170 | 52,911 |
| UID | 18 | 233 | 74,954 |

Contoh pada pemasangan/UID: canonical `.owner.json` di-stat **32 kali**, PAK dan
signature masing-masing **30 kali**, mount **26 kali**. Angka di atas **bukan
estimasi penghematan yang dapat langsung dicapai**: path dapat berubah selama
staging, rename, recovery, atau karena proses lain. Request sama pada fase
berbeda bukan otomatis duplikat yang aman dilewati.

## UI: lambat selesai berbeda dari main-thread freeze

FrameTimeline Android ikut direkam. Berikut jumlah row frame app seluruh trace
(bukan sampling FPS seluruh WebView, bukan hanya window operasi):

| Sampel | Row frame lengkap | Mengandung `App Deadline Missed` | Durasi row maksimum, ms |
|---|---:|---:|---:|
| Izin | 29 | 8 | 116,313 |
| Buka | 95 | 7 | 116,762 |
| Pasang | 809 | 69 | 63,347 |
| Hapus | 108 | 13 | 44,552 |
| UID | 121 | 20 | 48,366 |

Label `Buffer Stuffing` juga banyak muncul; jangan menyamakan seluruh label
tersebut dengan pekerjaan JS berat. Pada rekaman mutasi, tidak ditemukan slice
main thread app >100 ms, tetapi ini tidak membuktikan semua interval main thread
atau JS terinstrumentasi. Pada pembukaan, `bindApplication` sekitar **288 ms**
dan `WebViewChromium.init` **211 ms**; pada izin, masing-masing **327 ms** dan
**239 ms** saat startup.

**Kesimpulan terbatas:** ada jank pada frame yang tercatat dan ada biaya startup,
tetapi trace tidak mendukung klaim main thread membeku selama 100–180 detik.
Penantian penyelesaian backend harus dipisahkan dari kelancaran animasi/progres.

## Prioritas perbaikan dan pembuktian berikutnya

Tidak ada perubahan di bagian ini yang sudah diimplementasikan oleh analisis.

1. **Operasi filesystem langsung pada UserService.** Mulai dari probe, `stat`,
   `list`, dan `read_text` karena berpengaruh pada semua skenario. Eksekusi harus
   tetap di proses privileged Shizuku, bukan dipindahkan ke UID app yang dibatasi
   scoped storage. Pertahankan validasi root game, traversal, symlink/escape,
   bounded metadata, dan penolakan file bukan regular. Pemilihan API harus tetap
   sesuai minSdk, termasuk perangkat sebelum API 26.
2. **Batch hasil metadata/hash dengan guard yang sama.** Hindari kombinasi
   stat+hash+stat lewat beberapa call jika satu operasi remote terverifikasi
   bisa mengembalikan metadata/digest. SHA-1 untuk descriptor dan SHA-256 untuk
   integritas tetap dibutuhkan. Hash langsung/streaming juga dapat menghilangkan
   proses `sha*sum`; ukur lagi sesudah metadata diperbaiki.
3. **Kurangi pengulangan per fase, bukan safety boundary.** Evaluasi lifecycle
   scan, recovery tanpa mutasi, serta verifikasi committed yang beririsan.
   Invalidasikan snapshot saat write/copy/move/remove dan saat berpindah boundary
   transaksi; jangan reuse hasil lintas request dashboard/preflight atau memakai
   hasil cache untuk menyembunyikan perubahan eksternal. Keamanan terhadap
   perubahan konkuren perlu diputuskan dan diverifikasi, bukan diasumsikan.
4. **Tambahkan marker fase.** `permission.wait_user`, `permission.bind_service`,
   `probe`, `discover`, `inspect`, `release.latest`, `download` beserta cache hit
   dan byte, `uid.prepare`, `stage`, `journal.apply`, `journal.finalize`,
   `inspect.terminal`, `dashboard.response`, serta frontend busy-start/busy-end.
   Gunakan clock monotonic/correlation ID; jangan merekam isi UID atau payload.
   Marker akan membedakan dialog pengguna dari startup service dan jaringan dari
   hash/repack. Recording opsional debug tetap harus dikunci native; build Vite
   production bukan indikator bahwa APK pasti release.

Verifikasi setelah perbaikan: ulangi **kelima skenario yang sama** pada perangkat,
catat jenis APK dan kondisi cache, tambah sampel penggantian UID yang sudah
terpasang, lalu bandingkan window, call/helper count, CPU, dan gap per fase.
Pertahankan kasus traversal/symlink, perubahan file eksternal, file rusak,
rollback/recovery, serta ownership. Bukti sukses tetap berupa hasil aksi dan
berkas/game yang sesuai; trace cepat saja tidak membuktikan pemasangan benar.
Tidak ada persentase speedup yang dijanjikan dari satu sampel ini.

## Mereproduksi analisis

Unduh shell resmi sesuai
[dokumentasi Trace Processor](https://perfetto.dev/docs/analysis/trace-processor).
Wrapper memerlukan Python 3 dan mengunduh binary native pada pemakaian pertama.
Contoh dari root repo, setelah wrapper tersedia sebagai `trace_processor`:

```sh
trace_processor --version
trace_processor query -f docs/android-traces.sql android_traces/pasang-terjemahan.pftrace
```

Jalankan per berkas, sebaiknya **berurutan** untuk membatasi memori parser trace
besar. SQL memilih instance app/service terakhir; untuk trace dengan restart
berulang atau beberapa operasi, pisahkan instance/window terlebih dahulu. SQL
membuat tabel analisis sementara di Trace Processor, bukan berkas repo/resource.
Pada session warm, jalankan SQL sekali; nama tabel sudah ada pada pemanggilan
kedua. Klasifikasi script/path sesuai dataset ini dan harus ditinjau jika kontrak
shell berubah. `cmdline` mengandung quote/newline; untuk ekspor gunakan
`hex(cmdline)` lalu decode, bukan parsing CSV command mentah.

**Smoke analisis:** SQL pendamping dieksekusi langsung pada kelima berkas. Seluruh
call count, pencocokan shell, nol overlap, jumlah durasi call, window, agregasi
operasi, jumlah helper, dan CPU helper cocok dengan pengukuran awal independen
hingga toleransi 1 mikrodetik untuk output durasi yang dibulatkan. Semua operasi
terklasifikasi. Tidak menjalankan tes/build aplikasi karena perubahan hanya
dokumentasi dan SQL analisis.

## Implementasi WM-23

Perubahan ini mengatasi jalur storage yang ditemukan oleh lima trace di atas.
Baseline tetap **rekaman perangkat lama**; belum tersedia trace perangkat
sesudah perubahan. Pengujian perangkat/game dilakukan pengguna saja.

### Cutover dan boundary keamanan

- `ResourceFiles.kt` berjalan **hanya dalam Shizuku UserService**. Probe root,
  `list`, `stat`, `stat_many`, `read_text`, dan `hashes` memakai `File`/`android.system.Os`
  langsung, bukan proses shell. `IStorageUserService.metadata` tidak membawa
  input pipe; plugin melakukan call Binder pada executor storage yang sudah ada.
- Probe readable/writable digabung dalam satu call. Batch stat maksimal 64 path,
  hasil berurutan dengan jumlah yang harus tepat. Listing/text/metadata tetap
  dibatasi 256 KiB; JSON listing mempertahankan nama berisi newline/quote.
- Hash mengembalikan ukuran, SHA-256, dan SHA-1 opsional dari **satu stream regular
  file**. `O_NOFOLLOW`/`O_NONBLOCK`, `lstat`/`fstat`, serta pemeriksaan identitas,
  ukuran, timestamp, dan path sesudah pembacaan menolak file berubah/non-regular.
  Timestamp nanosecond dipakai mulai API 27; API 24–26 memakai field seconds.
  Ini bukan lock terhadap semua penulis eksternal atau snapshot atomik multi-path.
  Buffer dan engine digest dipakai ulang pada service serial, tetapi engine
  di-reset sebelum setiap hash. Smoke syscall tambahan memverifikasi pergantian
  isi file, SHA-256 saja, file kosong, kedua digest berulang, dan sesudah kegagalan.
- Root game dan containment diperiksa lagi per operasi; traversal, symlink
  komponen/escape, dan file khusus ditolak. Tidak ada cache hash/metadata lintas
  operasi, transaksi, dashboard, atau preflight.
- `installer::inspect` menggabungkan/deduplicate lifecycle dan target stat dalam
  satu pembacaan batch. Recovery yang hasil inspeksinya akan dibuang sekarang
  hanya memulihkan transaksi; recovery publik tetap mengembalikan inspeksi segar.
- Signature sumber dan stage memverifikasi ukuran serta kedua digest bersama.
  Verifikasi SHA-1 mount dan SHA-256 integritas tetap ada. Pemeriksaan committed
  yang beririsan dalam `apply`/`finalize` dikurangi; sebelum mutasi, sesudah rename,
  sebelum cleanup, serta inspeksi terminal tetap memeriksa state aktual.
- Mutasi `mkdir`, `copy`, `move_file`, `write`, `remove`, dan force-stop **masih
  menggunakan shell yang dibatasi allowlist**. Jurnal, hash cleanup, rollback,
  recovery, dan larangan menulis berkas bawaan tidak diganti dengan fast path.
  Shell/helper belum dihapus seluruhnya; target cutover adalah metadata/hash.
- Kontrak `Storage::hash` lama dan script metadata/hash lama dihapus; semua
  konsumen memakai `FileHashes` dan batch stat. UserService version dinaikkan ke
  2 agar sesi proses lama tidak dipakai dengan kontrak AIDL baru.

### Marker debug

Rust dikunci oleh `debug_assertions`; Android dikunci oleh `BuildConfig.DEBUG`.
Frontend membaca `wuwa_trace_enabled` dari native, bukan `import.meta.env.DEV`.
Build APK debug tetap dapat merekam meskipun aset Vite dibangun production.
APK release tidak dapat mengaktifkan recording melalui IPC frontend.

Format log dengan tag `WuwaIDTrace`:

```text
id=23 phase=uid.prepare event=begin monoNs=3171590928 value=0
id=23 phase=uid.prepare event=end monoNs=3454543657 value=282952589
```

Label tetap, hanya ID/waktu/durasi/byte numerik; tidak ada path, isi UID, atau
payload. `value` pada end Rust = durasi ns; pada `cache hit`/`download bytes` =
jumlah byte; pada `ui.*` = timestamp `performance.now()` dalam mikrodetik.
`monoNs` Android/Rust Android memakai clock sejak boot yang sama. Clock frontend
punya origin berbeda: selisihkan dua `value` UI, jangan kurangi langsung dari
`monoNs`. Log host Rust memakai origin Instant proses, bukan clock perangkat.

Fase: `dashboard.load`, `operation`, `dashboard.response`, `diagnostics`,
`permission.wait_user`, `bind_service`, `probe`, `discover`, `inspect`,
`latest`, `cache.verify`, cache hit/miss, `download`, `download.transfer`,
`payload.hash`, `uid.prepare`, `staging`, `journal.apply`, `journal.finalize`,
`inspect.terminal`, `ui.busy` begin/end, dan `ui.response` received.
Metadata juga memiliki `bridge.*` dan `storage.*`; mutasi `storage.mutation`.
Marker bind muncul pada setiap resolusi koneksi, termasuk reuse; hanya call
pertama/lifecycle loss yang benar-benar melakukan bind/wait.
Permission begin tepat sebelum request dialog; end/cancel pada callback,
binder death, kegagalan request, atau plugin destroy. Durasi ini mencakup waktu
pengguna, bukan dianggap sebagai CPU backend.

Untuk trace berikutnya aktifkan data source `android.log` dengan filter tag
`WuwaIDTrace`, `linux.ftrace` untuk scheduling/proses/atrace, dan FrameTimeline.
Aktifkan atrace untuk app dan proses `com.titotfp.wuwaid:wuwaid-storage`.
Contoh query tambahan, terpisah dari SQL baseline yang mengklasifikasi shell:

```sql
SELECT ts, utid, msg FROM android_logs
WHERE tag = 'WuwaIDTrace' ORDER BY ts;
SELECT ts, dur, name FROM slice
WHERE name GLOB 'wuwaid:*' ORDER BY ts;
```

Jangan memakai shell count sebagai proxy jumlah call metadata baru. Gunakan
pasangan marker `bridge.*` untuk crossing, `storage.*` untuk kerja privileged,
dan scheduling/process tree untuk helper yang masih berasal dari mutasi.
Fase bersarang; jangan menjumlahkan seluruh span sebagai wall time.

### Bukti lokal sesudah cutover

- 21 tes Rust lulus. Regresi permanen memeriksa perubahan PAK dengan **ukuran
  tetap** setelah inspeksi sehat: inspeksi berikutnya tidak sehat, uninstall
  menolak tanpa menghapus file yang berubah, dan repair memulihkan rilis.
  Cakupan UID juga mengganti pilihan pada pemasangan sehat, lalu repair/Reset.
- Runtime host mengunduh rilis asli `v3.7.1-id.1`, 52.871.888 byte, lalu menguji
  cache hit, install, dua penggantian UID, ketiga target SQLite, Reset ke konten
  awal/digest rilis, penolakan tamper, repair, dan uninstall. Signature/PAK
  **fixture resmi lokal** dan sumber unduhan tetap utuh; bukan bukti game Android.
  Install 0,414 s; penggantian UID 0,973 s dan 0,976 s pada host debug opt-level 2.
  Ini satu run, bukan benchmark perangkat atau speedup yang sebanding dengan
  baseline lima trace.
- `ResourceFiles.kt` dijalankan di JVM menggunakan shim Android `Os` yang
  memanggil **syscall Linux asli melalui JNI**, bukan hasil filesystem yang
  dimock. Probe/list/batch/text/dual hash cocok dengan fixture; 19 kasus
  traversal, symlink, file khusus, limit, missing/non-regular, dan perubahan
  file ditolak. Cabang API 24 dan 34 dijalankan; dual hash 8 MiB 0,063 s di host.
  Shim tidak membuktikan Binder, FUSE/scoped storage, SELinux, atau syscall ART
  pada Samsung; batas ini tidak digantikan dengan hasil tes host.
- UI browser aktual menunjukkan busy/disabled saat Apply, UID tersimpan, Reset
  mengembalikan UID awal dan idle, tanpa error browser. Browser memakai mock IPC,
  sehingga bukan bukti backend Android.
- APK debug ARM64 berhasil dibangun. Lint varian UniversalDebug masih gagal pada
  dua masalah TV manifest yang sudah ada (`MissingTvBanner`,
  `ImpliedTouchscreenHardware`); tidak ada temuan `NewApi` pada storage baru.
  APK final: `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`;
  signature v2 dan alignment 16 KiB lulus. SHA-256:
  `5e1e054cd4f026c1499b220c02b03c3c8d8542c8edba451a99f85cc8b631820e`.
- Smoke marker native menjalankan Rust dengan debug assertions on/off dan
  `StorageTrace` Kotlin dengan native BuildConfig debug/release: debug menghasilkan
  ID/fase tetap; release menjalankan kerja tanpa log atau Trace recording.

### Serah terima verifikasi perangkat

Gunakan APK debug baru agar marker tersedia; tidak ada instalasi, peluncuran,
atau operasi game/perangkat otomatis dari pengerjaan WM-23. Catat APK/hash,
cache hit/miss, resource version, kondisi awal pemasangan, dan kondisi perangkat.
Ambil beberapa sampel sebanding untuk izin, pembukaan setelah izin, install,
uninstall, Apply saat belum terpasang, penggantian UID saat sudah terpasang,
dan Reset. Bandingkan wall per fase, busy UI, crossing/helper, CPU, dan gap.
Buktikan install/UID/Reset dalam game dan bandingkan hash berkas bawaan
sebelum/sesudah. Pengguna telah melaporkan pemasangan APK berhasil pada
perangkat sendiri dan hasil uji **“aman dan cepat”**. Ini bukti kualitatif dari pengguna
untuk APK sebelum perbaikan descriptor di bawah, bukan trace baru, angka speedup,
atau checklist/hash per skenario.

### Review pre-commit: ownership descriptor Android

[AOSP FileInputStream](https://android.googlesource.com/platform/libcore/+/refs/heads/main/ojluni/src/main/java/java/io/FileInputStream.java)
menyatakan `FileInputStream(FileDescriptor)` meminjam descriptor: `close()` tidak
menutup FD milik caller. Implementasi awal WM-23 hanya mengandalkan `use`, sehingga
read/hash berulang dapat membocorkan FD, meskipun smoke JVM biasa menutupnya.
`readRegular` sekarang membaca dengan `Os.read` dan selalu `Os.close` dalam
`finally`, termasuk saat limit/validasi gagal.

Regresi permanen `ResourceFilesTest` memeriksa digest/teks dan nol FD payload
tertinggal sesudah 32 pasangan read/hash serta 32 text read yang ditolak.
Kode produksi/test dijalankan pada host dengan syscall Linux nyata dan adapter
semantik borrowed-FD Android: sebelum fix kedua tes gagal (64/32 FD tertinggal),
sesudah fix **2/2 lulus**, juga pada cabang SDK 24. Smoke tambahan membuktikan
stream 8 MiB, file kosong, pilihan digest, batas text persis, serta 11 kasus
unsafe tanpa FD tertinggal. Ini bukan eksekusi instrumentation pada Android/ART.

APK debug ARM64 sesudah fix dan suite 21 tes Rust berhasil; signature v2 dan
alignment 16 KiB lulus. APK di path/hash di atas **belum dipasang atau diuji ulang
di perangkat oleh asisten**. Runner AndroidJUnitRunner dikonfigurasi untuk regresi
instrumentation; `assembleUniversalDebugAndroidTest` juga berhasil mengompilasi
APK tes. Eksekusi instrumentation pada perangkat tetap pengguna saja.
