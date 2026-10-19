package com.titotfp.wuwaid.storage

import android.content.Intent
import android.os.Binder
import android.os.Bundle
import frb.axeron.shell.ShellServiceProvider
import org.junit.After
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.shadows.ShadowBinder
import org.robolectric.shadows.ShadowProcess

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [24, 28])
class AxManagerBoundaryTest {
  @After fun resetIdentity() {
    ShadowBinder.reset()
    ShadowProcess.reset()
  }

  @Test fun unprivilegedProcessCannotHostTheStorageService() {
    ShadowProcess.setUid(10123)
    val service = AxManagerStorageService()
    assertThrows(IllegalStateException::class.java) { service.onBind(Intent()) }
  }

  @Test fun foreignApplicationsCannotHandInAStorageBinder() {
    ShadowBinder.setCallingUid(10123)
    val extras = Bundle().apply {
      putString("token", "forged-token")
      putBinder("binder", Binder())
    }
    assertNull(ShellServiceProvider().call("attach", null, extras))
  }

  @Test fun privilegedCallerStillCannotAttachWithoutAnOutstandingSecretToken() {
    val provider = ShellServiceProvider()
    for (uid in listOf(0, 2000)) {
      ShadowBinder.setCallingUid(uid)
      assertNull(provider.call("attach", null, Bundle().apply {
        putString("token", "not-an-outstanding-bind")
        putBinder("binder", Binder())
      }))
      assertNull(provider.call("attach", null, Bundle().apply { putBinder("binder", Binder()) }))
    }
  }
}
