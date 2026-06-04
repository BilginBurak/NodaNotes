package com.bubi.nodanotes.data.repository

import com.bubi.nodanotes.RustCore
import com.bubi.nodanotes.data.model.NoteDto
import com.bubi.nodanotes.data.model.NoteListItemDto
import com.bubi.nodanotes.data.model.NoteMetadataDto
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.builtins.serializer

class NoteRepository : BaseRepository() {

    @Serializable
    private data class ListNotesParams(val folder_path: String?)

    @Serializable
    private data class GetNoteParams(val note_id: String)

    @Serializable
    private data class CreateNoteParams(
        val title: String,
        val parent_folder: String?,
        val tags: List<String>
    )

    @Serializable
    private data class UpdateNoteParams(
        val note_id: String,
        val title: String,
        val body: String,
        val tags: List<String>,
        val color: String?,
        val pinned: Boolean,
        val trigger_snapshot: Boolean? = null,
        val snapshot_reason: String? = null
    )

    @Serializable
    private data class RenameNoteParams(
        val note_id: String,
        val new_title: String
    )

    @Serializable
    private data class DeleteNoteParams(
        val note_id: String
    )

    @Serializable
    private data class MoveNoteParams(
        val note_id: String,
        val target_folder: String
    )

    @Serializable
    private data class ToggleTaskStatusParams(
        val note_id: String,
        val line_content: String
    )

    suspend fun listNotes(folderPath: String?): Result<List<NoteListItemDto>> = withContext(Dispatchers.IO) {
        runCatching {
            val params = ListNotesParams(folderPath)
            val jsonInput = json.encodeToString(ListNotesParams.serializer(), params)
            val result = RustCore.listNotes(jsonInput)
            parseRustResult(result, ListSerializer(NoteListItemDto.serializer()))
        }
    }

    suspend fun getAllNotes(): Result<List<NoteListItemDto>> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.getAllNotes("{}")
            parseRustResult(result, ListSerializer(NoteListItemDto.serializer()))
        }
    }

    suspend fun getNote(noteId: String): Result<NoteDto> = withContext(Dispatchers.IO) {
        runCatching {
            val params = GetNoteParams(noteId)
            val jsonInput = json.encodeToString(GetNoteParams.serializer(), params)
            val result = RustCore.getNote(jsonInput)
            parseRustResult(result, NoteDto.serializer())
        }
    }

    suspend fun createNote(title: String, parentFolder: String?, tags: List<String>): Result<NoteDto> = withContext(Dispatchers.IO) {
        runCatching {
            val params = CreateNoteParams(title, parentFolder, tags)
            val jsonInput = json.encodeToString(CreateNoteParams.serializer(), params)
            val result = RustCore.createNote(jsonInput)
            parseRustResult(result, NoteDto.serializer())
        }
    }

    suspend fun updateNote(
        noteId: String,
        title: String,
        body: String,
        tags: List<String>,
        color: String?,
        pinned: Boolean,
        triggerSnapshot: Boolean? = null,
        snapshotReason: String? = null
    ): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = UpdateNoteParams(noteId, title, body, tags, color, pinned, triggerSnapshot, snapshotReason)
            val jsonInput = json.encodeToString(UpdateNoteParams.serializer(), params)
            val result = RustCore.updateNote(jsonInput)
            checkError(result)
        }
    }

    suspend fun renameNote(noteId: String, newTitle: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = RenameNoteParams(noteId, newTitle)
            val jsonInput = json.encodeToString(RenameNoteParams.serializer(), params)
            val result = RustCore.renameNote(jsonInput)
            checkError(result)
        }
    }

    suspend fun deleteNote(noteId: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = DeleteNoteParams(noteId)
            val jsonInput = json.encodeToString(DeleteNoteParams.serializer(), params)
            val result = RustCore.deleteNote(jsonInput)
            checkError(result)
        }
    }

    suspend fun moveNote(noteId: String, targetFolder: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = MoveNoteParams(noteId, targetFolder)
            val jsonInput = json.encodeToString(MoveNoteParams.serializer(), params)
            val result = RustCore.moveNote(jsonInput)
            checkError(result)
        }
    }

    suspend fun getNoteMetadata(noteId: String): Result<NoteMetadataDto> = withContext(Dispatchers.IO) {
        runCatching {
            val params = GetNoteParams(noteId)
            val jsonInput = json.encodeToString(GetNoteParams.serializer(), params)
            val result = RustCore.getNoteMetadata(jsonInput)
            parseRustResult(result, NoteMetadataDto.serializer())
        }
    }

    suspend fun getAllTags(): Result<List<String>> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.getAllTags("{}")
            parseRustResult(result, ListSerializer(String.serializer()))
        }
    }

    @Serializable
    private data class TriggerDailyNoteParams(val date: String?)

    suspend fun triggerDailyNote(date: String? = null): Result<NoteDto> = withContext(Dispatchers.IO) {
        runCatching {
            val params = TriggerDailyNoteParams(date)
            val jsonInput = json.encodeToString(TriggerDailyNoteParams.serializer(), params)
            val result = RustCore.triggerDailyNote(jsonInput)
            parseRustResult(result, NoteDto.serializer())
        }
    }

    suspend fun toggleTaskStatus(noteId: String, lineContent: String): Result<NoteDto> = withContext(Dispatchers.IO) {
        runCatching {
            val params = ToggleTaskStatusParams(noteId, lineContent)
            val jsonInput = json.encodeToString(ToggleTaskStatusParams.serializer(), params)
            val result = RustCore.toggleTaskStatus(jsonInput)
            parseRustResult(result, NoteDto.serializer())
        }
    }

    suspend fun listTagsWithCounts(): Result<List<com.bubi.nodanotes.data.model.TagWithCountDto>> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.listTagsWithCounts("{}")
            parseRustResult(result, ListSerializer(com.bubi.nodanotes.data.model.TagWithCountDto.serializer()))
        }
    }
}
