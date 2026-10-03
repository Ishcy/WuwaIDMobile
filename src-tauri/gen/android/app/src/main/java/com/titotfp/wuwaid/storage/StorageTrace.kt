package com.titotfp.wuwaid.storage

import android.os.SystemClock
import android.os.Trace
import android.util.Log
import com.titotfp.wuwaid.BuildConfig

internal object StorageTrace {
  fun mark(requestId: Long, phase: String, event: String) {
    if (!BuildConfig.DEBUG) return
    Log.d("WuwaIDTrace", "id=$requestId phase=$phase event=$event monoNs=${SystemClock.elapsedRealtimeNanos()}")
  }

  inline fun <T> phase(requestId: Long, name: String, block: () -> T): T {
    if (!BuildConfig.DEBUG) return block()
    mark(requestId, name, "begin")
    Trace.beginSection("wuwaid:$requestId:$name")
    try {
      return block()
    } finally {
      Trace.endSection()
      mark(requestId, name, "end")
    }
  }
}
