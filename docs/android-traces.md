# Performa storage Android

## Ringkasan

Lima trace baseline menunjukkan biaya bersama pada inspeksi/verifikasi storage serial melalui Shizuku UserService yang menjalankan shell dan banyak helper—bukan komputasi Rust atau penulisan PAK saja. Angka adalah satu sampel tiap skenario, bukan median/p95, benchmark debug-vs-release, atau bukti perbandingan umum install dan UID.

| Skenario | Storage call / helper | Window storage | Jumlah durasi call | Temuan utama |
|---|---:|---:|---:|---|
| Berikan izin | 49 / 361 | 15,461 s | 14,851 s | Probe/discovery; tanpa mutasi/hash |
| Buka setelah izin | 49 / 361 | 13,570 s | 12,936 s | Probe/discovery; tanpa mutasi/hash |
| Pasang | 411 / 3.157 | 179,198 s | 145,699 s | PAK `.new` ditulis 0,852 s; jeda tanpa storage 27,764 s |
| Hapus | 290 / 2.170 | 103,172 s | 99,832 s | Lima call `remove` hanya 1,702 s |
| UID kustom | 411 / 3.157 | 154,086 s | 145,786 s | PAK `.new` ditulis 0,913 s; bukan sampel ganti UID terpasang |

Jumlah durasi call hampir sama pada install dan UID; mutasi filesystem masing-masing hanya 6,164 s dan 6,013 s. Install memiliki jeda 27,764 s yang konsisten dengan pengambilan rilis/download/validasi cache menurut urutan kode dan aktivitas thread HTTP; trace tidak memisahkan waktu jaringan. UID memiliki jeda 2,669 s sebelum `force_stop`, konsisten dengan hash lokal dan persiapan/verifikasi PAK turunan, bukan penyebab utama window 154 s.

Prioritas optimasi: operasi filesystem langsung dalam UserService ber-UID shell, batch metadata, lalu kurangi verifikasi berulang dengan batas invalidasi yang jelas. Jangan melemahkan pemeriksaan integritas atau guard path, dan jangan memakai cache status lintas operasi. Analisis tidak mengubah aplikasi, menjalankan aksi perangkat, atau membuktikan install/UID di game. Baseline tetap trace lama; belum ada trace perangkat yang sebanding sesudah cutover WM-23.

## Dataset

Kelima berkas lokal `android_traces/` merekam perangkat yang sama:

```text
Android SDK 34 (Android 14); aarch64
Kernel 4.14.190-perf-28693687-abA525FXXSCFYF1
Fingerprint samsung/a52qnsxx/a52q:14/UP1A.231005.007/A525FXXSCFYF1:user/release-keys
```

Fingerprint menerangkan OS perangkat, bukan build APK. Jenis APK, versi, kondisi termal/cache unduhan, dan teks UID tidak diketahui.

| Trace | Ukuran byte | Durasi trace | PID app/service |
|---|---:|---:|---|
| `berikan-izin-shizuku.pftrace` | 67.650.378 | 47,639 s | 18619 / 18801 |
| `buka-setelah-izin-shizuku-diberikan.pftrace` | 54.521.381 | 23,267 s | 19562 / 19719 |
| `pasang-terjemahan.pftrace` | 310.181.861 | 187,918 s | 19562 / 19719 |
| `hapus-terjemahan.pftrace` | 175.493.453 | 116,716 s | 19562 / 19719 |
| `terapkan-uid-kustom.pftrace` | 264.890.747 | 168,401 s | 19562 / 19719 |

SHA-256 untuk identifikasi baseline:

```text
f305cd9d6021994884dbe9797f72f443f55cce15c4b001edbbf18ffb42689899  berikan-izin-shizuku.pftrace
f46d30423c9e5e69533dba1363f58bbaa6efc00185928d983a657bd45b13d91b  buka-setelah-izin-shizuku-diberikan.pftrace
035cb4f2f400db6284dd07b0f3322ae088065c9d6caee30aa8ddebd8e44c407e  pasang-terjemahan.pftrace
e0b1cd5df4f921a3b96534a0d5fa392e9fa1640728bfe77e262952721ec3bba0  hapus-terjemahan.pftrace
f8408c4ffcd7f440b7e12aec2ddd417de2dd65b4127f7e8a8b7b9536402dc980  terapkan-uid-kustom.pftrace
```

Trace mentah tidak dikomit (>800 MiB dan dapat memuat metadata perangkat/proses). Rekaman izin di sini berbeda dari trace sebelumnya di `Downloads/`; angka dihitung ulang dari dataset di atas.

## Metode dan batas

Alat **Perfetto Trace Processor v58.2-add693d8b**, RPC API 14. SQL pendamping: [`android-traces.sql`](android-traces.sql). Metode:

1. Pilih `upid` app `com.titotfp.wuwaid` dan `com.titotfp.wuwaid:wuwaid-storage`; `upid` lokal per trace meski PID sama.
2. Ambil slice `binder transaction` dari thread app dengan `destination process` PID service. Caller thread kernel bernama terpotong `wuwaid-user-ser`, dari `wuwaid-user-service-call` Kotlin.
3. Cocokkan waktu lahir shell anak langsung dengan interval call; klasifikasikan dari `process.cmdline`, bukan tebakan durasi. Semua 1.210 call cocok satu-ke-satu dengan 1.210 shell; tak ada operasi unknown.
4. Hitung seluruh keturunan UserService lewat `process.parent_upid`, termasuk helper turunan.
5. CPU helper memakai `sched.dur` seluruh trace; CPU thread app dipotong ke window/gap yang dibahas.
6. `stats` tidak memiliki nilai positif dengan severity `error` atau `data_loss`; ini bukan jaminan semua data profiling terekam.

Definisi: waktu relatif ke `trace_bounds.start_ts`, bukan tombol/app start. **Window storage** dari call pertama sampai akhir call terakhir, mencakup jeda antarcall tetapi bukan seluruh startup/dialog/idle. **Jumlah durasi call** menjumlahkan durasi caller Binder; call pada dataset serial/non-overlap, sehingga sama dengan union interval. **Occupancy** = jumlah durasi call / window. Durasi mencakup pekerjaan remote, wait, scheduling dan overhead bridge—bukan overhead Binder murni. CPU akumulatif ≠ wall time; jangan dijumlahkan. Fase yang dipetakan dari kode bertanda `[INFERENCE]`; frontend, download, repack dan respons tidak punya marker fase awal/akhir pada baseline.

## Rincian call, helper, dan CPU

Setiap sel operasi = jumlah call / jumlah durasi call (detik); pembulatan per sel dapat sedikit berbeda dari total.

| Operasi | Izin | Buka | Pasang | Hapus | UID |
|---|---:|---:|---:|---:|---:|
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

Probe, `list`, `stat`, `read_text` total **113,722 s** install, **81,411 s** uninstall, **114,085 s** UID.

| Helper process | Izin | Buka | Pasang | Hapus | UID |
|---|---:|---:|---:|---:|---:|
| `readlink` | 229 | 229 | 2.015 | 1.388 | 2.015 |
| `/system/bin/sh` | 49 | 49 | 412 | 291 | 412 |
| `printf` | 56 | 56 | 408 | 290 | 408 |
| `stat` | 0 | 0 | 118 | 69 | 118 |
| `wc` | 13 | 13 | 62 | 43 | 62 |
| `cat` | 13 | 13 | 66 | 44 | 66 |
| `sha256sum` / `sha1sum` | 0 / 0 | 0 / 0 | 54 / 11 | 26 / 6 | 54 / 11 |
| Lainnya (`ls`, `mkdir`, `cp`, `mv`, `rm`, `cmd`) | 1 | 1 | 11 | 13 | 11 |
| **Total helper (tanpa UserService)** | **361** | **361** | **3.157** | **2.170** | **3.157** |

Shell rekursif pada aksi mutasi satu lebih banyak dari shell langsung per call; helper count bukan request count.

| CPU akumulatif (s) | Izin | Buka | Pasang | Hapus | UID |
|---|---:|---:|---:|---:|---:|
| Helper `readlink` | 8,483 | 7,563 | 81,131 | 55,930 | 81,029 |
| Helper shell | 2,144 | 1,822 | 20,218 | 14,372 | 20,219 |
| Helper `printf` | 1,991 | 1,859 | 16,261 | 11,710 | 16,592 |
| Seluruh helper | 13,593 | 12,010 | 132,600 | 91,172 | 132,753 |
| `tokio-rt-worker` app dalam window storage | 0,020 | 0,016 | 3,302 | 0,108 | 2,822 |

Rust banyak menunggu bridge; optimisasi compiler Rust tidak menghilangkan CPU helper. CPU `readlink` mencakup seluruh proses (startup/runtime/syscall), bukan biaya `readlink(2)` saja.

## Temuan per skenario

### Berikan izin Shizuku

- Dialog activity `performResume` 23,969 s, `performPause` 25,047 s; selisih 1,077 s termasuk pilihan pengguna. Service muncul 25,021 s; call pertama 25,725 s, terakhir selesai 41,185 s. Dari pause ke akhir storage sekitar 16,139 s; pause bukan timestamp pasti callback izin.
- Semua 49 call probe/discovery/inspection, tanpa write/remove/force-stop/hash. `list("3.7.0/Mount")` mulai 35,096 s dan berjalan 1,508 s; 13 descriptor dibaca satu per satu (`read_text` total 4,423 s).
- Lambat setelah izin terutama konsisten dengan pembacaan status game pada `load_dashboard`, bukan dialog atau bind timeout 15 s. Call serial tetap maju; tidak terlihat satu bind macet.

### Buka setelah izin

- App resume 3,902 s; storage 6,040–19,610 s. App dan service baru muncul di trace. Izin sudah diberikan tetapi service belum tentu warm.
- Sama dengan izin: 49 call/361 helper, tanpa hash/mutasi. Listing Mount 1,264 s; 13 descriptor `read_text` total 3,425 s.
- Startup WebView terekam, tetapi storage berikutnya sendiri memerlukan window 13,570 s. Tidak ada bukti login atau game sedang berjalan.

### Pasang terjemahan

Fase di bawah dipetakan dari urutan call/script, bukan span terinstrumentasi:

| Rentang relatif | Durasi | Fase/anchor |
|---|---:|---|
| 3,813–20,827 s | 17,014 s | Probe/discover/inspect awal, 49 call |
| 20,827–48,591 s | 27,764 s | Tanpa call storage; `[INFERENCE]` latest/download/validasi cache |
| 48,591–83,239 s | 34,648 s | Recovery/preflight tambahan hingga `force_stop` |
| 83,239–162,573 s | 79,334 s | `force_stop`, staging, jurnal, commit, verifikasi, cleanup |
| 162,573–182,672 s | 20,099 s | Inspect terminal, mulai dari listing root |
| 182,672–183,010 s | 0,338 s | Probe readable/writable untuk respons dashboard |

Pada jeda 27,764 s, thread CPU `reqwest-interna` 4,106 s dan worker Tokio 2,707 s. Kode pada posisi tersebut menjalankan `fetch_release`/`release::download`. Tabel packet network kosong dan tidak ada event ftrace network: tidak bisa memisahkan DNS/TLS/transfer/hash, menentukan cache hit, atau mengklaim bandwidth.

PAK `.new` ditulis 0,852 s. Ada 252 call `stat`. Canonical PAK di-hash SHA-256 10 kali, signature 10, mount descriptor 10, owner 9; canonical PAK SHA-1 4 kali. Jumlah request hash bukan bukti aman menghapus request.

### Hapus terjemahan

| Rentang relatif | Durasi | Fase/anchor `[INFERENCE]` |
|---|---:|---|
| 5,509–27,722 s | 22,213 s | Probe/discover/inspect awal; 64 call termasuk hash milik app |
| 27,735–66,331 s | 38,596 s | Recovery, ownership, snapshot sebelum `force_stop` |
| 66,331–94,338 s | 28,007 s | Jurnal, rename target ke backup, verifikasi, penghapusan |
| 94,338–108,274 s | 13,936 s | Inspect terminal |
| 108,274–108,681 s | 0,407 s | Probe respons |

Gap awal 0,013 s; tidak ada gap antarcall >100 ms di window. Lima `remove` memakan 1,702 s. Sekitar 60,822 s telah berlalu dari awal storage sebelum `force_stop`; PAK dipindah ke `.bak` pada 70,754 s. Canonical PAK di-hash SHA-256 5 kali/SHA-1 3 kali; backup PAK SHA-256 2 kali. Recovery, ownership, snapshot dan cleanup transaksi lebih dominan daripada `rm`.

### Terapkan UID kustom

| Rentang relatif | Durasi | Fase/anchor `[INFERENCE]` |
|---|---:|---|
| 8,944–25,790 s | 16,846 s | Probe/discover/inspect awal, 49 call |
| 25,790–26,828 s | 1,038 s | Tanpa storage; latest/cache menurut urutan kode |
| 26,828–62,847 s | 36,019 s | Recovery/preflight dan persiapan payload |
| 62,847–141,780 s | 78,933 s | `force_stop` hingga cleanup transaksi selesai |
| 141,780–162,638 s | 20,858 s | Inspect terminal |
| 162,638–163,030 s | 0,392 s | Probe respons |

Gap 60,178–62,847 s adalah 2,669 s; worker Tokio memakai 2,261 s CPU. Ini konsisten dengan `hash_local_file`, `uid::prepare`, dan hash payload sebelum `force_stop`; tidak ada span yang memisahkan ekstraksi SQLite, patch, repack, sync, dan verifikasi entry. Pola 411 call/3.157 helper dan jumlah hash per target sama dengan install. Penulisan PAK 0,913 s; trace menunjukkan publish `.new` ke canonical tanpa rename canonical ke `.bak` untuk empat slot payload. Sampel konsisten dengan Apply sebelum pemasangan, bukan mengganti UID pada pemasangan yang ada; tidak ada bukti UID di game.

## Sumber, fan-out shell, dan verifikasi berulang

Pemetaan kode saat analisis: [`StorageUserService.kt`](../src-tauri/gen/android/app/src/main/java/com/titotfp/wuwaid/storage/StorageUserService.kt) (`execute`, `ROOT_CHECK`, `CHECK_PATH_FUNCTION`); [`GameStoragePlugin.kt`](../src-tauri/gen/android/app/src/main/java/com/titotfp/wuwaid/storage/GameStoragePlugin.kt) (`probeDevice`, `connectedUserService`, `runShell`); [`native.rs`](../src-tauri/src/native.rs) (`wuwa_run`, `load_dashboard`); [`installer.rs`](../src-tauri/src/pak/installer.rs) (`inspect`, `recover`, `checked_hash`, jurnal/verifikasi); [`release.rs`](../src-tauri/src/pak/release.rs) dan [`uid.rs`](../src-tauri/src/pak/uid.rs).

Setiap `execute` memulai `/system/bin/sh -c ...`. Probe meresolve root resource dan `files` game. Script path meresolve ulang dan menjalankan `readlink -f` pada setiap komponen; listing menjalankan `readlink` tiap entry; `read_text` menjalankan `wc`/`cat`; `printf` juga muncul sebagai proses. Validasi path memang perlu; biaya berasal dari banyak proses kecil per request. Reuse HTTP client, stream buffer, atau optimisasi Rust tidak menghilangkan fan-out ini.

Pola pengulangan yang diamati:

- `inspect`: lifecycle dibaca lewat `has_lifecycle_files`, lalu artifact lifecycle diperiksa lagi, berikut owner/konflik mount.
- `checked_hash` menjalankan `stat` sebelum `hash`; `verify_owned_content`, `capture_snapshots`, `fully_applied` juga men-stat.
- `install`/`uninstall` memanggil `recover` yang berakhir dengan `inspect`, walau orchestrator sudah inspect awal.
- `apply_journal`, caller dan `finalize_journal` punya pemeriksaan state/isi beririsan; operasi mengembalikan inspect terminal.
- Optimisasi `completed` pada `load_dashboard` menghindari inspect tambahan setelah transaksi sukses; dua probe respons tetap ada, tanpa discovery/inspect baru setelahnya.

| Sampel | Path `stat` berulang | Call setelah stat pertama/path | Durasi call ulang |
|---|---:|---:|---:|
| Izin | 10 | 10 | 2,440 s |
| Buka | 10 | 10 | 1,968 s |
| Pasang | 18 | 233 | 73,820 s |
| Hapus | 16 | 170 | 52,911 s |
| UID | 18 | 233 | 74,954 s |

Pada install/UID, canonical `.owner.json` di-stat 32 kali, PAK dan signature masing-masing 30, mount 26. Hitungan ini bukan penghematan yang dapat langsung dicapai: path dapat berubah saat staging, rename, recovery, atau oleh proses lain; request sama pada fase berbeda belum tentu aman di-skip.

### Jank dan startup

FrameTimeline mencakup seluruh trace, bukan FPS seluruh WebView atau hanya window operasi.

| Sampel | Row frame lengkap | `App Deadline Missed` | Row maksimum |
|---|---:|---:|---:|
| Izin | 29 | 8 | 116,313 ms |
| Buka | 95 | 7 | 116,762 ms |
| Pasang | 809 | 69 | 63,347 ms |
| Hapus | 108 | 13 | 44,552 ms |
| UID | 121 | 20 | 48,366 ms |

Banyak label `Buffer Stuffing` tidak otomatis berarti JS berat. Pada trace mutasi tidak ada slice main thread >100 ms, tetapi tidak semua interval main-thread/JS terinstrumentasi. `bindApplication` / `WebViewChromium.init`: buka 288/211 ms, izin 327/239 ms. Bukti menunjukkan frame jank dan biaya startup, bukan main-thread freeze 100–180 s; pisahkan waktu backend dari animasi/progres.

## Cutover WM-23 dan trace berikutnya

WM-23 mengubah baseline jalur metadata/hash, bukan semua operasi. Belum ada trace perangkat sesudah perubahan untuk perbandingan performa.

- `ResourceFiles.kt` berjalan hanya dalam Shizuku UserService. Probe, `list`, `stat`, `stat_many`, `read_text`, dan `hashes` memakai `File`/`android.system.Os` langsung; Binder memakai executor storage yang ada dan metadata tidak membawa input pipe. Probe readable/writable digabung. Batch stat maks. 64 path dengan hasil berurutan dan jumlah tepat. Listing/text/metadata dibatasi 256 KiB; JSON listing mempertahankan newline/quote pada nama.
- Hash satu stream regular mengembalikan size, SHA-256 dan SHA-1 opsional. `O_NOFOLLOW`/`O_NONBLOCK`, `lstat`/`fstat`, serta identitas, size, timestamp dan path pascabaca menolak file berubah/non-regular. Nanosecond timestamp API 27+; API 24–26 memakai seconds. Ini bukan lock terhadap semua writer eksternal atau snapshot atomik multi-path. Buffer/digest dipakai ulang dalam service serial; digest di-reset tiap hash.
- Root game dan containment dicek per operasi; traversal, symlink/escape, dan special file ditolak. Tidak ada cache hash/metadata lintas operasi, transaksi, dashboard, atau preflight.
- `inspect` menggabungkan/deduplicate lifecycle dan target stat dalam satu batch. Recovery yang hasil inspect-nya dibuang hanya memulihkan transaksi; recovery publik tetap mengembalikan inspect segar. Signature source/stage memverifikasi size dan kedua digest; SHA-1 mount dan SHA-256 integritas tetap. Pemeriksaan committed beririsan dikurangi, tetapi pemeriksaan state aktual tetap sebelum mutasi, sesudah rename, sebelum cleanup, dan pada inspect terminal.
- Mutasi `mkdir`, `copy`, `move_file`, `write`, `remove`, dan force-stop masih memakai shell allowlist. Jurnal, hash cleanup, rollback, recovery, serta larangan menulis file bawaan tetap. Shell belum dihapus seluruhnya; target cutover adalah metadata/hash. `Storage::hash` dan script metadata/hash lama dihapus, konsumen memakai `FileHashes`/batch stat.
- Read/hash memakai `Os.read` dan selalu `Os.close` di `finally`: constructor `FileInputStream(FileDescriptor)` meminjam FD, sehingga `use` saja membocorkan descriptor. Perbaikan diuji host dengan syscall Linux/adapter borrowed-FD, bukan Android/ART.

**Pengukuran host, bukan benchmark perangkat:** run debug `opt-level 2` memakai release `v3.7.1-id.1` (52.871.888 byte): install 0,414 s dan dua penggantian UID 0,973 s/0,976 s. Ini satu run, tidak sebanding dengan lima trace perangkat. `ResourceFiles.kt` memakai shim JVM yang memanggil syscall Linux melalui JNI; dual hash 8 MiB 0,063 s di host. Hasil tersebut tidak membuktikan Binder, FUSE/scoped storage, SELinux, atau syscall ART pada Samsung; fixture bukan bukti game Android.

### Marker debug

Perekaman dikunci native: Rust `debug_assertions`, Android `BuildConfig.DEBUG`; frontend membaca `wuwa_trace_enabled` dari native, bukan `import.meta.env.DEV`. APK debug dapat merekam walau aset Vite production; IPC frontend tidak dapat mengaktifkan recorder release.

Tag log `WuwaIDTrace`, contoh:

```text
id=23 phase=uid.prepare event=begin monoNs=3171590928 value=0
id=23 phase=uid.prepare event=end monoNs=3454543657 value=282952589
```

Label fase tetap; hanya ID, waktu, durasi, dan byte numerik—tanpa path, UID, atau payload. `value` pada event end Rust = durasi ns; cache hit/download bytes = byte; `ui.*` = `performance.now()` dalam mikrodetik. Android/Rust Android `monoNs` memakai clock sejak boot yang sama. Clock frontend ber-origin lain: selisihkan dua nilai UI, jangan kurangi dari `monoNs`. Log host Rust memakai origin `Instant` proses.

Fase: `dashboard.load`, `operation`, `dashboard.response`, `diagnostics`, `permission.wait_user`, `bind_service`, `probe`, `discover`, `inspect`, `latest`, `cache.verify`, cache hit/miss, `download`, `download.transfer`, `payload.hash`, `uid.prepare`, `staging`, `journal.apply`, `journal.finalize`, `inspect.terminal`, `ui.busy` begin/end, `ui.response` received; juga `bridge.*`, `storage.*`, `storage.mutation`. Marker bind ada pada setiap resolusi koneksi termasuk reuse; hanya call pertama/lifecycle loss benar-benar bind/wait. Permission mulai tepat sebelum dialog dan berakhir pada callback, cancel, binder death, request failure, atau plugin destroy; durasinya mencakup waktu pengguna, bukan CPU backend.

Rekam `android.log` dengan filter `WuwaIDTrace`, `linux.ftrace` (scheduling/process/atrace), dan FrameTimeline; aktifkan atrace untuk app serta `com.titotfp.wuwaid:wuwaid-storage`. Query marker tambahan (di luar SQL baseline):

```sql
SELECT ts, utid, msg FROM android_logs
WHERE tag = 'WuwaIDTrace' ORDER BY ts;
SELECT ts, dur, name FROM slice
WHERE name GLOB 'wuwaid:*' ORDER BY ts;
```

Shell count bukan proxy call metadata baru. Gunakan marker `bridge.*` untuk crossing, `storage.*` untuk kerja privileged, dan process tree/scheduling untuk helper mutasi. Fase bersarang; jangan jumlahkan span menjadi wall time.

### Reproduksi analisis baseline

Unduh shell resmi melalui [dokumentasi Trace Processor](https://perfetto.dev/docs/analysis/trace-processor); wrapper memerlukan Python 3 dan mengunduh binary native pada pemakaian pertama. Dari root repo, satu file tiap proses:

```sh
trace_processor --version
trace_processor query -f docs/android-traces.sql android_traces/pasang-terjemahan.pftrace
```

Jalankan berurutan untuk membatasi memori. SQL memilih instance app/service terakhir; trace dengan restart berulang atau beberapa operasi perlu dipisah per instance/window terlebih dahulu. SQL membuat tabel sementara di Trace Processor, bukan di repo/game; setiap `SELECT` menghasilkan tabel CSV terpisah. Pada session warm, jalankan SQL sekali karena tabel sudah ada pada run kedua. Klasifikasi script/path berlaku untuk dataset ini dan perlu ditinjau bila kontrak shell berubah. `cmdline` dapat memuat quote/newline: ekspor `hex(cmdline)` lalu decode, jangan parse CSV mentah.

Analisis SQL dijalankan pada kelima trace; call count, pencocokan shell, nol overlap, total durasi/window, operasi, helper, dan CPU cocok dengan pengukuran independen hingga toleransi 1 µs untuk output durasi dibulatkan. Semua operasi terklasifikasi.

### Pembuktian performa sesudah perubahan

Ambil beberapa sampel perangkat dengan APK/hash, Android, cache hit/miss, resource version, kondisi awal install, dan kondisi perangkat dicatat. Ulangi izin, buka setelah izin, install, uninstall, Apply saat belum terpasang, penggantian UID saat sudah terpasang, dan Reset. Bandingkan wall per fase, busy UI, crossing/helper, CPU, dan gap; pertahankan traversal/symlink, perubahan eksternal, file rusak, rollback/recovery, dan ownership. Buktikan hasil install/UID/Reset di game serta hash file bawaan sebelum/sesudah. Trace cepat saja tidak membuktikan instalasi benar; satu sampel baseline tidak mendukung persentase speedup.
