# 07 — Transaction Engine & Marker Format

Source: `smali_classes3/vn/wahu/vhwuwa/installer/TransactionalPatchEngine.smali` (8,046 lines), `InstallTransactionLayout`, `InstallTransactionCodec`, `TransactionMarker`, `FileSnapshot`. The codec manually emits JSON and parses it with four regexes; it is not a binary format.

## 1. File layout inside the mount directory

`InstallTransactionLayout` is pure path derivation — 9 methods, 5 literals:

```kotlin
fun backup(target: String)   = "$target.bak"          // backup copy of a managed file
fun staging(target: String)  = "$target.new"          // staged new copy
fun marker(paths: AndroidPatchPaths)  = "${paths.directory}/.wahu-transaction.json"
fun markerStaging(paths: AndroidPatchPaths) = "${marker(paths)}.new"

fun targets(paths)      = listOf(paths.pak, paths.signature, paths.mount)   // 3 managed files
fun stagingFiles(paths) = targets.map(::staging) + markerStaging(paths)    // 4 files
fun backupFiles(paths)  = targets.map(::backup)                            // 3 files
```

Named transaction paths: 3 live targets, 3 `.bak`, 3 target `.new`, marker, and marker `.new` (11 total). `stagingFiles()` returns the three target `.new` files plus marker `.new`; marker is beside the files in the WAHU mount directory, not a separate state directory.

## 2. `FileSnapshot` — the rollback primitive

```kotlin
data class FileSnapshot(val existed: Boolean, val sha256: String?)
```

Two invariants enforced in the constructor, both verified from the const pool:

- `Existing transaction snapshot must carry SHA-256` — `existed == true` requires a digest
- `Missing transaction target cannot carry SHA-256` — `existed == false` forbids a digest

Digest format is validated by `Regex("^[0-9a-fA-F]{64}$")`, and `captureSnapshots`
lowercases before storing (`toLowerCase(...)`).

This single pair is the whole rollback mechanism: `existed=false` means *delete this path
on rollback*, `existed=true` means *restore these bytes* (verified by digest before the move).

## 3. `TransactionMarker` — the on-disk contract

```kotlin
data class TransactionMarker(
    val schema: Int,
    val operation: TransactionOperation,      // INSTALL | REPAIR | UNINSTALL
    val phase: InstallPhase,
    val resourceVersion: String,
    val variant: TranslationVariant,
    val pak: FileSnapshot,
    val signature: FileSnapshot,
    val mount: FileSnapshot,
)
```

### Wire format (from `InstallTransactionCodec.encode`)

**Shape is verified, values are illustrative.** Key order, the PascalCase `Existed`/`Sha256`
casing and the `null` spelling all come from `InstallTransactionCodec.encode`; the concrete
`phase` / `resourceVersion` / `variant` values are constructed. This is encoder output as it
*would* be written, not a file captured from a device.

```json
{"schema":1,
 "operation":"INSTALL",
 "phase":"BACKUP",
 "resourceVersion":"3.7.0",
 "variant":"HAN_VIET",
 "pak":{"Existed":true,"Sha256":"<64 hex lowercase>"},
 "signature":{"Existed":false,"Sha256":null},
 "mount":{"Existed":false,"Sha256":null}}
```

Notes that matter for a port:

- Field names are **PascalCase inside snapshots** (`Existed`, `Sha256`) but camelCase at the
  top level (`resourceVersion`). The codec appends them literally.
- `Sha256` is emitted lowercase, or the bare token `null`.
- `NULL_SENTINEL = "__WAHU_NULL__"` exists as an internal round-trip marker so a literal
  string `"null"` in a text field cannot be confused with an absent value.
- Parse side uses exactly four regexes: `"\s*:\s*(true|false)"`, `"\s*:\s*(\d+)"`,
  `"\s*:\s*null`, `"\s*:\s*"([^"]*)""`. No real JSON parser.

### Write protocol

`persistMarker` is a write-verify-read cycle, not a blind write:

1. write marker (UTF-8 bytes)
2. `Transaction marker verify thất bại` → re-read failed
3. `Không đọc lại được transaction marker` → cannot re-read

The marker is persisted **before** every destructive step, so the on-disk phase always
reflects the step about to run. A crash therefore leaves a marker that names the phase to
resume from — which is what `recover` reads.

## 4. `TransactionalPatchEngine` — phase sequences

Traced by resolving `InstallPhase` static-field references and intra-class calls inside each
method's exact range:

| Entry point | Sequence |
|---|---|
| `install` | `ownership` → `replaceOwnedSet` |
| `repair` | `ownership` → `replaceOwnedSet` |
| `uninstall` | `ownership` → `ensureNoOrphanBackups` → `cleanupStaging` → **BACKUP** → `captureSnapshots` → `markerFor` → **BACKUP** → `persistMarker` → `backupTargets` → **INSTALL** → `persistMarker` → **VERIFY_TARGET** → `persistMarker` → **COMMIT** → `persistMarker` → `finalizeCommitted` (`rollbackAfterFailure` on throw) |
| `recover` | `ensureNoOrphanBackups` → `cleanupStaging` → if already past **COMMIT**, `finalizeCommitted`; otherwise **ROLLBACK** → `rollbackFromMarker` → **VERIFY_ROLLBACK** |
| `rollbackAfterFailure` | **ROLLBACK** → `cleanupStaging` → `rollbackFromMarker` → **VERIFY_ROLLBACK** |

`replaceOwnedSet` is the only long routine and the one worth porting verbatim:

```
ensureNoOrphanBackups
cleanupStaging
STAGE          → write pak/signature/mount to their .new paths
VERIFY_SOURCE  → normalizeNewlines on the mount descriptor, then compare
                 "Mount descriptor staging không khớp"
                 "PAK <f> bị sai SHA-256 sau khi stage"
                 "SIG staging không khớp SIG chính thức"
                 "Không tính được SHA-1 PAK staging" / "... SIG staging" / "... SIG chính thức"
BACKUP         → captureSnapshots (3× exists+sha256) → markerFor → persistMarker → backupTargets
INSTALL        → persistMarker, then move every .new over its target
VERIFY_TARGET  → persistMarker → verifyInstalled
                 "Mount descriptor đã cài không khớp"
                 "SHA-1 SIG đã cài không khớp"
                 "SHA-256 PAK đã cài không khớp"
COMMIT         → persistMarker → finalizeCommitted (cleanupStaging)
```

`uninstall` runs the **same skeleton** — the marker is written with
`operation=UNINSTALL` and the `INSTALL` phase removes the three files instead of replacing
them. That symmetry is the design: install and uninstall are one routine parameterised by
`TransactionOperation`.

`finalizeCommitted` = `cleanupStaging` only. Commit is the point of no return.

## 5. Rollback

`rollbackAfterFailure` refuses to guess:

- `Transaction marker hỏng; từ chối rollback mù` — marker unreadable/corrupt
- `Rollback SHA-256 không khớp: ` — restored bytes differ from the snapshot
- `Rollback thiếu target gốc: ` — `.bak` for a path the marker says existed, is gone

`verifyRollback` then asserts the post-state:

- `Rollback verify hash thất bại: `
- `Rollback phải phục hồi target: ` — a file that existed before must be back
- `Rollback phải xóa target mới: ` — a file that did not exist before must be gone

Both directions are checked. That is what makes repair idempotent.

## 6. Preflight guards

- `ensureNoOrphanBackups` → `Backup WAHU orphan chưa được xử lý: ` and
  `Phát hiện backup WAHU không có transaction marker; từ chối tự đoán rollback`
  — a `.bak` with no marker is a hard stop, never a guess.
- `ownership` (gate for install/repair/uninstall) →
  `Có file cùng tên WAHU nhưng thiếu PAK baseline; từ chối ghi/xóa`
  `WuWaVH_99_P.pak hiện tại không khớp baseline WAHU; từ chối ghi đè/xóa`
  `Mount/wahuviethoa.txt không được xác nhận là WAHU-owned; từ chối ghi đè/xóa`
- `isWahuDescriptor` recognises a descriptor as WAHU-owned by the literals `.pak`,
  `WuWaVH_99_P.pak`, `wahuviethoa/`, split on `::` — a structural check on the file content,
  not a stored flag.
- `normalizeNewlines` converts `\r\n` → `\n` before the descriptor is written or compared,
  so the descriptor on disk is always LF.

## 7. Progress strings emitted by the engine

Vietnamese, user-visible, and part of the localization surface:

```
Đang stage <file>…            Đang xác minh staging…
Đang tạo rollback snapshot…   Đang commit bộ file WAHU…
Đang xác minh bản cài…        Có lỗi, đang rollback exact…
Rollback đã được xác minh
Đang tạo rollback snapshot trước khi gỡ…   Đang gỡ các file WAHU-owned…
Đang xác minh trạng thái đã gỡ…
Phát hiện transaction gián đoạn, đang rollback…   Đã phục hồi transaction gián đoạn
```

`recover` additionally distinguishes the two interesting refusals:

- `Transaction marker thuộc resource khác` — marker is for a different resource version
- `Transaction marker WAHU không hợp lệ; từ chối rollback mù`

and `repair`:
- `Không đọc được PAK WAHU để Repair`
- `Chưa có patch WAHU để sửa`

## 8. Port checklist

`phase` is persisted as the `InstallPhase` enum **name**; renaming it breaks in-flight recovery.
Declaration order is alphabetical—`BACKUP, COMMIT, IDLE, INSTALL, PREFLIGHT, ROLLBACK,
STAGE, VERIFY_ROLLBACK, VERIFY_SOURCE, VERIFY_TARGET`—not runtime order (§4); never drive
the state machine from `ordinal()`.