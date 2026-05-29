package com.bubi.nodanotes.data.repository

import com.bubi.nodanotes.RustCore
import com.bubi.nodanotes.data.model.FolderDto
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer

class FolderRepository : BaseRepository() {

    @Serializable
    private data class CreateFolderParams(
        val parent_path: String?,
        val name: String
    )

    @Serializable
    private data class RenameFolderParams(
        val folder_path: String,
        val new_name: String
    )

    @Serializable
    private data class DeleteFolderParams(
        val folder_path: String
    )

    @Serializable
    private data class MoveFolderParams(
        val source_path: String,
        val target_parent: String
    )

    suspend fun listFolders(): Result<List<FolderDto>> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.listFolders("{}")
            parseRustResult(result, ListSerializer(FolderDto.serializer()))
        }
    }

    suspend fun createFolder(parentPath: String?, name: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = CreateFolderParams(parentPath, name)
            val jsonInput = json.encodeToString(CreateFolderParams.serializer(), params)
            val result = RustCore.createFolder(jsonInput)
            checkError(result)
        }
    }

    suspend fun renameFolder(folderPath: String, newName: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = RenameFolderParams(folderPath, newName)
            val jsonInput = json.encodeToString(RenameFolderParams.serializer(), params)
            val result = RustCore.renameFolder(jsonInput)
            checkError(result)
        }
    }

    suspend fun deleteFolder(folderPath: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = DeleteFolderParams(folderPath)
            val jsonInput = json.encodeToString(DeleteFolderParams.serializer(), params)
            val result = RustCore.deleteFolder(jsonInput)
            checkError(result)
        }
    }

    suspend fun moveFolder(sourcePath: String, targetParent: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = MoveFolderParams(sourcePath, targetParent)
            val jsonInput = json.encodeToString(MoveFolderParams.serializer(), params)
            val result = RustCore.moveFolder(jsonInput)
            checkError(result)
        }
    }
}
