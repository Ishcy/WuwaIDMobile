package com.titotfp.wuwaid.storage

import android.content.Context
import android.system.ErrnoException
import android.system.Os
import android.system.OsConstants
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ResourceFilesTest {
  private fun withFixture(block: (ResourceFiles, File) -> Unit) {
    val context = ApplicationProvider.getApplicationContext<Context>()
    val game = File(context.cacheDir, "resource-files-${System.nanoTime()}")
    val root = File(game, "UE4Game/Client/Client/Saved/Resources")
    check(root.mkdirs())
    try {
      block(ResourceFiles(game), File(root, "payload"))
    } finally {
      game.deleteRecursively()
    }
  }

  private fun openDescriptors(file: File): List<String> {
    val target = file.canonicalPath
    return checkNotNull(File("/proc/self/fd").list()).filter { name ->
      try {
        Os.readlink("/proc/self/fd/$name") == target
      } catch (error: ErrnoException) {
        if (error.errno != OsConstants.ENOENT) throw error
        false
      }
    }
  }

  @Test
  fun repeatedReadsAndHashesReleaseTheirDescriptors() = withFixture { files, payload ->
    payload.writeText("hello")
    repeat(32) {
      assertEquals("hello", String(files.metadata("read_text", "payload", null), Charsets.UTF_8))
      val hashes = JSONObject(String(files.metadata("hashes", "payload", "both"), Charsets.UTF_8))
      assertEquals(5L, hashes.getLong("size"))
      assertEquals("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824", hashes.getString("sha256"))
      assertEquals("aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d", hashes.getString("sha1"))
    }
    assertEquals(emptyList<String>(), openDescriptors(payload))
  }

  @Test
  fun rejectedOversizedReadsReleaseTheirDescriptors() = withFixture { files, payload ->
    payload.writeBytes(ByteArray(256 * 1024 + 1))
    repeat(32) {
      assertThrows(IllegalArgumentException::class.java) {
        files.metadata("read_text", "payload", null)
      }
    }
    assertEquals(emptyList<String>(), openDescriptors(payload))
  }
}
