package com.titotfp.wuwaid.storage

import android.os.Bundle
import android.os.ParcelFileDescriptor
import java.io.ByteArrayOutputStream
import java.io.InputStream
import java.nio.charset.StandardCharsets

private val resourceVersion = Regex("""(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)""")
private val ownedFiles = buildSet {
  for (path in listOf("wuwaid/WuwaID_99_P.pak", "wuwaid/WuwaID_99_P.sig",
    "wuwaid/.owner.json", "wuwaid/.transaction.json", "Mount/wuwaid.txt")) {
    add(path)
    add("$path.new")
    add("$path.bak")
  }
}

internal fun requireWritableTarget(
  path: String,
  allowPatchRoot: Boolean,
  allowMountDirectory: Boolean = false,
  directoryOnly: Boolean = false
) {
  val separator = path.indexOf('/')
  if (separator <= 0 || !resourceVersion.matches(path.substring(0, separator))) {
    throw SecurityException("Mutation requires a numeric game resource version")
  }
  val relative = path.substring(separator + 1)
  val directory = (allowPatchRoot && relative == "wuwaid") ||
    (allowMountDirectory && relative == "Mount")
  if (!directory && (directoryOnly || relative !in ownedFiles)) {
    throw SecurityException("Mutation is restricted to WuwaID-owned resource paths")
  }
}

class StorageUserService : IStorageUserService.Stub() {
  override fun execute(
    operation: String?,
    first: String?,
    second: String?,
    input: ParcelFileDescriptor?
  ): Bundle {
    val inputFd = input ?: return failure("Storage command input pipe is missing")
    val script = try {
      scriptFor(operation, first, second)
    } catch (error: Exception) {
      try { inputFd.close() } catch (_: Exception) {}
      return failure(error.message ?: error.javaClass.simpleName)
    }
    val arguments = buildList {
      if (first != null) add(first)
      if (second != null) add(second)
    }
    val command = arrayListOf("/system/bin/sh", "-c", script, "wuwaid-storage", RESOURCE_ROOT)
    command.addAll(arguments)
    val process = try {
      ProcessBuilder(command).start()
    } catch (error: Exception) {
      try { inputFd.close() } catch (_: Exception) {}
      return failure(error.message ?: error.javaClass.simpleName)
    }

    return try {
      val stdout = OutputCapture(process.inputStream, MAX_METADATA_BYTES)
      val stderr = OutputCapture(process.errorStream, MAX_ERROR_BYTES)
      val stdoutThread = Thread(stdout, "wuwaid-user-service-stdout").apply { isDaemon = true; start() }
      val stderrThread = Thread(stderr, "wuwaid-user-service-stderr").apply { isDaemon = true; start() }
      val inputFailure = try {
        ParcelFileDescriptor.AutoCloseInputStream(inputFd).use { remoteInput ->
          process.outputStream.use { shellInput -> remoteInput.copyTo(shellInput, STREAM_BUFFER_BYTES) }
        }
        null
      } catch (error: Exception) {
        error
      }
      if (inputFailure != null) {
        process.destroy()
        return failure(inputFailure.message ?: inputFailure.javaClass.simpleName)
      }

      val exitCode = process.waitFor()
      stdoutThread.join()
      stderrThread.join()
      val stdoutBytes = stdout.bytes()
      val stderrBytes = stderr.bytes()
      if (stdoutBytes.size > MAX_METADATA_BYTES) {
        return failure("Shizuku metadata response exceeded its limit")
      }
      if (stderrBytes.size > MAX_ERROR_BYTES) {
        return failure("Shizuku error output exceeded its limit")
      }
      Bundle().apply {
        putInt(KEY_EXIT_CODE, exitCode)
        putByteArray(KEY_STDOUT, stdoutBytes)
        putByteArray(KEY_STDERR, stderrBytes)
      }
    } catch (error: Exception) {
      process.destroy()
      failure(error.message ?: error.javaClass.simpleName)
    } finally {
      try { inputFd.close() } catch (_: Exception) {}
    }
  }

  override fun destroy() {
    System.exit(0)
  }

  private fun scriptFor(operation: String?, first: String?, second: String?): String {
    val op = operation ?: throw IllegalArgumentException("Storage operation is required")
    when (op) {
      "root_readable", "root_writable", "force_stop" -> require(first == null && second == null) { "Unexpected storage arguments" }
      "list" -> {
        require(first != null && second == null) { "Directory path is required" }
        validateRelative(first, allowEmpty = true)
      }
      "stat", "read_text", "remove" -> {
        require(first != null && second == null) { "Resource path is required" }
        val path = validateRelative(first, allowEmpty = false)
        if (op == "remove") requireWritableTarget(path, allowPatchRoot = true)
      }
      "hash" -> {
        require(first != null && second != null) { "Path and hash algorithm are required" }
        validateRelative(first, allowEmpty = false)
        require(second == "sha1" || second == "sha256") { "Unsupported hash algorithm" }
      }
      "mkdir" -> {
        require(first != null && second == null) { "Directory path is required" }
        val path = validateRelative(first, allowEmpty = false)
        requireWritableTarget(path, allowPatchRoot = true, allowMountDirectory = true, directoryOnly = true)
      }
      "copy" -> {
        require(first != null && second != null) { "Source and target paths are required" }
        validateRelative(first, allowEmpty = false)
        requireWritableTarget(validateRelative(second, allowEmpty = false), allowPatchRoot = false)
      }
      "move_file" -> {
        require(first != null && second != null) { "Source and target paths are required" }
        requireWritableTarget(validateRelative(first, allowEmpty = false), allowPatchRoot = false)
        requireWritableTarget(validateRelative(second, allowEmpty = false), allowPatchRoot = false)
      }
      "write" -> {
        require(first != null && second == null) { "Write target path is required" }
        requireWritableTarget(validateRelative(first, allowEmpty = false), allowPatchRoot = false)
      }
      else -> throw IllegalArgumentException("Unsupported storage operation")
    }
    return when (op) {
      "root_readable" -> ROOT_READABLE_SCRIPT
      "root_writable" -> ROOT_WRITABLE_SCRIPT
      "list" -> LIST_SCRIPT
      "stat" -> STAT_SCRIPT
      "read_text" -> READ_TEXT_SCRIPT
      "hash" -> HASH_SCRIPT
      "mkdir" -> MKDIR_SCRIPT
      "copy" -> COPY_SCRIPT
      "move_file" -> MOVE_SCRIPT
      "remove" -> REMOVE_SCRIPT
      "write" -> WRITE_SCRIPT
      "force_stop" -> FORCE_STOP_SCRIPT
      else -> throw IllegalArgumentException("Unsupported storage operation")
    }
  }


  private fun validateRelative(path: String, allowEmpty: Boolean): String {
    if (path.length > MAX_PATH_LENGTH || path.indexOf('\u0000') >= 0 || path.startsWith('/') || path.contains('\\')) {
      throw IllegalArgumentException("Invalid resource-relative path")
    }
    if (path.isEmpty()) {
      if (allowEmpty) return path
      throw IllegalArgumentException("Path cannot be empty")
    }
    if (path.split('/').any { it.isEmpty() || it == "." || it == ".." }) {
      throw IllegalArgumentException("Invalid resource-relative path")
    }
    return path
  }

  private fun failure(message: String): Bundle = Bundle().apply {
    putInt(KEY_EXIT_CODE, 255)
    putByteArray(KEY_STDOUT, byteArrayOf())
    putByteArray(KEY_STDERR, message.take(4096).toByteArray(StandardCharsets.UTF_8))
  }

  private class OutputCapture(private val stream: InputStream, private val limit: Int) : Runnable {
    private val output = ByteArrayOutputStream(minOf(limit + 1, 8192))
    @Volatile private var failure: Exception? = null

    override fun run() {
      try {
        stream.use { input ->
          val bytes = ByteArray(STREAM_BUFFER_BYTES)
          while (true) {
            val count = input.read(bytes)
            if (count < 0) break
            val remaining = (limit + 1 - output.size()).coerceAtLeast(0)
            if (remaining > 0) output.write(bytes, 0, minOf(count, remaining))
          }
        }
      } catch (error: Exception) {
        failure = error
      }
    }

    fun bytes(): ByteArray {
      failure?.let { throw IllegalStateException("Unable to read Shizuku process output", it) }
      return output.toByteArray()
    }
  }

  companion object {
    private const val RESOURCE_ROOT = "/storage/emulated/0/Android/data/com.kurogame.wutheringwaves.global/files/UE4Game/Client/Client/Saved/Resources"
    private const val MAX_PATH_LENGTH = 4096
    private const val MAX_METADATA_BYTES = 256 * 1024
    private const val MAX_ERROR_BYTES = 16 * 1024
    private const val STREAM_BUFFER_BYTES = 64 * 1024
    private const val KEY_EXIT_CODE = "exitCode"
    private const val KEY_STDOUT = "stdout"
    private const val KEY_STDERR = "stderr"

    private val ROOT_CHECK = shell("""
EXPECTED_FILES=$(readlink -f -- "/storage/emulated/0/Android/data/com.kurogame.wutheringwaves.global/files") || { echo 'game files directory is unavailable' >&2; exit 1; }
[ -d "§EXPECTED_FILES" ] && [ "§ROOT" = "§EXPECTED_FILES/UE4Game/Client/Client/Saved/Resources" ] || { echo 'resource root is not the expected game directory' >&2; exit 1; }
""")

    private val ROOT_READABLE_SCRIPT = shell("""
set -f
ROOT=$(readlink -f -- "§1") || { echo 'resource root is unavailable' >&2; exit 1; }
""") + ROOT_CHECK + shell("""
[ -d "§ROOT" ] && [ -r "§ROOT" ] && [ -x "§ROOT" ] && ls -A "§ROOT" >/dev/null || { echo 'resource root is not readable' >&2; exit 1; }
printf '1'
""")

    private val ROOT_WRITABLE_SCRIPT = shell("""
set -f
ROOT=$(readlink -f -- "§1") || { echo 'resource root is unavailable' >&2; exit 1; }
""") + ROOT_CHECK + shell("""
[ -d "§ROOT" ] && [ -w "§ROOT" ] && [ -x "§ROOT" ] || { echo 'resource root is not writable' >&2; exit 1; }
printf '1'
""")

    private val CHECK_PATH_FUNCTION = shell("""
set -f
ROOT=$(readlink -f -- "§1") || { echo 'resource root is unavailable' >&2; exit 1; }
""") + ROOT_CHECK + shell("""
check_rel() {
  rel="§1"
  [ -n "§rel" ] || return 0
  current="§ROOT"
  saved_ifs="§IFS"
  IFS='/'
  for component in §rel; do
    [ -n "§component" ] || { IFS="§saved_ifs"; echo 'invalid path component' >&2; return 1; }
    current="§current/§component"
    [ ! -L "§current" ] || { IFS="§saved_ifs"; echo 'symlinks are not allowed' >&2; return 1; }
    if [ -e "§current" ]; then
      resolved=$(readlink -f -- "§current") || { IFS="§saved_ifs"; echo 'unable to resolve resource path' >&2; return 1; }
      case "§resolved" in "§ROOT"|"§ROOT"/*) ;; *) IFS="§saved_ifs"; echo 'resource path escapes the game root' >&2; return 1 ;; esac
    fi
  done
  IFS="§saved_ifs"
  return 0
}
""")

    private fun checked(body: String): String = CHECK_PATH_FUNCTION + shell(body)

    private val LIST_SCRIPT = checked("""
REL="§2"
check_rel "§REL" || exit 1
DIR="§ROOT"
[ -z "§REL" ] || DIR="§ROOT/§REL"
[ -d "§DIR" ] || { echo 'resource path is not a directory' >&2; exit 1; }
set +f
for entry in "§DIR"/* "§DIR"/.[!.]* "§DIR"/..?*; do
  [ -e "§entry" ] || [ -L "§entry" ] || continue
  [ ! -L "§entry" ] || { echo 'symlinks are not allowed' >&2; exit 1; }
  resolved=$(readlink -f -- "§entry") || { echo 'unable to resolve directory entry' >&2; exit 1; }
  case "§resolved" in "§ROOT"|"§ROOT"/*) ;; *) echo 'directory entry escapes the game root' >&2; exit 1 ;; esac
  name=§{entry##*/}
  if [ -d "§entry" ]; then kind=1; else kind=0; fi
  printf '%s\000%s\000' "§name" "§kind"
done
""")

    private val STAT_SCRIPT = checked("""
REL="§2"
check_rel "§REL" || exit 1
TARGET="§ROOT"
[ -z "§REL" ] || TARGET="§ROOT/§REL"
if [ -d "§TARGET" ]; then
  printf 'D\t0'
elif [ -f "§TARGET" ]; then
  size=$(stat -c %s "§TARGET") || { echo 'unable to stat resource file' >&2; exit 1; }
  printf 'F\t%s' "§size"
else
  printf 'N\t0'
fi
""")

    private val READ_TEXT_SCRIPT = checked("""
REL="§2"
check_rel "§REL" || exit 1
TARGET="§ROOT/§REL"
[ -f "§TARGET" ] || { echo 'resource is not a regular file' >&2; exit 1; }
size=$(wc -c < "§TARGET") || { echo 'unable to read resource size' >&2; exit 1; }
set -- §size
[ "§{1:-0}" -le 262144 ] || { echo 'text resource exceeds the 256 KiB limit' >&2; exit 1; }
cat "§TARGET"
""")

    private val HASH_SCRIPT = checked("""
REL="§2"
ALGORITHM="§3"
check_rel "§REL" || exit 1
TARGET="§ROOT/§REL"
[ -f "§TARGET" ] || { echo 'resource is not a regular file' >&2; exit 1; }
case "§ALGORITHM" in
  sha1) result=$(sha1sum "§TARGET") || { echo 'sha1sum failed' >&2; exit 1; } ;;
  sha256) result=$(sha256sum "§TARGET") || { echo 'sha256sum failed' >&2; exit 1; } ;;
  *) echo 'unsupported hash algorithm' >&2; exit 1 ;;
esac
printf '%s' "§{result%% *}"
""")

    private val MKDIR_SCRIPT = checked("""
REL="§2"
check_rel "§REL" || exit 1
TARGET="§ROOT/§REL"
mkdir -p "§TARGET" || { echo 'unable to create owned resource directory' >&2; exit 1; }
check_rel "§REL" || exit 1
[ -d "§TARGET" ] || { echo 'created resource path is not a directory' >&2; exit 1; }
""")

    private val COPY_SCRIPT = checked("""
SOURCE_REL="§2"
TARGET_REL="§3"
check_rel "§SOURCE_REL" || exit 1
check_rel "§TARGET_REL" || exit 1
SOURCE="§ROOT/§SOURCE_REL"
TARGET="§ROOT/§TARGET_REL"
[ -f "§SOURCE" ] || { echo 'copy source is not a regular file' >&2; exit 1; }
[ ! -e "§TARGET" ] && [ ! -L "§TARGET" ] || { echo 'copy target already exists' >&2; exit 1; }
PARENT=§{TARGET%/*}
[ -d "§PARENT" ] || { echo 'copy target parent is not a directory' >&2; exit 1; }
cp "§SOURCE" "§TARGET" || { echo 'resource copy failed' >&2; exit 1; }
check_rel "§TARGET_REL" || exit 1
""")

    private val MOVE_SCRIPT = checked("""
SOURCE_REL="§2"
TARGET_REL="§3"
check_rel "§SOURCE_REL" || exit 1
check_rel "§TARGET_REL" || exit 1
SOURCE="§ROOT/§SOURCE_REL"
TARGET="§ROOT/§TARGET_REL"
[ -e "§SOURCE" ] || { echo 'move source does not exist' >&2; exit 1; }
[ ! -e "§TARGET" ] && [ ! -L "§TARGET" ] || { echo 'move target already exists' >&2; exit 1; }
PARENT=§{TARGET%/*}
[ -d "§PARENT" ] || { echo 'move target parent is not a directory' >&2; exit 1; }
mv "§SOURCE" "§TARGET" || { echo 'resource rename failed' >&2; exit 1; }
check_rel "§TARGET_REL" || exit 1
""")

    private val REMOVE_SCRIPT = checked("""
REL="§2"
check_rel "§REL" || exit 1
TARGET="§ROOT/§REL"
if [ -d "§TARGET" ]; then
  rmdir "§TARGET" || { echo 'owned directory is not empty' >&2; exit 1; }
else
  rm -f "§TARGET" || { echo 'resource removal failed' >&2; exit 1; }
fi
""")
    private val WRITE_SCRIPT = checked("""
REL="§2"
check_rel "§REL" || exit 1
TARGET="§ROOT/§REL"
PARENT=§{TARGET%/*}
[ -d "§PARENT" ] || { echo 'write target parent is not a directory' >&2; exit 1; }
[ ! -d "§TARGET" ] || { echo 'write target is a directory' >&2; exit 1; }
cat > "§TARGET" || { echo 'resource write failed' >&2; exit 1; }
check_rel "§REL" || exit 1
""")

    private const val FORCE_STOP_SCRIPT = """
set -f
am force-stop com.kurogame.wutheringwaves.global || { echo 'unable to force-stop Wuthering Waves' >&2; exit 1; }
"""

    private fun shell(script: String): String = script.replace('§', '$')
  }
}