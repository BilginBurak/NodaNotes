package com.bubi.nodanotes.data.repository

import com.bubi.nodanotes.RustCore
import com.bubi.nodanotes.data.model.ConflictEntryDto
import com.bubi.nodanotes.data.model.NoteDto
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer

class ConflictRepository : BaseRepository() {

    @Serializable
    private data class GetConflictNoteParams(val archived_path: String)

    @Serializable
    private data class ResolveConflictParams(
        val note_id: String,
        val resolution: String
    )

    suspend fun listConflicts(): Result<List<ConflictEntryDto>> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.listConflicts("{}")
            parseRustResult(result, ListSerializer(ConflictEntryDto.serializer()))
        }
    }

    suspend fun getConflictNote(archivedPath: String): Result<NoteDto> = withContext(Dispatchers.IO) {
        runCatching {
            val params = GetConflictNoteParams(archivedPath)
            val jsonInput = json.encodeToString(GetConflictNoteParams.serializer(), params)
            val result = RustCore.getConflictNote(jsonInput)
            parseRustResult(result, NoteDto.serializer())
        }
    }

    suspend fun resolveConflict(noteId: String, resolution: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = ResolveConflictParams(noteId, resolution)
            val jsonInput = json.encodeToString(ResolveConflictParams.serializer(), params)
            val result = RustCore.resolveConflict(jsonInput)
            checkError(result)
        }
    }
}
