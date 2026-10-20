package com.titotfp.wuwaid.diagnostics

import android.app.Activity
import android.content.ClipData
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.os.Build
import androidx.activity.result.ActivityResult
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.FileProvider
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicReference

@InvokeArg
class DiagnosticExportArgs {
    var action: String = ""
    var content: String = ""
}

@TauriPlugin
class DiagnosticsPlugin(private val activity: Activity) : Plugin(activity) {
    private data class Pending(val invoke: Invoke, val bytes: ByteArray)
    private val pending = AtomicReference<Pending?>(null)
    private val worker = Executors.newSingleThreadExecutor { runnable ->
        Thread(runnable, "wuwaid-diagnostics").apply { isDaemon = true }
    }

    @Command
    fun environment(invoke: Invoke) {
        val application = section {
            val info = activity.packageManager.getPackageInfo(activity.packageName, 0)
            JSONObject().put("applicationId", activity.packageName)
                .put("name", activity.packageManager.getApplicationLabel(activity.applicationInfo).toString())
                .put("version", info.versionName ?: JSONObject.NULL)
                .put("versionCode", if (Build.VERSION.SDK_INT >= 28) info.longVersionCode else info.versionCode.toLong())
                .put("buildType", if (activity.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0) "debug" else "release")
        }
        val device = section {
            JSONObject().put("os", "Android").put("sdk", Build.VERSION.SDK_INT)
                .put("version", Build.VERSION.RELEASE).put("manufacturer", Build.MANUFACTURER)
                .put("model", Build.MODEL).put("abis", JSONArray().also { abis ->
                    for (abi in Build.SUPPORTED_ABIS) abis.put(abi)
                })
        }
        invoke.resolveObject(JSONObject().put("application", application).put("device", device))
    }

    private inline fun section(collect: () -> JSONObject): JSONObject = try {
        JSONObject().put("status", "ok").put("data", collect())
            .put("error", JSONObject.NULL).put("reason", JSONObject.NULL)
    } catch (error: Exception) {
        JSONObject().put("status", "error").put("data", JSONObject.NULL)
            .put("error", error.message ?: "Metadata tidak dapat dibaca.").put("reason", JSONObject.NULL)
    }

    @Command
    fun exportReport(invoke: Invoke) {
        if (pending.get() != null) {
            invoke.reject("Ekspor diagnostik lain masih berjalan.")
            return
        }
        val args = try {
            invoke.parseArgs(DiagnosticExportArgs::class.java).also {
                require(it.action == "save" || it.action == "share") { "Tindakan diagnostik tidak dikenal." }
                require(it.content.isNotEmpty()) { "Laporan diagnostik kosong." }
            }
        } catch (error: Exception) {
            invoke.reject(error.message ?: "Permintaan diagnostik tidak valid.")
            return
        }
        val request = Pending(invoke, args.content.toByteArray(Charsets.UTF_8))
        if (!pending.compareAndSet(null, request)) {
            invoke.reject("Ekspor diagnostik lain masih berjalan.")
            return
        }
        if (args.action == "save") {
            onUi(request) {
                val intent = Intent(Intent.ACTION_CREATE_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE)
                    .setType("application/json").putExtra(Intent.EXTRA_TITLE, "wuwaidmobile-diagnostics.json")
                startActivityForResult(invoke, intent, "documentReturned")
            }
        } else {
            dispatch(request) {
                val directory = File(activity.cacheDir, "diagnostics")
                check(directory.isDirectory || directory.mkdirs()) { "Cache diagnostik tidak dapat dibuat." }
                // Keep each granted URI immutable; remove only our reports older than 24 hours.
                val cutoff = System.currentTimeMillis() - 24 * 60 * 60 * 1000L
                directory.listFiles()?.forEach {
                    if (it.isFile && it.name.startsWith("wuwaidmobile-diagnostics-") &&
                        it.name.endsWith(".json") && it.lastModified() < cutoff) it.delete()
                }
                val file = File.createTempFile("wuwaidmobile-diagnostics-", ".json", directory)
                try {
                    file.outputStream().use { it.write(request.bytes) }
                    val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.fileprovider", file)
                    val send = Intent(Intent.ACTION_SEND).setType("application/json")
                        .putExtra(Intent.EXTRA_STREAM, uri).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                    send.clipData = ClipData.newRawUri("Diagnostik WuwaID Mobile", uri)
                    onUi(request) {
                        try {
                            activity.startActivity(Intent.createChooser(send, "Bagikan diagnostik"))
                            finish(request, "share_opened")
                        } catch (error: Exception) {
                            file.delete()
                            throw error
                        }
                    }
                } catch (error: Exception) {
                    file.delete()
                    throw error
                }
            }
        }
    }

    @ActivityCallback
    fun documentReturned(invoke: Invoke, result: ActivityResult) {
        val request = pending.get()?.takeIf { it.invoke === invoke } ?: return
        if (result.resultCode == Activity.RESULT_CANCELED) {
            finish(request, "cancelled")
            return
        }
        if (result.resultCode != Activity.RESULT_OK) {
            fail(request, "Dialog penyimpanan tidak selesai (${result.resultCode}).")
            return
        }
        val uri = result.data?.data
        if (uri == null || uri.scheme != "content") {
            fail(request, "Android tidak memberikan tujuan dokumen yang valid.")
            return
        }
        dispatch(request) {
            val stream = activity.contentResolver.openOutputStream(uri, "wt")
                ?: throw IllegalStateException("Tujuan dokumen tidak dapat dibuka.")
            stream.use { it.write(request.bytes); it.flush() }
            // Closing the stream is part of success; close/write exceptions resolve as failed.
            finish(request, "saved")
        }
    }

    private fun dispatch(request: Pending, action: () -> Unit) {
        try {
            worker.execute {
                if (pending.get() !== request) return@execute
                try { action() } catch (error: Exception) { fail(request, error.message ?: "Ekspor diagnostik gagal.") }
            }
        } catch (error: Exception) {
            fail(request, error.message ?: "Worker diagnostik tidak tersedia.")
        }
    }

    private fun onUi(request: Pending, action: () -> Unit) {
        activity.runOnUiThread {
            if (pending.get() !== request) return@runOnUiThread
            try { action() } catch (error: Exception) { fail(request, error.message ?: "Dialog Android tidak dapat dibuka.") }
        }
    }

    private fun finish(request: Pending, status: String, message: String? = null) {
        if (pending.compareAndSet(request, null)) {
            val result = JSONObject().put("status", status)
            if (message != null) result.put("message", message)
            request.invoke.resolveObject(result)
        }
    }

    private fun fail(request: Pending, message: String) = finish(request, "failed", message)

    override fun onDestroy(activity: AppCompatActivity) {
        pending.getAndSet(null)?.invoke?.reject("Aplikasi ditutup; hasil ekspor belum dapat dikonfirmasi.")
        worker.shutdownNow()
    }
}
