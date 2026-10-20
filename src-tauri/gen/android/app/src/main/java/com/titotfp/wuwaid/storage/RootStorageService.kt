package com.titotfp.wuwaid.storage

import android.content.Intent
import android.os.IBinder
import android.os.Process
import com.topjohnwu.superuser.ipc.RootService

// libsu launches this class in a non-daemon root process, not an exported Android Service.
class RootStorageService : RootService() {
  private var storage: StorageUserService? = null

  override fun onBind(intent: Intent): IBinder {
    check(Process.myUid() == 0) { "Storage root service did not obtain UID 0" }
    return storage ?: StorageUserService().also { storage = it }
  }

  override fun onDestroy() {
    storage?.close()
  }
}
