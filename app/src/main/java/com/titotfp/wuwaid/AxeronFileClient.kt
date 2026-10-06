package com.titotfp.wuwaid

import android.content.Context
import android.os.ParcelFileDescriptor
import frb.axeron.client.AxeronBridge
import frb.axeron.server.FileStat
import frb.axeron.server.IFileService
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.io.IOException
import java.nio.charset.StandardCharsets
import java.security.MessageDigest

/**
 * Privileged file backend backed directly by AxManager/Axeron.
 *
 * This avoids Shizuku UserService entirely. AxManager must have
 * AX-Permission Intercept enabled so it delivers its Axeron binder to the
 * AxeronProvider declared by WuwaIDMobile.
 */
internal class AxeronFileClient(
    context: Context,
) : PrivilegedFiles {
    private val appContext = context.applicationContext
    private val allowedRoots = buildList {
        for (pkg in GamePaths.SUPPORTED_PACKAGES) {
            add(GamePaths.gameRoot(pkg))
        }
        appContext.getExternalFilesDir(null)?.canonicalPath?.let {
            add(it.substringBefore("/files"))
        }
    }.map { File(it).canonicalFile }.distinct()

    @Volatile private var error = ""

    fun start(onStateChanged: () -> Unit) {
        AxeronBridge.start(onStateChanged)
    }

    fun stop(onStateChanged: () -> Unit) {
        AxeronBridge.stop(onStateChanged)
    }

    fun isAvailable(): Boolean = AxeronBridge.isAvailable()

    fun hasPermission(): Boolean = AxeronBridge.hasPermission()

    fun requestPermission() = AxeronBridge.requestPermission()

    fun isReady(): Boolean = isAvailable() && hasPermission() && AxeronBridge.fileService() != null

    override fun copyFile(source: String, destination: String): Boolean =
        runCatching {
            val src = checked(source)
            val dst = checked(destination)
            val fs = requireFileService()
            val stat = requireNotNull(fs.stat(src)) { "File sumber tidak tersedia" }
            require(stat.exists && stat.isFile) { "File sumber tidak tersedia" }
            copyRemote(fs, src, dst)
        }.finish()

    override fun replaceFile(source: String, destination: String): Boolean =
        runCatching {
            val src = checked(source)
            val dst = checked(destination)
            requireFileService().renameTo(src, dst, true)
        }.finish()

    override fun deleteFile(path: String): Boolean =
        runCatching { !requireFileService().exists(checked(path)) || requireFileService().delete(checked(path)) }.finish()

    override fun exists(path: String): Boolean =
        runCatching { requireFileService().exists(checked(path)) }.value(false)

    override fun mkdirs(path: String): Boolean =
        runCatching { requireFileService().mkdirs(checked(path)) }.finish()

    override fun listFiles(path: String): Array<String> =
        runCatching {
            requireFileService()
                .list(checked(path), IFileService.FILTER_ALL, IFileService.SORT_NAME, true, 0, -1)
                .orEmpty()
                .mapNotNull { it.name }
                .sorted()
                .toTypedArray()
        }.value(emptyArray())

    override fun readText(path: String): String =
        runCatching {
            val pfd = requireFileService().inputStreamPfd(checked(path))
                ?: throw IOException("Tidak bisa membuka file")
            ParcelFileDescriptor.AutoCloseInputStream(pfd).use {
                it.readBytes().toString(StandardCharsets.UTF_8)
            }
        }.value("")

    override fun writeTextAtomic(path: String, content: String): Boolean =
        runCatching {
            val destination = checked(path)
            val temporary = checked("${destination}.tmp")
            val fs = requireFileService()
            writeRemote(fs, temporary, content.toByteArray(StandardCharsets.UTF_8))
            if (!fs.renameTo(temporary, destination, true)) {
                fs.delete(temporary)
                throw IOException("rename atomic gagal")
            }
            true
        }.finish()

    override fun sha1(path: String): String = hash(path, "SHA-1")

    override fun sha256(path: String): String = hash(path, "SHA-256")

    override fun lastError(): String = error

    private fun hash(path: String, algorithm: String): String =
        runCatching {
            val pfd = requireFileService().inputStreamPfd(checked(path))
                ?: throw IOException("Tidak bisa membuka file")
            val digest = MessageDigest.getInstance(algorithm)
            ParcelFileDescriptor.AutoCloseInputStream(pfd).use { input ->
                val buffer = ByteArray(BUFFER_SIZE)
                while (true) {
                    val count = input.read(buffer)
                    if (count < 0) break
                    digest.update(buffer, 0, count)
                }
            }
            digest.digest().joinToString("") { "%02x".format(it.toInt() and 0xff) }
        }.value("")

    private fun copyRemote(fs: IFileService, source: String, destination: String): Boolean {
        val srcPfd = fs.inputStreamPfd(source) ?: throw IOException("Tidak bisa membuka sumber")
        val input = ParcelFileDescriptor.AutoCloseInputStream(srcPfd)
        val pipe = ParcelFileDescriptor.createPipe()
        val readEnd = pipe[0]
        val writeEnd = pipe[1]
        val done = java.util.concurrent.CountDownLatch(1)
        var remoteError: String? = null
        val callback = object : frb.axeron.server.IOutputStreamCallback.Stub() {
            override fun onComplete() { done.countDown() }
            override fun onError(errCode: Int, message: String?) {
                remoteError = message ?: "Output service gagal ($errCode)"
                done.countDown()
            }
        }
        try {
            fs.outputStreamPfd(destination, readEnd, callback, false)
            FileOutputStream(writeEnd.fileDescriptor).use { output ->
                input.use { inputStream -> inputStream.copyTo(output, BUFFER_SIZE) }
                output.flush()
                fs.fsync(writeEnd)
            }
            writeEnd.close()
            if (!done.await(30, java.util.concurrent.TimeUnit.SECONDS)) {
                throw IOException("Output service timeout")
            }
            remoteError?.let { throw IOException(it) }
            return fs.exists(destination)
        } finally {
            runCatching { readEnd.close() }
            runCatching { writeEnd.close() }
        }
    }

    private fun writeRemote(fs: IFileService, destination: String, bytes: ByteArray) {
        val pipe = ParcelFileDescriptor.createPipe()
        val readEnd = pipe[0]
        val writeEnd = pipe[1]
        val done = java.util.concurrent.CountDownLatch(1)
        var remoteError: String? = null
        val callback = object : frb.axeron.server.IOutputStreamCallback.Stub() {
            override fun onComplete() { done.countDown() }
            override fun onError(errCode: Int, message: String?) {
                remoteError = message ?: "Output service gagal ($errCode)"
                done.countDown()
            }
        }
        try {
            fs.outputStreamPfd(destination, readEnd, callback, false)
            FileOutputStream(writeEnd.fileDescriptor).use { output ->
                output.write(bytes)
                output.flush()
                fs.fsync(writeEnd)
            }
            writeEnd.close()
            if (!done.await(30, java.util.concurrent.TimeUnit.SECONDS)) {
                throw IOException("Output service timeout")
            }
            remoteError?.let { throw IOException(it) }
        } finally {
            runCatching { readEnd.close() }
            runCatching { writeEnd.close() }
        }
    }

    private fun requireFileService(): IFileService =
        AxeronBridge.fileService() ?: throw IOException("AxManager/Axeron file service belum siap")

    private fun checked(path: String): String {
        require(path.isNotBlank()) { "Path kosong" }
        val file = File(path).canonicalFile
        val allowed = allowedRoots.any { root ->
            file == root || file.path.startsWith(root.path + File.separator)
        }
        require(allowed) { "Path di luar direktori yang diizinkan" }
        return file.path
    }

    private fun Result<Boolean>.finish(): Boolean =
        fold(
            onSuccess = {
                error = ""
                it
            },
            onFailure = {
                error = it.message ?: it.javaClass.simpleName
                false
            },
        )

    private fun <T> Result<T>.value(fallback: T): T =
        fold(
            onSuccess = {
                error = ""
                it
            },
            onFailure = {
                error = it.message ?: it.javaClass.simpleName
                fallback
            },
        )

    companion object {
        private const val BUFFER_SIZE = 64 * 1024
    }
}
