package com.bubi.nodanotes.data.repository

import com.bubi.nodanotes.RustCore
import com.bubi.nodanotes.data.model.DuplicateNoteGroupDto
import com.bubi.nodanotes.data.model.OrphanedRemnantsDto
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer

class DiagnosticsRepository : BaseRepository() {

    @Serializable
    private data class DeleteDuplicateFileParams(val file_path: String)

    @Serializable
    private data class DeleteOrphanedFileParams(val file_path: String)

    @Serializable
    data class OrphanedAttachmentDto(
        val name: String,
        val size_bytes: Long
    )

    suspend fun rebuildCache(): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.rebuildCache("{}")
            checkError(result)
        }
    }

    suspend fun optimizeFts(): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.optimizeFts("{}")
            checkError(result)
        }
    }

    suspend fun getDuplicateNotes(): Result<List<DuplicateNoteGroupDto>> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.getDuplicateNotes("{}")
            parseRustResult(result, ListSerializer(DuplicateNoteGroupDto.serializer()))
        }
    }

    suspend fun deleteDuplicateFile(filePath: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = DeleteDuplicateFileParams(filePath)
            val jsonInput = json.encodeToString(DeleteDuplicateFileParams.serializer(), params)
            val result = RustCore.deleteDuplicateFile(jsonInput)
            checkError(result)
        }
    }

    suspend fun getOrphanedRemnants(): Result<OrphanedRemnantsDto> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.getOrphanedRemnants("{}")
            parseRustResult(result, OrphanedRemnantsDto.serializer())
        }
    }

    suspend fun deleteOrphanedFile(filePath: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = DeleteOrphanedFileParams(filePath)
            val jsonInput = json.encodeToString(DeleteOrphanedFileParams.serializer(), params)
            val result = RustCore.deleteOrphanedFile(jsonInput)
            checkError(result)
        }
    }

    suspend fun deleteAllOrphanedRemnants(): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.deleteAllOrphanedRemnants("{}")
            checkError(result)
        }
    }

    suspend fun getOrphanedAttachments(): Result<List<OrphanedAttachmentDto>> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.getOrphanedAttachments("{}")
            parseRustResult(result, ListSerializer(OrphanedAttachmentDto.serializer()))
        }
    }

    suspend fun clearRemoteTrackingCache(): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.clearRemoteTrackingCache("{}")
            checkError(result)
        }
    }

    suspend fun resetSyncQueue(): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.resetSyncQueue("{}")
            checkError(result)
        }
    }
}
