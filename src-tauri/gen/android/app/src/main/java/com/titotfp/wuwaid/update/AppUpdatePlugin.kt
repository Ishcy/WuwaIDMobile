package com.titotfp.wuwaid.update

import android.app.Activity
import android.content.ClipData
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.activity.result.ActivityResult
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.FileProvider
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Channel
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import org.json.JSONObject
import java.io.File
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicReference

@InvokeArg
class AppUpdateArgs {
  var action: String = "status"
  var progress: Channel? = null
}

@TauriPlugin
class AppUpdatePlugin internal constructor(
  private val activity: Activity,
  private val repository: AppUpdateRepository,
) : Plugin(activity) {
  constructor(activity: Activity) : this(activity, AppUpdateRepository(activity))
  private val worker = Executors.newSingleThreadExecutor { runnable ->
    Thread(runnable, "wuwaid-app-update").apply { isDaemon = true }
  }
  private val pending = AtomicReference<Invoke?>(null)
  @Volatile private var progress: Channel? = null

  @Command
  fun update(invoke: Invoke) {
    val args = try {
      invoke.parseArgs(AppUpdateArgs::class.java)
    } catch (error: Exception) {
      invoke.reject("Permintaan updater tidak valid: ${error.message}")
      return
    }
    if (!pending.compareAndSet(null, invoke)) {
      invoke.reject("Operasi updater aplikasi masih berjalan.")
      return
    }
    progress = args.progress
    dispatch(invoke) {
      when (args.action) {
        "status" -> finish(invoke, repository.snapshot())
        "check" -> finish(invoke, repository.check())
        "download" -> finish(invoke, repository.download(::report))
        "install" -> {
          report(100, "Memverifikasi APK sebelum pemasangan…")
          val apk = repository.verifyReady()
          onUi(invoke) {
            if (Build.VERSION.SDK_INT >= 26 && !activity.packageManager.canRequestPackageInstalls()) {
              report(100, "Izinkan pemasangan dari WuwaID Mobile, lalu kembali ke aplikasi.")
              startActivityForResult(invoke, Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                Uri.parse("package:${activity.packageName}")), "permissionReturned")
            } else {
              launchInstaller(invoke, apk)
            }
          }
        }
        else -> throw IllegalArgumentException("Tindakan updater tidak dikenal.")
      }
    }
  }

  @ActivityCallback
  fun permissionReturned(invoke: Invoke, result: ActivityResult) {
    // Settings activities do not reliably report RESULT_OK: inspect the actual grant.
    if (Build.VERSION.SDK_INT >= 26 && !activity.packageManager.canRequestPackageInstalls()) {
      fail(invoke, "Izin pemasangan belum diberikan. APK tetap tersimpan; izinkan sumber ini saat mencoba Pasang update lagi.")
      return
    }
    dispatch(invoke) {
      val apk = repository.verifyReady()
      onUi(invoke) { launchInstaller(invoke, apk) }
    }
  }

  private fun launchInstaller(invoke: Invoke, apk: File) {
    val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.fileprovider", apk)
    val intent = Intent(Intent.ACTION_INSTALL_PACKAGE).setDataAndType(uri, "application/vnd.android.package-archive")
      .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
      .putExtra(Intent.EXTRA_RETURN_RESULT, true)
    intent.clipData = ClipData.newRawUri("APK update WuwaID Mobile", uri)
    report(100, "Konfirmasikan pemasangan update pada layar Android.")
    startActivityForResult(invoke, intent, "installerReturned")
  }

  @ActivityCallback
  fun installerReturned(invoke: Invoke, result: ActivityResult) {
    dispatch(invoke) {
      val state = repository.snapshot()
      state.put("message", when (result.resultCode) {
        Activity.RESULT_OK -> "Pemasang Android selesai. Jika aplikasi belum dimulai ulang, tutup dan buka kembali untuk membaca versi terpasang."
        Activity.RESULT_CANCELED -> "Pemasangan dibatalkan atau belum selesai. APK tetap siap dipasang lagi."
        else -> "Android tidak menyelesaikan pemasangan update. APK tetap tersimpan; periksa ruang kosong dan persyaratan perangkat."
      })
      finish(invoke, state)
    }
  }

  private fun report(percent: Int, label: String) {
    val channel = progress ?: return
    val event = JSObject()
    event.put("percent", percent)
    event.put("label", label)
    channel.send(event)
  }

  private fun dispatch(invoke: Invoke, action: () -> Unit) {
    try {
      worker.execute {
        if (pending.get() !== invoke) return@execute
        try {
          action()
        } catch (error: Exception) {
          fail(invoke, error.message ?: "Operasi updater gagal.")
        }
      }
    } catch (error: Exception) {
      fail(invoke, "Worker updater tidak tersedia: ${error.message}")
    }
  }

  private fun onUi(invoke: Invoke, action: () -> Unit) {
    activity.runOnUiThread {
      if (pending.get() !== invoke) return@runOnUiThread
      try {
        action()
      } catch (error: Exception) {
        fail(invoke, "Tidak dapat membuka pemasang atau pengaturan Android: ${error.message}")
      }
    }
  }

  private fun finish(invoke: Invoke, state: JSONObject) {
    if (pending.compareAndSet(invoke, null)) {
      progress = null
      invoke.resolveObject(state)
    }
  }

  private fun fail(invoke: Invoke, message: String) {
    if (pending.compareAndSet(invoke, null)) {
      progress = null
      invoke.reject(message)
    }
  }

  override fun onDestroy(activity: AppCompatActivity) {
    pending.getAndSet(null)?.reject("Updater dihentikan karena aplikasi ditutup. Periksa pembaruan lagi setelah membuka aplikasi.")
    progress = null
    worker.shutdownNow()
  }
}
