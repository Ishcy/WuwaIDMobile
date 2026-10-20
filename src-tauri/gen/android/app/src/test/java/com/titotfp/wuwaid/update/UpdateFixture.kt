package com.titotfp.wuwaid.update

import android.app.Activity
import android.content.pm.PackageManager
import android.content.pm.Signature
import android.content.pm.SigningInfo
import android.os.Build
import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import com.android.apksig.ApkSigner
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.tls.HandshakeCertificates
import okhttp3.tls.HeldCertificate
import org.json.JSONArray
import org.json.JSONObject
import org.robolectric.Shadows
import org.robolectric.shadow.api.Shadow
import org.robolectric.shadows.ShadowBuild
import java.io.File
import javax.net.ssl.HttpsURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.zip.CRC32
import java.util.zip.ZipEntry
import java.util.zip.ZipFile
import java.util.zip.ZipOutputStream

internal const val UPDATE_PACKAGE = "com.titotfp.wuwaid"
internal const val REPOSITORY_URL = "https://github.com/TitoTFP/WuwaIDMobile/releases/download"

class UpdateTestActivity : AppCompatActivity() {
    var installedPackage = UPDATE_PACKAGE
    override fun getPackageName(): String = installedPackage
    override fun onCreate(savedInstanceState: Bundle?) {
        setTheme(androidx.appcompat.R.style.Theme_AppCompat)
        super.onCreate(savedInstanceState)
    }
}

/** Real SDK-produced APKs and local TLS; never uses the release private key or GitHub network. */
internal class UpdateFixture(private val root: File) : AutoCloseable {
    val key = HeldCertificate.Builder().rsa2048().commonName("WM-26 disposable test signer").build()
    val certificate = sha256(key.certificate.encoded)
    val server = MockWebServer()
    private val tlsKey = HeldCertificate.Builder().addSubjectAlternativeName("localhost").build()
    private val client = HandshakeCertificates.Builder().addTrustedCertificate(tlsKey.certificate).build()
    private val sdk = File(requireNotNull(System.getProperty("wm26.sdk")))
    private val aapt = File(sdk, "build-tools/36.0.0/aapt2")
    private val platform = File(sdk, "platforms/android-36/android.jar")
    private val library = File(root, "libfixture.so")
    private var sequence = 0

    init {
        root.mkdirs()
        val ndk = File(System.getProperty("wm26.ndk").takeUnless { it.isNullOrBlank() }
            ?: error("Set NDK_HOME to build the ARM64 test APK fixture"))
        val host = File(ndk, "toolchains/llvm/prebuilt").listFiles()!!.single { it.isDirectory }
        command(listOf(File(host, "bin/clang").path, "--target=aarch64-linux-android24", "-shared",
            "-nostdlib", "-x", "c", "-", "-o", library.path), "int wm26_fixture(void) { return 26; }")
        server.useHttps(HandshakeCertificates.Builder().heldCertificate(tlsKey).build().sslSocketFactory(), false)
        server.start()
    }

    fun repository(activity: Activity): AppUpdateRepository = AppUpdateRepository(activity, certificate) { official ->
        // Repository validates the original URL before this factory, including every redirect.
        val local = URL(server.url(official.file).toString())
        (local.openConnection() as HttpsURLConnection).apply { sslSocketFactory = client.sslSocketFactory() }
    }

    fun installBaseline(activity: Activity, version: String = "0.4.0", code: Int = 8, signer: HeldCertificate = key) {
        ShadowBuild.setSupportedAbis(arrayOf("arm64-v8a"))
        val archive = apk(version = version, code = code, signers = listOf(signer))
        val info = requireNotNull(activity.packageManager.getPackageArchiveInfo(archive.path, 0)) {
            "Android PackageManager could not parse the signed baseline fixture"
        }
        // Installed-package metadata is an OS boundary; the APK-under-test is parsed separately.
        val signatures = arrayOf(Signature(signer.certificate.encoded))
        info.signatures = signatures
        if (Build.VERSION.SDK_INT >= 28) {
            info.signingInfo = Shadow.newInstanceOf(SigningInfo::class.java).also {
                Shadows.shadowOf(it).setSignatures(signatures)
            }
        }
        Shadows.shadowOf(activity.packageManager).installPackage(info)
    }

    fun apk(version: String = "0.4.1", code: Int = 9, packageName: String = UPDATE_PACKAGE,
            minSdk: Int = 24, abi: String? = "arm64-v8a", signers: List<HeldCertificate> = listOf(key)): File {
        val folder = File(root, "apk-${sequence++}").apply { mkdirs() }
        val manifest = File(folder, "AndroidManifest.xml").apply { writeText("""
            <manifest xmlns:android="http://schemas.android.com/apk/res/android" package="$packageName"
                android:versionCode="$code" android:versionName="$version">
                <uses-sdk android:minSdkVersion="$minSdk" android:targetSdkVersion="28" />
                <application android:label="WM-26 test fixture" android:hasCode="false" />
            </manifest>
        """.trimIndent()) }
        val linked = File(folder, "linked.apk")
        command(listOf(aapt.path, "link", "-I", platform.path, "--manifest", manifest.path, "-o", linked.path))
        val unsigned = File(folder, "unsigned.apk")
        ZipOutputStream(unsigned.outputStream()).use { output ->
            ZipFile(linked).use { input ->
                input.entries().asSequence().forEach { entry ->
                    output.putNextEntry(ZipEntry(entry.name))
                    input.getInputStream(entry).use { it.copyTo(output) }
                    output.closeEntry()
                }
            }
            if (abi != null) {
                output.putNextEntry(ZipEntry("lib/$abi/libfixture.so"))
                library.inputStream().use { it.copyTo(output) }
                output.closeEntry()
            }
            val payload = PAYLOAD.toByteArray()
            output.putNextEntry(ZipEntry("assets/probe.dat").apply {
                method = ZipEntry.STORED; size = payload.size.toLong()
                crc = CRC32().apply { update(payload) }.value
            })
            output.write(payload)
            output.closeEntry()
        }
        if (signers.isEmpty()) return unsigned
        val signed = File(folder, "signed.apk")
        ApkSigner.Builder(signers.mapIndexed { index, signer ->
            ApkSigner.SignerConfig.Builder("wm26-$index", signer.keyPair.private, listOf(signer.certificate)).build()
        })
            .setInputApk(unsigned).setOutputApk(signed).setMinSdkVersion(minSdk)
            .setV1SigningEnabled(false).setV2SigningEnabled(true).setV3SigningEnabled(false)
            .build().sign()
        return signed
    }

    fun tampered(apk: File): File {
        val bytes = apk.readBytes()
        val marker = PAYLOAD.toByteArray()
        val offset = (0..bytes.size - marker.size).single { index ->
            marker.indices.all { bytes[index + it] == marker[it] }
        }
        bytes[offset] = (bytes[offset].toInt() xor 1).toByte()
        return File(apk.parentFile, "tampered.apk").apply { writeBytes(bytes) }
    }

    fun release(apk: File, version: String = "0.4.1", digest: String? = "sha256:${sha256(apk.readBytes())}",
                size: Long = apk.length()): JSONObject {
        val tag = "v$version"
        val name = "WuwaID-Mobile-$tag.apk"
        val asset = JSONObject().put("name", name).put("state", "uploaded").put("size", size)
            .put("browser_download_url", "$REPOSITORY_URL/$tag/$name")
        if (digest != null) asset.put("digest", digest)
        return JSONObject().put("tag_name", tag).put("draft", false).put("prerelease", false)
            .put("assets", JSONArray().put(asset))
    }

    fun enqueueRelease(release: JSONObject) = server.enqueue(MockResponse().setBody(release.toString()))
    fun enqueueApk(apk: File) = server.enqueue(MockResponse().setBody(okio.Buffer().write(apk.readBytes())))
    override fun close() { server.shutdown() }

    private fun command(args: List<String>, input: String = "") {
        val process = ProcessBuilder(args).redirectErrorStream(true).start()
        process.outputStream.bufferedWriter().use { it.write(input) }
        val output = process.inputStream.bufferedReader().readText()
        check(process.waitFor() == 0) { "Fixture tool failed: ${args.first()}\n$output" }
    }

    companion object {
        private const val PAYLOAD = "wm26-original-signed-payload-for-integrity-regression"
        fun sha256(bytes: ByteArray): String = MessageDigest.getInstance("SHA-256")
            .digest(bytes).joinToString("") { "%02x".format(it) }
    }
}
