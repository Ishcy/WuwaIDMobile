package com.titotfp.wuwaid.update

import android.app.Activity
import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import android.os.Build
import android.system.Os
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileOutputStream
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.zip.ZipFile

// Accessed only by the updater's single worker, never by the game-storage service.
internal class AppUpdateRepository(
  private val activity: Activity,
  private val certificate: String = CERTIFICATE,
  private val connect: (URL) -> HttpURLConnection = { it.openConnection() as HttpURLConnection },
) {
  private val directory = File(activity.cacheDir, "app-updates")
  val apk = File(directory, "update.apk")
  private val metadata = File(directory, "candidate.json")
  private val partial = File(directory, "update.part.apk")
  private val metadataPartial = File(directory, "candidate.json.part")
  private var restored = false
  private var candidate: JSONObject? = null
  private var cachedDigest: String? = null
  private var checked = false
  private var notice = "Belum memeriksa pembaruan aplikasi."

  private fun installed(): PackageInfo = activity.packageManager.getPackageInfo(activity.packageName, signingFlags())

  // API 28's archive parser collects certificates only when GET_SIGNATURES is also set.
  private fun signingFlags(): Int = if (Build.VERSION.SDK_INT >= 28)
    PackageManager.GET_SIGNING_CERTIFICATES or PackageManager.GET_SIGNATURES else PackageManager.GET_SIGNATURES

  private fun code(info: PackageInfo): Long = if (Build.VERSION.SDK_INT >= 28) info.longVersionCode else info.versionCode.toLong()

  private fun signer(info: PackageInfo): String {
    val signatures = if (Build.VERSION.SDK_INT >= 28) info.signingInfo?.apkContentsSigners else info.signatures
    require(signatures?.size == 1) { "APK harus memiliki tepat satu sertifikat signing." }
    return hex(MessageDigest.getInstance("SHA-256").digest(signatures[0].toByteArray()))
  }

  private fun unsupported(current: PackageInfo): String? = when {
    activity.packageName != PACKAGE -> "Updater hanya tersedia pada APK release; APK debug tidak dapat memperbarui aplikasi release."
    !Build.SUPPORTED_ABIS.contains("arm64-v8a") -> "Rilis aplikasi saat ini hanya mendukung Android ARM64."
    signer(current) != certificate -> "Sertifikat aplikasi terpasang tidak cocok dengan signing rilis asli."
    else -> null
  }

  fun snapshot(): JSONObject {
    val current = installed()
    val reason = unsupported(current)
    if (reason == null) restore()
    val release = candidate
    return JSONObject()
      .put("supported", reason == null)
      .put("currentVersion", current.versionName ?: JSONObject.NULL)
      .put("currentVersionCode", code(current))
      .put("availableVersion", release?.getString("version") ?: JSONObject.NULL)
      .put("size", release?.getLong("size") ?: 0L)
      .put("ready", release != null && cachedDigest == release.getString("sha256") && apk.isFile)
      .put("checked", checked)
      .put("message", reason ?: notice)
  }

  private fun requireSupported() {
    val reason = unsupported(installed())
    check(reason == null) { reason ?: "Updater tidak tersedia." }
  }

  private fun restore() {
    if (restored) return
    cleanPartials()
    if (!metadata.isFile) {
      remove(apk)
      restored = true
      return
    }
    restored = true
    try {
      require(metadata.length() in 1..8192) { "Metadata cache updater tidak valid." }
      val release = JSONObject(metadata.readText())
      validate(release)
      if (compareVersion(release.getString("version"), installed().versionName.orEmpty()) <= 0) {
        clearCache()
        return
      }
      verifyApk(release, apk)
      candidate = release
      cachedDigest = release.getString("sha256")
      notice = "APK update yang sudah diverifikasi siap dipasang."
    } catch (error: Exception) {
      clearCache()
      throw IllegalStateException("Cache update tidak dapat digunakan: ${error.message}. Periksa pembaruan lagi.")
    }
  }

  fun check(): JSONObject {
    requireSupported()
    restore()
    val body = text("$API/releases/latest", 4 * 1024 * 1024, allowMissing = true)
    checked = true
    if (body == null) {
      notice = if (candidate == null) "Belum ada rilis stabil aplikasi yang dipublikasikan."
        else "Belum ada rilis stabil pada server; kandidat update yang sebelumnya diperiksa tetap tersedia di cache."
      return snapshot()
    }
    val release = JSONObject(body)
    require(!release.getBoolean("draft") && !release.getBoolean("prerelease")) { "Rilis aplikasi bukan rilis stabil." }
    val tag = release.getString("tag_name")
    require(TAG.matches(tag)) { "Tag versi aplikasi tidak valid." }
    if (compareVersion(tag.removePrefix("v"), installed().versionName.orEmpty()) <= 0) {
      clearCache()
      notice = "Tidak ada versi aplikasi yang lebih baru pada kanal stabil."
      return snapshot()
    }
    val name = "WuwaID-Mobile-$tag.apk"
    val assets = release.getJSONArray("assets")
    val asset = uniqueAsset(assets, name)
    val url = asset.getString("browser_download_url")
    require(url == assetUrl(tag, name)) { "URL APK bukan aset rilis aplikasi resmi." }
    val digest = asset.optString("digest").takeIf { it.isNotEmpty() && it != "null" }
    val hash = if (digest != null) {
      require(digest.startsWith("sha256:")) { "Algoritme digest APK tidak didukung." }
      digest.removePrefix("sha256:")
    } else {
      val sums = uniqueAsset(assets, "SHA256sums.txt")
      require(sums.getString("browser_download_url") == assetUrl(tag, "SHA256sums.txt")) {
        "Checksum APK tidak tersedia pada rilis resmi."
      }
      val checksum = text(assetUrl(tag, "SHA256sums.txt"), 64 * 1024)!!
      var found: String? = null
      for (line in checksum.lineSequence()) {
        val match = CHECKSUM.matchEntire(line) ?: continue
        if (match.groupValues[2] != name) continue
        require(found == null) { "Checksum APK tidak unik." }
        found = match.groupValues[1]
      }
      found ?: throw IllegalStateException("Checksum APK tidak ditemukan.")
    }
    val selected = JSONObject().put("version", tag.removePrefix("v")).put("tag", tag)
      .put("name", name).put("url", url).put("size", asset.getLong("size"))
      .put("sha256", hash.lowercase())
    validate(selected)
    // A successfully selected different release supersedes the old cached APK.
    // Failed/offline checks never reach this point, preserving the ready candidate.
    if (cachedDigest != selected.getString("sha256") || candidate?.optString("tag") != tag ||
      candidate?.optLong("size") != selected.getLong("size")) clearCache()
    candidate = selected
    notice = "Pembaruan aplikasi ${selected.getString("version")} tersedia."
    return snapshot()
  }

  private fun uniqueAsset(assets: org.json.JSONArray, name: String): JSONObject {
    var found: JSONObject? = null
    for (index in 0 until assets.length()) {
      val asset = assets.getJSONObject(index)
      if (asset.getString("name") != name || asset.getString("state") != "uploaded") continue
      require(found == null) { "Aset rilis $name tidak unik." }
      found = asset
    }
    return found ?: throw IllegalStateException("Rilis tidak memiliki aset resmi $name.")
  }

  fun download(progress: (Int, String) -> Unit): JSONObject {
    requireSupported()
    restore()
    val release = candidate ?: throw IllegalStateException("Periksa pembaruan sebelum mengunduh APK.")
    validate(release)
    if (cachedDigest == release.getString("sha256") && apk.isFile) {
      verifyReady()
      progress(100, "APK update terverifikasi sudah tersedia di cache.")
      return snapshot()
    }
    check(directory.isDirectory || directory.mkdirs()) { "Tidak dapat menyiapkan cache update." }
    cleanPartials()
    try {
      val connection = open(release.getString("url"))!!
      val expected = release.getLong("size")
      val hasher = MessageDigest.getInstance("SHA-256")
      var count = 0L
      var lastPercent = -1
      progress(0, "Mengunduh APK update…")
      try {
        connection.inputStream.use { input ->
          FileOutputStream(partial).use { output ->
            val buffer = ByteArray(64 * 1024)
            while (true) {
              check(!Thread.currentThread().isInterrupted) { "Unduhan update dihentikan." }
              val size = input.read(buffer)
              if (size < 0) break
              count += size
              require(count <= expected) { "Ukuran unduhan APK melebihi metadata rilis." }
              output.write(buffer, 0, size)
              hasher.update(buffer, 0, size)
              val percent = (count * 95 / expected).toInt()
              if (percent != lastPercent) {
                lastPercent = percent
                progress(percent, "Mengunduh APK update…")
              }
            }
            output.fd.sync()
          }
        }
      } finally {
        connection.disconnect()
      }
      require(count == expected && hex(hasher.digest()) == release.getString("sha256")) {
        "Ukuran atau SHA-256 APK update tidak cocok. File tidak akan dipasang."
      }
      progress(96, "Memeriksa identitas dan signing APK…")
      verifyPackage(release, partial)
      check(partial.setReadOnly()) { "Tidak dapat mengunci APK update." }
      FileOutputStream(metadataPartial).use { output ->
        output.write(release.toString().toByteArray(Charsets.UTF_8))
        output.fd.sync()
      }
      Os.rename(partial.path, apk.path)
      Os.rename(metadataPartial.path, metadata.path)
      cachedDigest = release.getString("sha256")
      notice = "APK update sudah diverifikasi. Tekan Pasang update untuk membuka pemasang Android."
      progress(100, "APK update siap dipasang.")
      return snapshot()
    } finally {
      cleanPartials()
    }
  }

  fun verifyReady(): File {
    requireSupported()
    restore()
    val release = candidate ?: throw IllegalStateException("Belum ada APK update yang siap dipasang.")
    check(cachedDigest == release.getString("sha256")) { "Unduh APK update terlebih dahulu." }
    try {
      verifyApk(release, apk)
    } catch (error: Exception) {
      cachedDigest = null
      remove(apk)
      remove(metadata)
      throw IllegalStateException("APK cache tidak lolos verifikasi: ${error.message}. Unduh ulang APK update.")
    }
    return apk
  }

  private fun verifyApk(release: JSONObject, file: File) {
    require(file.isFile && file.length() == release.getLong("size")) { "Ukuran APK di cache tidak cocok." }
    val hasher = MessageDigest.getInstance("SHA-256")
    file.inputStream().use { input ->
      val buffer = ByteArray(64 * 1024)
      while (true) {
        check(!Thread.currentThread().isInterrupted) { "Verifikasi update dihentikan." }
        val count = input.read(buffer)
        if (count < 0) break
        hasher.update(buffer, 0, count)
      }
    }
    require(hex(hasher.digest()) == release.getString("sha256")) { "SHA-256 APK di cache tidak cocok." }
    verifyPackage(release, file)
  }

  private fun verifyPackage(release: JSONObject, file: File) {
    val current = installed()
    val info = activity.packageManager.getPackageArchiveInfo(file.path, signingFlags())
      ?: throw IllegalStateException("Android tidak dapat membaca APK update.")
    require(info.packageName == PACKAGE && info.versionName == release.getString("version")) {
      "Package atau versi APK tidak sesuai dengan rilis resmi."
    }
    require(code(info) > code(current)) { "versionCode APK harus lebih tinggi dari aplikasi terpasang." }
    require(signer(info) == certificate && signer(current) == certificate) {
      "Sertifikat APK update tidak cocok dengan aplikasi rilis asli."
    }
    require((info.applicationInfo?.minSdkVersion ?: Int.MAX_VALUE) <= Build.VERSION.SDK_INT) {
      "APK update memerlukan versi Android yang lebih baru."
    }
    ZipFile(file).use { zip ->
      require(zip.entries().asSequence().any { it.name.startsWith("lib/arm64-v8a/") && it.name.endsWith(".so") }) {
        "APK update tidak memiliki pustaka native ARM64."
      }
    }
  }

  private fun validate(release: JSONObject) {
    val tag = release.getString("tag")
    val name = release.getString("name")
    require(TAG.matches(tag) && release.getString("version") == tag.removePrefix("v")) { "Versi update tidak valid." }
    require(name == "WuwaID-Mobile-$tag.apk" && release.getString("url") == assetUrl(tag, name)) { "Aset update tidak valid." }
    require(release.getLong("size") in 1..MAX_APK_BYTES && HASH.matches(release.getString("sha256"))) {
      "Ukuran atau digest update tidak valid."
    }
  }

  private fun clearCache() {
    candidate = null
    cachedDigest = null
    remove(apk)
    remove(metadata)
    cleanPartials()
  }

  private fun remove(file: File) {
    check(!file.exists() || file.delete()) { "Tidak dapat membersihkan cache update ${file.name}." }
  }

  private fun cleanPartials() {
    remove(partial)
    remove(metadataPartial)
  }

  private fun text(url: String, limit: Int, allowMissing: Boolean = false): String? {
    val connection = open(url, allowMissing) ?: return null
    try {
      connection.inputStream.use { input ->
        val output = ByteArrayOutputStream()
        val buffer = ByteArray(8192)
        while (true) {
          val count = input.read(buffer)
          if (count < 0) break
          require(output.size() + count <= limit) { "Respons server update terlalu besar." }
          output.write(buffer, 0, count)
        }
        return output.toString("UTF-8")
      }
    } finally {
      connection.disconnect()
    }
  }

  private fun open(address: String, allowMissing: Boolean = false): HttpURLConnection? {
    var url = URL(address)
    for (redirect in 0..5) {
      check(!Thread.currentThread().isInterrupted) { "Permintaan update dihentikan." }
      require(url.protocol == "https" && url.host in HOSTS && (url.port == -1 || url.port == 443) && url.userInfo == null && url.ref == null) {
        "URL update tidak aman."
      }
      val connection = connect(url)
      connection.instanceFollowRedirects = false
      connection.connectTimeout = 15000
      connection.readTimeout = 20000
      connection.setRequestProperty("User-Agent", "WuwaIDMobile/${installed().versionName}")
      connection.setRequestProperty("Accept-Encoding", "identity")
      if (url.host == "api.github.com") connection.setRequestProperty("Accept", "application/vnd.github+json")
      try {
        val status = connection.responseCode
        if (status == 301 || status == 302 || status == 303 || status == 307 || status == 308) {
          val location = connection.getHeaderField("Location") ?: throw IllegalStateException("Redirect update tidak memiliki tujuan.")
          url = URL(url, location)
          connection.disconnect()
          continue
        }
        if (status == 404 && allowMissing) {
          connection.disconnect()
          return null
        }
        require(status == 200) {
          if (status == 403 || status == 429) "GitHub membatasi permintaan update. Coba lagi nanti."
          else "Server update mengembalikan HTTP $status."
        }
        return connection
      } catch (error: Exception) {
        connection.disconnect()
        throw error
      }
    }
    throw IllegalStateException("Terlalu banyak redirect server update.")
  }

  private fun compareVersion(left: String, right: String): Int {
    require(VERSION.matches(left) && VERSION.matches(right)) { "Format versi aplikasi tidak didukung." }
    var ai = 0
    var bi = 0
    repeat(3) {
      var a = 0L
      var b = 0L
      while (ai < left.length && left[ai] != '.') a = a * 10 + (left[ai++] - '0')
      while (bi < right.length && right[bi] != '.') b = b * 10 + (right[bi++] - '0')
      if (a != b) return a.compareTo(b)
      ai++
      bi++
    }
    return 0
  }

  private fun assetUrl(tag: String, name: String) = "https://github.com/TitoTFP/WuwaIDMobile/releases/download/$tag/$name"
  private fun hex(bytes: ByteArray): String {
    val chars = CharArray(bytes.size * 2)
    val digits = "0123456789abcdef"
    for (index in bytes.indices) {
      val byte = bytes[index].toInt() and 255
      chars[index * 2] = digits[byte ushr 4]
      chars[index * 2 + 1] = digits[byte and 15]
    }
    return String(chars)
  }

  companion object {
    private const val PACKAGE = "com.titotfp.wuwaid"
    private const val CERTIFICATE = "2cb1b4fed5446cf52018b5d56d58c379d7328022c1ba17278c19c0002e484df9"
    private const val API = "https://api.github.com/repos/TitoTFP/WuwaIDMobile"
    private const val MAX_APK_BYTES = 256L * 1024 * 1024
    private val VERSION = Regex("[0-9]{1,9}\\.[0-9]{1,9}\\.[0-9]{1,9}")
    private val TAG = Regex("v[0-9]{1,9}\\.[0-9]{1,9}\\.[0-9]{1,9}")
    private val HASH = Regex("[0-9a-f]{64}")
    private val CHECKSUM = Regex("([0-9a-fA-F]{64}) [ *](.+)")
    private val HOSTS = setOf("api.github.com", "github.com", "release-assets.githubusercontent.com", "objects.githubusercontent.com")
  }
}
