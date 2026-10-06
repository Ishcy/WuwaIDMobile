package com.titotfp.wuwaid

enum class LauncherStatus {
    NEEDS_SHIZUKU,
    GAME_NOT_READY,
    CONFLICT,
    NOT_INSTALLED,
    UPDATE_AVAILABLE,
    READY,
    BUSY,
    ERROR,
}

data class StateInputs(
    val shizukuAvailable: Boolean,
    val shizukuPermission: Boolean,
    val serviceReady: Boolean,
    val resourceVersion: String?,
    val conflicts: List<String>,
    val anyOwnedPatch: Boolean,
    val currentHealthy: Boolean,
    val releaseAvailable: Boolean,
    val matchesLatest: Boolean,
    val axManagerAvailable: Boolean = false,
    val axManagerPermission: Boolean = false,
    val axManagerReady: Boolean = false,
)

object LauncherStateResolver {
    fun resolve(inputs: StateInputs): LauncherStatus = when {
        !privilegedReady(inputs) -> LauncherStatus.NEEDS_SHIZUKU
        inputs.resourceVersion == null -> LauncherStatus.GAME_NOT_READY
        inputs.conflicts.isNotEmpty() -> LauncherStatus.CONFLICT
        !inputs.currentHealthy && inputs.anyOwnedPatch -> LauncherStatus.UPDATE_AVAILABLE
        !inputs.currentHealthy -> LauncherStatus.NOT_INSTALLED
        inputs.releaseAvailable && !inputs.matchesLatest -> LauncherStatus.UPDATE_AVAILABLE
        else -> LauncherStatus.READY
    }

    private fun privilegedReady(inputs: StateInputs): Boolean =
        inputs.serviceReady ||
            (inputs.axManagerAvailable && inputs.axManagerPermission && inputs.axManagerReady)
}
