package com.titotfp.wuwaid.storage

import android.app.Activity
import android.content.ComponentName
import android.content.Intent
import android.content.ServiceConnection
import android.content.pm.PackageManager
import android.os.Build
import android.os.IBinder
import android.os.Process
import frb.axeron.server.IAxeronService
import frb.axeron.shell.Broker
import frb.axeron.shell.ShellService
import java.util.concurrent.CountDownLatch
import java.util.concurrent.FutureTask
import java.util.concurrent.TimeUnit

internal fun isAxManagerCompatible(sdk: Int, userId: Int, appVersion: Long, serverVersion: Long? = null): Boolean =
  sdk >= 26 && userId == 0 && appVersion >= 20100 && (serverVersion == null || serverVersion >= 20100)

/** Independent of Shizuku's global binder. Only request() can prompt or bind privileged code. */
internal class AxManagerSession(private val activity: Activity) {
  private val lock = Any()
  private var closed = false
  private var started = false
  private var connection: ServiceConnection? = null
  private var ready: CountDownLatch? = null
  private var storage: IStorageUserService? = null
  private var server: IBinder? = null

  @Suppress("DEPRECATION")
  fun status(checkServer: Boolean = false): Map<String, Any> {
    val info = try {
      activity.packageManager.getPackageInfo(PACKAGE, 0)
    } catch (_: PackageManager.NameNotFoundException) {
      null
    }
    val versionCode = info?.let { if (Build.VERSION.SDK_INT >= 28) it.longVersionCode else it.versionCode.toLong() } ?: 0L
    val userId = Process.myUid() / 100000
    val capable = isAxManagerCompatible(Build.VERSION.SDK_INT, userId, versionCode)
    val current = if (capable && checkServer) runCatching { remote() }.getOrNull() else null
    val serverInfo = current?.let { runCatching { it.serverInfo }.getOrNull() }
    // Updating the manager APK does not update an already-running server.
    val supported = capable && (current == null || (serverInfo != null &&
      isAxManagerCompatible(Build.VERSION.SDK_INT, userId, versionCode, serverInfo.versionCode) &&
      (serverInfo.uid == 0 || serverInfo.uid == 2000)))
    val running = current?.asBinder()?.pingBinder() == true
    val version = info?.versionName ?: ""
    return mapOf("installed" to (info != null), "supported" to supported, "running" to running,
      "version" to if (serverInfo != null && serverInfo.versionCode != versionCode)
        "$version / server ${serverInfo.version}" else version)
  }

  private fun remote(): IAxeronService? {
    synchronized(lock) {
      check(!closed) { "AxManager storage session was destroyed" }
      if (!started) {
        Broker.start(activity)
        started = true
      }
    }
    return Broker.service()
  }

  // Never infer a grant from Shizuku, a cached permission callback, or a live storage binder alone.
  fun granted(): Boolean = runCatching { connected(); true }.getOrDefault(false)

  fun connected(): IStorageUserService {
    check(Build.VERSION.SDK_INT >= 26 && Process.myUid() / 100000 == 0) { "AxManager memerlukan Android 8+ dan user utama." }
    val current = remote() ?: throw IllegalStateException("AxManager belum berjalan. Aktifkan server di AxManager.")
    check(current.checkSelfPermission()) { "Izin AxManager belum diberikan atau telah dicabut." }
    return synchronized(lock) {
      check(!closed && server === current.asBinder()) { "Server AxManager berubah. Hubungkan ulang melalui tombol izin." }
      storage?.takeIf { it.asBinder().isBinderAlive }
        ?: throw IllegalStateException("Storage AxManager belum terhubung. Tekan Berikan izin AxManager.")
    }
  }

  fun request(requestId: Long) {
    val state = status(checkServer = true)
    check(state["installed"] == true) { "Pasang AxManager 2.0.1 atau lebih baru terlebih dahulu." }
    check(state["supported"] == true) { "AxManager memerlukan Android 8+, user utama, dan AxManager 2.0.1 atau lebih baru." }
    if (granted()) return
    val current = remote() ?: throw IllegalStateException("Aktifkan server AxManager, lalu coba lagi.")
    StorageTrace.phase(requestId, "permission.axmanager") {
      if (!current.checkSelfPermission()) {
        check(Broker.awaitPermission(current) && current.checkSelfPermission()) {
          "Izin AxManager ditolak atau belum dijawab. Periksa halaman Privilege di AxManager."
        }
      }
      val latch = CountDownLatch(1)
      var failure: String? = null
      val conn = object : ServiceConnection {
        override fun onServiceConnected(name: ComponentName, binder: IBinder) {
          val accepted = synchronized(lock) {
            if (!closed && connection === this) {
              storage = IStorageUserService.Stub.asInterface(binder)
              server = current.asBinder()
              ready = null
              latch.countDown()
              true
            } else false
          }
          if (!accepted) activity.runOnUiThread { ShellService.unbind(this) }
        }

        override fun onServiceDisconnected(name: ComponentName) {
          synchronized(lock) {
            if (connection === this) {
              storage = null
              server = null
            }
            latch.countDown()
          }
        }

        override fun onBindingDied(name: ComponentName) {
          synchronized(lock) {
            failure = ShellService.lastFailure(name) ?: "Service AxManager terputus saat menghubungkan."
          }
          onServiceDisconnected(name)
        }

        override fun onNullBinding(name: ComponentName) = onBindingDied(name)
      }
      val previous = synchronized(lock) {
        check(!closed) { "AxManager storage session was destroyed" }
        val old = connection
        connection = conn
        storage = null
        server = null
        ready = latch
        old
      }
      try {
        onUi {
          synchronized(lock) {
            check(!closed && connection === conn) { "AxManager storage binding was cancelled" }
            previous?.let { ShellService.unbind(it) }
            ShellService.bind(Intent(activity, AxManagerStorageService::class.java), conn)
          }
        }
        check(latch.await(20, TimeUnit.SECONDS)) { "Storage AxManager tidak tersambung dalam 20 detik. Coba lagi." }
        synchronized(lock) { failure }?.let { throw IllegalStateException(it) }
        connected()
      } catch (error: Exception) {
        synchronized(lock) {
          if (connection === conn) {
            connection = null
            storage = null
            server = null
            ready?.countDown()
            ready = null
          }
        }
        activity.runOnUiThread { ShellService.unbind(conn) }
        throw error
      }
    }
  }

  // Used on method changes and destroy; reopening the app requires explicit reconnection.
  fun disconnect() {
    val conn = synchronized(lock) {
      val old = connection
      connection = null
      storage = null
      server = null
      ready?.countDown()
      ready = null
      old
    }
    conn?.let { activity.runOnUiThread { ShellService.unbind(it) } }
  }

  fun close() {
    synchronized(lock) { closed = true }
    disconnect()
    if (started) Broker.stop()
  }

  private fun <T> onUi(action: () -> T): T {
    val task = FutureTask(action)
    activity.runOnUiThread(task)
    return task.get(15, TimeUnit.SECONDS)
  }

  companion object {
    const val PACKAGE = "frb.axeron.manager"
  }
}
