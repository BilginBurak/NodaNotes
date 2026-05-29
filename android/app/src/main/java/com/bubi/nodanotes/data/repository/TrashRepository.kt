package com.bubi.nodanotes.data.repository

import com.bubi.nodanotes.RustCore
import com.bubi.nodanotes.data.model.TrashEntryDto
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer

class TrashRepository : BaseRepository() {

    @Serializable
    private data class TrashParams(val note_id: String)

    suspend fun listTrash(): Result<List<TrashEntryDto>> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.listTrash("{}")
            parseRustResult(result, ListSerializer(TrashEntryDto.serializer()))
        }
    }

    suspend fun restoreFromTrash(noteId: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = TrashParams(noteId)
            val jsonInput = json.encodeToString(TrashParams.serializer(), params)
            val result = RustCore.restoreFromTrash(jsonInput)
            checkError(result)
        }
    }

    suspend fun permanentDelete(noteId: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = TrashParams(noteId)
            val jsonInput = json.encodeToString(TrashParams.serializer(), params)
            val result = RustCore.permanentDelete(jsonInput)
            checkError(result)
        }
    }

    suspend fun emptyTrash(): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.emptyTrash("{}")
            checkError(result)
        }
    }
}
