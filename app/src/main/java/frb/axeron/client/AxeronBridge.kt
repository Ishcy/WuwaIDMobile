package frb.axeron.client

import android.content.Context
import android.content.pm.PackageManager
import android.os.Binder
import android.os.Bundle
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.Parcel
import android.util.Log
import frb.axeron.server.IAxeronService
import java.util.concurrent.CopyOnWriteArrayList

/**
 * Minimal Axeron IPC client.
 *
 * AxManager's Axeron service exposes a Shizuku-compatible application binder.
 * We intentionally keep this bridge small: only application attachment,
 * permission state and the remote file service are required by WuwaID.
 */
internal object AxeronBridge {
    private const val TAG = "WuwaID-Axeron"
    private const val SERVICE_DESCRIPTOR = "frb.axeron.server.IAxeronService"
    private const val APPLICATION_DESCRIPTOR = "moe.shizuku.server.IShizukuApplication"
    private const val TRANSACTION_ATTACH_APPLICATION = 14

    private const val PERMISSION_GRANTED = "shizuku:attach-reply-permission-granted"
    private const val REQUEST_ALLOWED = "shizuku:request-permission-reply-allowed"

    private val main = Handler(Looper.getMainLooper())
    private val listeners = CopyOnWriteArrayList<() -> Unit>()

    @Volatile private var binder: IBinder? = null
    @Volatile private var service: IAxeronService? = null
    @Volatile private var permissionGranted = false

    private val applicationBinder = object : Binder() {
        init { attachInterface(null, APPLICATION_DESCRIPTOR) }

        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            return try {
                when (code) {
                    IBinder.INTERFACE_TRANSACTION -> {
                        reply?.writeString(APPLICATION_DESCRIPTOR)
                        true
                    }
                    1 -> {
                        data.enforceInterface(APPLICATION_DESCRIPTOR)
                        val bundle = data.readBundle(javaClass.classLoader) ?: Bundle()
                        permissionGranted = bundle.getBoolean(PERMISSION_GRANTED, false)
                        notifyListeners()
                        true
                    }
                    2 -> {
                        data.enforceInterface(APPLICATION_DESCRIPTOR)
                        data.readInt() // requestCode
                        val bundle = data.readBundle(javaClass.classLoader) ?: Bundle()
                        permissionGranted = bundle.getBoolean(REQUEST_ALLOWED, false)
                        notifyListeners()
                        true
                    }
                    3 -> {
                        data.enforceInterface(APPLICATION_DESCRIPTOR)
                        data.readInt()
                        data.readInt()
                        data.readString()
                        data.readInt()
                        true
                    }
                    else -> super.onTransact(code, data, reply, flags)
                }
            } catch (error: Throwable) {
                Log.e(TAG, "application binder callback gagal", error)
                false
            }
        }
    }

    fun start(listener: () -> Unit) {
        listeners.add(listener)
        listener()
    }

    fun stop(listener: () -> Unit) {
        listeners.remove(listener)
    }

    fun attachBinder(context: Context, newBinder: IBinder) {
        if (!newBinder.pingBinder()) return
        if (binder === newBinder && service != null) {
            notifyListeners()
            return
        }

        binder?.let { old -> runCatching { old.unlinkToDeath(deathRecipient, 0) } }
        binder = newBinder
        service = IAxeronService.Stub.asInterface(newBinder)
        permissionGranted = false

        runCatching { newBinder.linkToDeath(deathRecipient, 0) }
        runCatching {
            val data = Parcel.obtain()
            val reply = Parcel.obtain()
            try {
                data.writeInterfaceToken(SERVICE_DESCRIPTOR)
                data.writeStrongBinder(applicationBinder)
                // Axeron's current client API uses the legacy Shizuku-compatible
                // transaction 14 with a package-name String.
                data.writeString(context.packageName)
                check(newBinder.transact(TRANSACTION_ATTACH_APPLICATION, data, reply, 0)) {
                    "Axeron attachApplication ditolak"
                }
                reply.readException()
            } finally {
                reply.recycle()
                data.recycle()
            }
        }.onFailure {
            Log.e(TAG, "attachApplication gagal", it)
        }
        notifyListeners()
    }

    fun binder(): IBinder? = binder

    fun isAvailable(): Boolean =
        runCatching { binder?.pingBinder() == true && service != null }.getOrDefault(false)

    fun hasPermission(): Boolean {
        if (!isAvailable()) return false
        if (permissionGranted) return true
        permissionGranted = runCatching {
            service?.checkSelfPermission() == true
        }.getOrDefault(false)
        return permissionGranted
    }

    fun requestPermission(requestCode: Int = 10001) {
        runCatching { service?.requestPermission(requestCode) }
            .onFailure { Log.e(TAG, "requestPermission gagal", it) }
    }

    fun fileService(): frb.axeron.server.IFileService? =
        runCatching { service?.getFileService() }.getOrNull()

    private val deathRecipient = IBinder.DeathRecipient {
        binder = null
        service = null
        permissionGranted = false
        notifyListeners()
    }

    private fun notifyListeners() {
        main.post { listeners.forEach { runCatching { it() } } }
    }
}
