package com.bubi.nodanotes.data.repository

import com.bubi.nodanotes.RustCore
import com.bubi.nodanotes.data.model.SnapshotDiffDto
import com.bubi.nodanotes.data.model.SnapshotDto
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer

class HistoryRepository : BaseRepository() {

    @Serializable
    private data class ListSnapshotsParams(val note_id: String)

    @Serializable
    private data class SnapshotParams(
        val note_id: String,
        val timestamp: String
    )

    suspend fun listSnapshots(noteId: String): Result<List<SnapshotDto>> = withContext(Dispatchers.IO) {
        runCatching {
            val params = ListSnapshotsParams(noteId)
            val jsonInput = json.encodeToString(ListSnapshotsParams.serializer(), params)
            val result = RustCore.listSnapshots(jsonInput)
            parseRustResult(result, ListSerializer(SnapshotDto.serializer()))
        }
    }

    suspend fun getSnapshotDiff(noteId: String, timestamp: String): Result<SnapshotDiffDto> = withContext(Dispatchers.IO) {
        runCatching {
            val params = SnapshotParams(noteId, timestamp)
            val jsonInput = json.encodeToString(SnapshotParams.serializer(), params)
            val result = RustCore.getSnapshotDiff(jsonInput)
            parseRustResult(result, SnapshotDiffDto.serializer())
        }
    }

    suspend fun restoreSnapshot(noteId: String, timestamp: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = SnapshotParams(noteId, timestamp)
            val jsonInput = json.encodeToString(SnapshotParams.serializer(), params)
            val result = RustCore.restoreSnapshot(jsonInput)
            checkError(result)
        }
    }

    suspend fun deleteSnapshot(noteId: String, timestamp: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = SnapshotParams(noteId, timestamp)
            val jsonInput = json.encodeToString(SnapshotParams.serializer(), params)
            val result = RustCore.deleteSnapshot(jsonInput)
            checkError(result)
        }
    }
}
