# 06 — Backend Shizuku (perintah shell yang sebenarnya)

Sumber argv: `smali_classes5/vn/wahu/vhwuwa/storage/ShizukuStorageAdapter.smali` (40 method).

## Arsitektur

```
GameStorage (interface)
  └── ShizukuStorageAdapter          ← 40 method; semua perintah shell dibangun DI SINI
        │  rootPath (default, hardcode):
        │  /storage/emulated/0/Android/data/com.kurogame.wutheringwaves.global/files/UE4Game/Client/Client/Saved/Resources
        ├── StoragePathGuard.resolvePosix(root, path)   ← setiap path dinormalkan + dijaga
        ├── raw(args: List<String>, stdin: ByteArray?) : BinaryShellResult
        ├── requireOk(args, stdin)                     ← lempar StorageException kalau exit != 0
        └── writeRaw(path, bytes)                       ← satu-satunya jalur tulis biner
              ↓
        ShizukuCommandRunner (interface) — satu method abstrak:
              run(args: List<String>, stdin: ByteArray?): BinaryShellResult
              ↓
        AndroidShizukuCommandRunner  →  Shizuku.newProcess(...)  (dipanggil REFLEKTIF)
```

`BinaryShellResult` hanya punya tiga field: `code: Int`, `stdout: ByteArray`, `stderr: ByteArray`.

`stdin` adalah parameter opsional pada `ShizukuCommandRunner.run`. Di bytecode ia muncul sebagai
dua method: `run$default(adapter, args, stdin, cont, mask, marker)` untuk pemanggil yang
melewatkan `stdin`, dan method abstrak `run(List, byte[], Continuation)` yang dipanggil
`ShizukuStorageAdapter.raw` secara langsung (baris 1133). Jadi byte **memang** dialirkan.

Rantai lengkap di `AndroidShizukuCommandRunner$run$2` (baris dalam kurung):

```
Method.invoke(null, {args, env, dir})          ← newProcess, REFLEKTIF            (:241–297)
  ├─ Process.getOutputStream()                  ← branch: stdin == null → langsung close (:314–318)
  ├─ OutputStream.write(byte[]) + closeFinally  ← branch: stdin != null            (:324–351)
  ├─ Process.getInputStream()  → readBytes()    → stdout                            (:355–)
  ├─ Process.getErrorStream() → readBytes()     → stderr                            (:369–)
  ├─ Process.waitFor()                          → exit code                         (:385)
  └─ BinaryShellResult(code, stdout, stderr)                                        (:389)
```

Tidak ada `redirectErrorStream`, jadi stderr benar-benar terpisah dari stdout. Urutannya
penting: stdin ditulis dan stream-nya ditutup **sebelum** stdout dibaca — kalau tidak,
proses yang menulis besar ke stdout akan memblokir pipe.

`run$default` melempar `UnsupportedOperationException("Super calls with default arguments not supported in this target, function: run")` hanya saat subclass memanggil `super.run` (`p5 != 0`); jalur biasa memasukkan `null` sesuai bit mask lalu memanggil method abstrak. Argumen shell disusun di method luar sebelum state-machine `$1` (yang tidak punya `const-string`) memasuki titik suspend.

## Tiga kelas storage — dan fakta bahwa hanya satu yang dipakai

| Kelas | Implementasi / instansiasi |
|---|---|
| `NormalGameStorage` | I/O `java.io.File`; tidak pernah diinstansiasi. |
| `ShizukuGameStorage` | Wrapper yang hanya meneruskan ke `ShizukuStorageAdapter`; tidak pernah diinstansiasi. |
| `ShizukuStorageAdapter` | Shell Shizuku; satu-satunya backend yang dipakai. |

### Apa yang benar-benar ditolak `NormalGameStorage`

Hanya dua operasi yang memang mustahil tanpa shell:

- `forceStop` → selalu `Normal file backend cannot force-stop applications`
- `capability()` → melaporkan kondisi, bukan melempar:
  `Storage root does not exist`, `Storage root is not a directory`,
  `Normal app process cannot read the storage root`,
  `Normal app process has read-only access to the game resource root`

Sisanya (`readBytes`, `writeBytes`, `list`, `findFirst`, `digest`, `copy`, `move`, `delete`,
`mkdirs`, `stat`) benar-benar mencoba I/O biasa dan gagal saat runtime dengan:

- `ensureWritableDirectory` → `Cannot create target parent`, `Target parent is not writable`, `Target has no parent`
- `requireReadable` → `Path is not readable: `, `Path not found: `
- `ioFailure` → `"<op> failed for <path>: <exception>"`

### Kebijakan pemilihan backend

`WuWaMobileController.<init>` selalu menjalankan `storage = ShizukuStorageAdapter()` (default
root); tidak ada probe `capability()`, cabang, atau fallback. Aplikasi tidak menginstansiasi
`NormalGameStorage`/`ShizukuGameStorage`; installer menerima adapter sebagai `SafeGameStorage`.
Shizuku wajib sejak app dibuka: UI menampilkan `Cần quyền Shizuku` /
`Mở Shizuku trước; WAHU không tự bật hoặc tự cấp quyền`, dengan readiness
`SHIZUKU_NOT_RUNNING` / `SHIZUKU_PERMISSION_REQUIRED`. Fallback I/O biasa adalah fitur baru;
Android 11+ juga membatasi tulis biasa ke `Android/data`.

### Path guard

`LocalStoragePathGuard` pada `NormalGameStorage` memanggil `StoragePathGuard.validateSegments`
yang sama, lalu memakai `File.getCanonicalFile()` dan `Path.startsWith(rootPath)` untuk
containment; perbedaannya hanya pemeriksaan containment.

### Kontrak `SafeGameStorage` (memperluas `GameStorage`)

Perimeter interface = 22 method:

| Interface | Kontrak |
|---|---|
| `GameStorage` (15 abstrak) | `canAccess`, `copy`, `delete`, `exists`, `findFirst`, `forceStop`, `hasPermission`, `isAvailable`, `list`, `mkdirs`, `move`, `readText`, `requestPermission`, `sha1`, `sha256` |
| `SafeGameStorage` (6 abstrak) | `capability`, `getBackend`, `getRootPath`, `readBytes`, `stat`, `writeBytes` |
| `SafeGameStorage` (1 default) | `hash(path, alg)`: `SHA1 → sha1`, `SHA256 → sha256`; nilai lain melempar `NoWhenBranchMatchedException` |

Installer memanggil 16: `capability`, `copy`, `delete`, `exists`, `findFirst`, `forceStop`,
`hasPermission`, `isAvailable`, `list`, `mkdirs`, `move`, `readBytes`, `readText`, `sha1`,
`sha256`, `writeBytes`.

`raw(args, stdin)` memanggil runner; gagal memulai proses melempar
`StorageException("Shizuku command could not be started: " + …)`. `requireOk` mengembalikan
hasil bila `code == 0`; selain itu mendekode `stderr` sebagai UTF-8, `trim()`, lalu melempar
`StorageException("Shizuku command failed ($code): $err", …)`.

Setiap operasi storage membuat proses Shizuku baru. `requireGranted()` menolak dengan
`Shizuku permission has not been granted by the user` atau `Shizuku service is not running`.

## Tabel perintah lengkap

| Method | argv | Catatan |
|---|---|---|
| `canAccess(p)` | `test -r <p>` | |
| `capability()` | `test -e <root>` / `test -r <root>` / `test -w <root>` | tiga probe terpisah; pesan: `Game resource root does not exist`, `Shizuku shell cannot read the game resource root`, `Shizuku shell has read-only access to the game resource root`, `Shizuku capability probe failed: …` |
| `exists(p)` | `test -e -- <p>` | |
| `mkdirs(p)` | `mkdir -p -- <p>` | |
| `readBytes(p)` | `cat -- <p>` | guard `"Read target is not a regular file: "` via `test -f` |
| `readText(p)` | `readBytes` + decode UTF-8 | |
| `stat(p)` | `stat -c '%s\t%Y' -- <p>` + `test -e` + `test -d` | dua kolom: ukuran, mtime; galat: `Invalid stat size for …`, `Invalid stat timestamp for …`, `Unexpected stat output for …` |
| `list(dir)` | `find <abs> -mindepth 1 -maxdepth 1 -print0` | `-print0` → aman terhadap nama file ber-spasi/ber-newline; hasil di-filter: `Shizuku returned a path outside the configured root`, `List target is not a directory: `, `Directory does not exist: ` |
| `findFirst(dir, pat)` | `find <abs> -type f -name <pat> -print -quit` | **pat disanitasi**: ditolak kalau mengandung `/`, `\`, atau salah satu dari `*?[` → `"Unsafe find suffix"`; hasil: `"find returned a path outside storage root"` |
| `copy(src,dst)` | `mkdir -p -- <parent>` lalu `cp -f -- <src> <dst>` | |
| `move(src,dst)` | `mkdir -p -- <parent>` lalu `mv -f -- <src> <dst>` | |
| `delete(p)` | `rmdir -- <p>` (kalau direktori) / `rm -f -- <p>` | root sendiri dilindungi: `"Deleting the storage root is not allowed"` |
| `sha1(p)` / `sha256(p)` | `toybox sha1sum -- <p>` / `toybox sha256sum -- <p>` | `HashAlgorithm` memetakan `SHA1→"sha1sum"`, `SHA256→"sha256sum"`; label `SHA-1`/`SHA-256`, nama enum `SHA1`/`SHA256` |
| `digest(p,alg)` | `test -f -- <p>` dulu, lalu perintah algoritma | lowercase output (`toLowerCase(...)`) |
| `forceStop(pkg)` | `am force-stop <pkg>` | **allowlist**: hanya `com.kurogame.wutheringwaves.global`; selain itu `"Refusing to force-stop an unexpected package"` |
| `writeRaw(p, bytes)` | `sh -c 'cat > "$1"' wahu-write <p>` | bytes via stdin; `<p>` dikirim sebagai argumen `$1`, bukan interpolasi skrip (spasi/kutip tidak di-parse ulang) |

```text
target = resolve(path)
requireOk(["mkdir", "-p", "--", parentOf(target)])
if !atomic: writeRaw(target, data)
else:
    suffix = tempSuffix()  // injectable
    require ^[A-Za-z0-9._-]+$ or throw "Unsafe temporary suffix"
    tmp = target + ".wahu.tmp-" + suffix
    writeRaw(tmp, data)
    requireOk(["mv", "-f", "--", tmp, target])
    on failure: requireOk(["rm", "-f", "--", tmp])
```

Penulisan atomik mengganti file final dengan rename; suffix hanya boleh berisi karakter regex
tersebut, dan file sementara dibersihkan pada jalur kegagalan.

## `StoragePathGuard` (chokepoint)

Semua 40 method publik memanggil `resolve(path)` → `StoragePathGuard.resolvePosix(root, path)`;
root dinormalisasi sekali pada konstruktor. Guard berbasis string (bukan `java.nio.Path`):

| Method / field | Kontrak |
|---|---|
| `normalizePosixRoot(root)` | wajib root absolut POSIX; selainnya `Storage root must be an absolute POSIX path` |
| `resolvePosix(root, path)` | `\` → `/`; cek `startsWith("/")`, buang prefix root, pecah segmen, tolak `.`/`..`, bangun ulang dengan `StringBuilder`; keluar root → `Path is outside storage root: ` / `StorageErrorCode.PATH_OUTSIDE_ROOT` |
| `validateSegments(path)` | menolak segmen `.`/`..` → `Traversal or ambiguous path segments are not allowed: ` |
| `drivePrefix = Regex("^[A-Za-z]:")` | menolak path Windows → `Windows-style absolute path is not allowed` |

`resolvePosix` juga menolak path absolut (`Unsafe absolute path: `) dan NUL
(`NUL is not allowed in a storage path`). `find`/`list` memvalidasi ulang hasil shell terhadap
root: `find returned a path outside storage root` / `Shizuku returned a path outside the
configured root`. `LocalStoragePathGuard` berbagi `validateSegments`.
