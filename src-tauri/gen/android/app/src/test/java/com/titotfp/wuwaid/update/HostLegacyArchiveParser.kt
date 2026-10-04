package com.titotfp.wuwaid.update

import android.content.pm.ApplicationInfo
import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import android.content.pm.Signature
import com.android.apksig.ApkVerifier
import com.android.apksig.apk.ApkUtils
import com.android.apksig.internal.apk.AndroidBinXmlParser
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Implementation
import org.robolectric.annotation.Implements
import org.robolectric.shadows.ShadowApplicationPackageManager
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.zip.ZipFile

/**
 * API 24-27 Robolectric resources cannot parse external binary APK manifests. Model that OS
 * boundary by decoding the actual APK bytes and verifying them with Google's APK verifier.
 * This does not prove the on-device PackageManager implementation or APK installation.
 */
@Implements(className = "android.app.ApplicationPackageManager", isInAndroidSdk = false, minSdk = 24, maxSdk = 27)
class HostLegacyArchiveParser : ShadowApplicationPackageManager() {
    @Implementation
    override fun getPackageArchiveInfo(archiveFilePath: String, flags: Int): PackageInfo? {
        val file = File(archiveFilePath)
        if (!file.isFile) return null
        val manifest = ZipFile(file).use { zip ->
            zip.getInputStream(zip.getEntry("AndroidManifest.xml")).use {
                ByteBuffer.wrap(it.readBytes()).order(ByteOrder.LITTLE_ENDIAN)
            }
        }
        val parsedPackage = ApkUtils.getPackageNameFromBinaryAndroidManifest(manifest.duplicate())
        val info = PackageInfo().apply {
            packageName = parsedPackage
            versionCode = ApkUtils.getVersionCodeFromBinaryAndroidManifest(manifest.duplicate())
            applicationInfo = ApplicationInfo().apply {
                packageName = parsedPackage
                sourceDir = archiveFilePath
                publicSourceDir = archiveFilePath
                minSdkVersion = ApkUtils.getMinSdkVersionFromBinaryAndroidManifest(manifest.duplicate())
                targetSdkVersion = ApkUtils.getTargetSdkVersionFromBinaryAndroidManifest(manifest.duplicate())
            }
        }
        val parser = AndroidBinXmlParser(manifest.duplicate())
        while (parser.next() != AndroidBinXmlParser.EVENT_END_DOCUMENT) {
            if (parser.eventType == AndroidBinXmlParser.EVENT_START_ELEMENT && parser.name == "manifest") {
                for (index in 0 until parser.attributeCount) {
                    if (parser.getAttributeName(index) == "versionName") info.versionName = parser.getAttributeStringValue(index)
                }
            }
        }
        if (flags and PackageManager.GET_SIGNATURES != 0) {
            val api = RuntimeEnvironment.getApiLevel()
            val verified = ApkVerifier.Builder(file).setMinCheckedPlatformVersion(api)
                .setMaxCheckedPlatformVersion(api).build().verify()
            if (!verified.isVerified) return null
            info.signatures = verified.signerCertificates.map { Signature(it.encoded) }.toTypedArray()
        }
        return info
    }
}
