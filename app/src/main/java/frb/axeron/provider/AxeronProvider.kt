package frb.axeron.provider

import android.content.ContentProvider
import android.content.ContentValues
import android.database.Cursor
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.IBinder
import frb.axeron.client.AxeronBridge
import moe.shizuku.api.BinderContainer

/**
 * Compatibility provider used by AxManager to deliver its privileged binder.
 *
 * AxManager's AX-Permission Intercept mode sends a BinderContainer through
 * the standard Axeron provider contract. This provider intentionally exposes
 * only the binder hand-off methods; file operations stay behind the AIDL service.
 */
class AxeronProvider : ContentProvider() {
    companion object {
        const val METHOD_SEND_BINDER = "sendBinder"
        const val METHOD_GET_BINDER = "getBinder"
        const val ACTION_BINDER_RECEIVED = "AxServer.BINDER_RECEIVED"
        const val EXTRA_BINDER = "AxServer.BINDER"
    }

    override fun attachInfo(context: android.content.Context, info: android.content.pm.ProviderInfo) {
        super.attachInfo(context, info)
        check(!info.multiprocess) { "AxeronProvider must not be multiprocess" }
        check(info.exported) { "AxeronProvider must be exported" }
    }

    override fun onCreate(): Boolean = true

    override fun call(method: String, arg: String?, extras: Bundle?): Bundle? {
        if (extras == null) return null
        extras.classLoader = BinderContainer::class.java.classLoader
        val reply = Bundle()
        when (method) {
            METHOD_SEND_BINDER -> {
                val container = if (Build.VERSION.SDK_INT >= 33) {
                    extras.getParcelable(EXTRA_BINDER, BinderContainer::class.java)
                } else {
                    @Suppress("DEPRECATION")
                    extras.getParcelable(EXTRA_BINDER)
                }
                val binder = container?.binder
                if (binder != null) {
                    AxeronBridge.attachBinder(requireNotNull(context), binder)
                    context!!.sendBroadcast(
                        IntentCompat.packageBroadcast(context!!, ACTION_BINDER_RECEIVED)
                    )
                }
            }
            METHOD_GET_BINDER -> {
                val binder = AxeronBridge.binder()
                if (binder == null || !binder.pingBinder()) return null
                reply.putParcelable(EXTRA_BINDER, BinderContainer(binder))
            }
        }
        return reply
    }

    override fun query(uri: Uri, projection: Array<String>?, selection: String?, selectionArgs: Array<String>?, sortOrder: String?): Cursor? = null
    override fun getType(uri: Uri): String? = null
    override fun insert(uri: Uri, values: ContentValues?): Uri? = null
    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<String>?): Int = 0
    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<String>?): Int = 0
}

private object IntentCompat {
    fun packageBroadcast(context: android.content.Context, action: String): android.content.Intent =
        android.content.Intent(action).setPackage(context.packageName)
}
