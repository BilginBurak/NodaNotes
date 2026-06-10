package com.bubi.nodanotes.data.model

import kotlinx.serialization.Serializable

@Serializable
data class NoteDto(
    val id: String,
    val parent_id: String?,
    val title: String,
    val body: String,
    val color: String?,
    val pinned: Boolean,
    val tags: List<String>,
    val inline_tags: List<String> = emptyList(),
    val created_at: String,
    val updated_at: String,
    val file_path: String,
)

@Serializable
data class NoteListItemDto(
    val id: String,
    val parent_id: String?,
    val title: String,
    val color: String?,
    val pinned: Boolean,
    val tags: List<String>,
    val inline_tags: List<String> = emptyList(),
    val updated_at: String,
    val file_path: String,
)

@Serializable
data class VaultInfoDto(
    val name: String,
    val path: String,
)

@Serializable
data class SearchResultDto(
    val note_id: String,
    val title: String,
    val snippet: String,
    val match_type: String,
    val score: Double,
)

@Serializable
data class SnapshotDto(
    val note_id: String,
    val timestamp: String,
    val file_path: String,
    val size_bytes: Long,
    val reason: String = "",
)

@Serializable
data class DiffChunk(
    val tag: String, // "Equal", "Insert", "Delete", "Separator"
    val text: String,
)

@Serializable
data class SnapshotDiffDto(
    val note_id: String,
    val timestamp: String,
    val body_chunks: List<DiffChunk>,
)

@Serializable
data class TrashEntryDto(
    val id: String,
    val title: String,
    val original_path: String,
    val deleted_at: String,
    val trash_path: String,
)

@Serializable
data class ConflictEntryDto(
    val id: String,
    val title: String,
    val file_path: String,
    val archived_path: String,
    val detected_at: String,
)

@Serializable
data class SyncReportDto(
    val uploads: Int = 0,
    val downloads: Int = 0,
    val deletes_local: Int = 0,
    val deletes_remote: Int = 0,
    val conflicts: Int = 0,
    val uploaded_files: List<String> = emptyList(),
    val downloaded_files: List<String> = emptyList(),
    val deleted_local_files: List<String> = emptyList(),
    val deleted_remote_files: List<String> = emptyList(),
    val conflict_files: List<String> = emptyList(),
    val sync_time: String? = null,
)

@Serializable
data class NoteMetadataDto(
    val id: String,
    val title: String,
    val file_name: String,
    val relative_path: String,
    val absolute_path: String,
    val created_at: String,
    val updated_at: String,
    val tags: List<String>,
    val history_count: Int,
    val last_upload_time: String?,
    val file_size_bytes: Long,
    val word_count: Int,
    val char_count: Int,
)

@Serializable
data class FolderDto(
    val name: String,
    val path: String,
    val children: List<FolderDto>,
)

@Serializable
data class AppErrorDto(
    val error: String,
)

@Serializable
data class AttachmentInfoDto(
    val name: String,
    val size_bytes: Long,
    val mime_type: String,
    val modified_at: Long,
)

@Serializable
data class DuplicateFileEntryDto(
    val path: String,
    val size_bytes: Long,
    val modified_at: String,
)

@Serializable
data class DuplicateNoteGroupDto(
    val note_id: String,
    val title: String,
    val files: List<DuplicateFileEntryDto>,
)

@Serializable
data class OrphanedFileDto(
    val relative_path: String,
    val title: String,
    val size_bytes: Long,
    val last_modified: String,
    val file_type: String, // "history" or "conflict"
)

@Serializable
data class OrphanedRemnantsDto(
    val files: List<OrphanedFileDto>,
    val total_recovered_bytes: Long,
)

@Serializable
data class SyncStatusDto(
    val is_syncing: Boolean,
    val last_sync_at: String,
    val pending_count: Int,
    val quarantined_files: List<QuarantinedFileDto> = emptyList(),
)

@Serializable
data class QuarantinedFileDto(
    val path: String,
    val retry_count: Int,
    val sync_error: String?,
)

@Serializable
data class AppearanceSettingsDto(
    val theme: String,
    val accent_color: String,
)

@Serializable
data class EditorSettingsDto(
    val font_size: Int,
    val typography: String,
    val show_word_count: Boolean,
    val auto_save_delay_ms: Int,
    val default_daily_template: String? = null,
)

@Serializable
data class HistorySettingsDto(
    val retention_days: Int,
    val max_snapshots_per_note: Int,
    val empty_trash_after_days: Int,
    val snapshot_interval_mins: Int = 5,
)

@Serializable
data class SyncConfigDto(
    val webdav_url: String,
    val webdav_username: String,
    val webdav_password: String?,
    val interval_secs: Long,
    val device_name: String = "",
)

@Serializable
data class SettingsDto(
    val appearance: AppearanceSettingsDto,
    val editor: EditorSettingsDto,
    val sync: SyncConfigDto,
    val history: HistorySettingsDto,
)

@Serializable
data class TagWithCountDto(
    val name: String,
    val count: Int,
)
