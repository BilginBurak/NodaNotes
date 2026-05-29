package com.bubi.nodanotes.data.repository

import com.bubi.nodanotes.RustCore
import com.bubi.nodanotes.data.model.AttachmentInfoDto
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer

class AttachmentRepository : BaseRepository() {

    @Serializable
    private data class AddAttachmentParams(val source_path: String)

    @Serializable
    data class AddAttachmentResult(
        val attachment_name: String,
        val markdown_link: String
    )

    @Serializable
    private data class GetAttachmentDataParams(val attachment_name: String)

    @Serializable
    data class AttachmentDataDto(
        val data_base64: String,
        val mime_type: String
    )

    suspend fun addAttachment(sourcePath: String): Result<AddAttachmentResult> = withContext(Dispatchers.IO) {
        runCatching {
            val params = AddAttachmentParams(sourcePath)
            val jsonInput = json.encodeToString(AddAttachmentParams.serializer(), params)
            val result = RustCore.addAttachment(jsonInput)
            parseRustResult(result, AddAttachmentResult.serializer())
        }
    }

    suspend fun listAttachments(): Result<List<AttachmentInfoDto>> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.listAttachments("{}")
            parseRustResult(result, ListSerializer(AttachmentInfoDto.serializer()))
        }
    }

    suspend fun getAttachmentData(attachmentName: String): Result<AttachmentDataDto> = withContext(Dispatchers.IO) {
        runCatching {
            val params = GetAttachmentDataParams(attachmentName)
            val jsonInput = json.encodeToString(GetAttachmentDataParams.serializer(), params)
            val result = RustCore.getAttachmentData(jsonInput)
            parseRustResult(result, AttachmentDataDto.serializer())
        }
    }
}
