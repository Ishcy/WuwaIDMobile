package com.titotfp.wuwaid.update

import android.content.pm.Signature
import android.content.pm.SigningInfo
import org.robolectric.annotation.ClassName
import org.robolectric.annotation.Implementation
import org.robolectric.annotation.Implements
import org.robolectric.annotation.RealObject
import org.robolectric.shadows.ShadowSigningInfo
import org.robolectric.shadows.ShadowLinux
import org.robolectric.shadows.ShadowPosix
import org.robolectric.util.ReflectionHelpers
import org.robolectric.util.ReflectionHelpers.ClassParameter
import org.robolectric.util.reflector.Direct
import org.robolectric.util.reflector.ForType
import org.robolectric.util.reflector.Reflector.reflector
import java.io.EOFException
import java.io.FileDescriptor
import java.io.FileInputStream
import java.nio.ByteBuffer
import java.nio.file.Files
import java.nio.file.Paths
import java.nio.file.StandardCopyOption

/** Only replaces Android's libcore-only DirectByteBuffer/mmap adapter, not APK verification. */
@Implements(className = "android.util.apk.MemoryMappedFileDataSource", isInAndroidSdk = false, minSdk = 26)
class HostApkFileDataSource {
    @RealObject private lateinit var source: Any

    @Implementation
    protected fun feedIntoDataDigester(@ClassName("android.util.apk.DataDigester") digester: Any,
                                      offset: Long, size: Int) {
        val descriptor = ReflectionHelpers.getField<FileDescriptor>(source, "mFd")
        val position = ReflectionHelpers.getField<Long>(source, "mFilePosition") + offset
        val buffer = ByteBuffer.allocate(size)
        // Borrowed descriptor: the platform verifier's RandomAccessFile owns and closes it.
        val channel = FileInputStream(descriptor).channel
        while (buffer.hasRemaining()) {
            if (channel.read(buffer, position + buffer.position()) < 0) throw EOFException()
        }
        buffer.flip()
        ReflectionHelpers.callInstanceMethod<Void>(digester, "consume", ClassParameter.from(ByteBuffer::class.java, buffer))
    }
}

/** Robolectric's default shadow ignores signing details supplied by the real framework parser. */
@Implements(SigningInfo::class, minSdk = 28)
class HostSigningInfo : ShadowSigningInfo() {
    @RealObject private lateinit var info: SigningInfo

    @Implementation
    override fun getApkContentsSigners(): Array<Signature>? = super.getApkContentsSigners()
        ?: reflector(SigningInfoAccess::class.java, info).getApkContentsSigners()

    @ForType(SigningInfo::class)
    private interface SigningInfoAccess {
        @Direct fun getApkContentsSigners(): Array<Signature>?
    }
}

/** Robolectric has no rename implementation; preserve the production atomic replacement semantics. */
@Implements(className = "libcore.io.Linux", isInAndroidSdk = false, minSdk = 26)
class HostLinux : ShadowLinux() {
    @Implementation protected fun rename(source: String, target: String) {
        Files.move(Paths.get(source), Paths.get(target), StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING)
    }
}

@Implements(className = "libcore.io.Posix", isInAndroidSdk = false, maxSdk = 25)
class HostPosix : ShadowPosix() {
    @Implementation protected fun rename(source: String, target: String) {
        Files.move(Paths.get(source), Paths.get(target), StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING)
    }
}
