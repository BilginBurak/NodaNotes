package com.bubi.nodanotes.data.repository

import com.bubi.nodanotes.RustCore
import com.bubi.nodanotes.data.model.SyncReportDto
import com.bubi.nodanotes.data.model.SyncStatusDto
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable

class SyncRepository : BaseRepository() {

    @Serializable
    private data class SaveSyncConfigParams(
        val webdav_url: String,
        val username: String,
        val password: String?,
        val interval_secs: Long
    )

    @Serializable
    data class LoadSyncConfigResult(
        val webdav_url: String,
        val username: String,
        val interval_secs: Long,
        val is_configured: Boolean
    )

    @Serializable
    private data class TestWebdavConnectionParams(
        val webdav_url: String,
        val username: String,
        val password: String?
    )

    suspend fun saveSyncConfig(
        webdavUrl: String,
        username: String,
        password: String?,
        intervalSecs: Long
    ): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = SaveSyncConfigParams(webdavUrl, username, password, intervalSecs)
            val jsonInput = json.encodeToString(SaveSyncConfigParams.serializer(), params)
            val result = RustCore.saveSyncConfig(jsonInput)
            checkError(result)
        }
    }

    suspend fun loadSyncConfig(): Result<LoadSyncConfigResult> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.loadSyncConfig("{}")
            parseRustResult(result, LoadSyncConfigResult.serializer())
        }
    }

    suspend fun testWebdavConnection(
        webdavUrl: String,
        username: String,
        password: String?
    ): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = TestWebdavConnectionParams(webdavUrl, username, password)
            val jsonInput = json.encodeToString(TestWebdavConnectionParams.serializer(), params)
            val result = RustCore.testWebdavConnection(jsonInput)
            checkError(result)
        }
    }

    suspend fun syncNow(): Result<SyncReportDto> = withContext(Dispatchers.IO) {
        runCatching {
            ActiveNoteTracker.triggerPreSyncSave()
            val result = RustCore.syncNow("{}")
            parseRustResult(result, SyncReportDto.serializer())
        }
    }

    suspend fun getSyncStatus(): Result<SyncStatusDto> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.getSyncStatus("{}")
            parseRustResult(result, SyncStatusDto.serializer())
        }
    }
}

object ActiveNoteTracker {
    var activeSaveAction: (suspend (String) -> Unit)? = null

    suspend fun triggerPreSyncSave() {
        activeSaveAction?.invoke("Pre-Sync")
    }
}
