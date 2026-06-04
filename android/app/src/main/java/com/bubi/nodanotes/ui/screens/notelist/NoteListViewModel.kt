package com.bubi.nodanotes.ui.screens.notelist

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.bubi.nodanotes.data.model.NoteListItemDto
import com.bubi.nodanotes.data.preferences.VaultPreferences
import com.bubi.nodanotes.data.repository.NoteRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

import android.app.PendingIntent
import android.content.Intent
import com.bubi.nodanotes.data.repository.SyncRepository
import com.bubi.nodanotes.data.model.SyncReportDto

class NoteListViewModel(application: Application) : AndroidViewModel(application) {

    private val noteRepository = NoteRepository()
    private val vaultPreferences = VaultPreferences(application)
    private val syncRepository = SyncRepository()

    private val _uiState = MutableStateFlow<NoteListUiState>(NoteListUiState.Loading)
    val uiState: StateFlow<NoteListUiState> = _uiState.asStateFlow()

    private val _currentFolder = MutableStateFlow<String?>(null)
    val currentFolder: StateFlow<String?> = _currentFolder.asStateFlow()

    private val _vaultName = MutableStateFlow("")
    val vaultName: StateFlow<String> = _vaultName.asStateFlow()

    private val _isRefreshing = MutableStateFlow(false)
    val isRefreshing: StateFlow<Boolean> = _isRefreshing.asStateFlow()

    private val _syncStatus = MutableStateFlow("Sync idle")
    val syncStatus: StateFlow<String> = _syncStatus.asStateFlow()

    private var recentlyDeletedNote: NoteListItemDto? = null
    private var recentlyDeletedPath: String? = null

    init {
        val path = vaultPreferences.getVaultPath()
        _vaultName.value = path?.substringAfterLast('/') ?: "NodaNotes"
        updateRelativeSyncStatus()
    }

    fun loadNotes(folderPath: String? = null) {
        _currentFolder.value = folderPath
        viewModelScope.launch(Dispatchers.IO) {
            val currentState = _uiState.value
            if (currentState !is NoteListUiState.Success) {
                _uiState.value = NoteListUiState.Loading
            }
            noteRepository.getAllNotes().fold(
                onSuccess = { allNotes ->
                    val tagFilter = FolderContext.selectedTag
                    
                    // Filter Daily Notes isolation
                    val dailyNotesPrefix = "Daily Notes/"
                    val folderFiltered = if (folderPath == null) {
                        // All Notes: exclude Daily Notes
                        allNotes.filter { !it.file_path.startsWith(dailyNotesPrefix) }
                    } else if (folderPath == "Daily Notes") {
                        // Daily Notes screen: only show Daily Notes
                        allNotes.filter { it.file_path.startsWith(dailyNotesPrefix) }
                    } else {
                        // Standard folder: filter by subfolder hierarchy
                        allNotes.filter { note ->
                            val noteFolder = note.file_path.substringBeforeLast('/', "")
                            noteFolder == folderPath || noteFolder.startsWith("$folderPath/")
                        }
                    }

                    val finalFiltered = if (tagFilter == null) {
                        folderFiltered
                    } else {
                        folderFiltered.filter { note ->
                            note.tags.contains(tagFilter) || note.inline_tags.contains(tagFilter)
                        }
                    }
                    val newState = NoteListUiState.Success(finalFiltered)
                    if (_uiState.value != newState) {
                        _uiState.value = newState
                    }
                },
                onFailure = { error ->
                    _uiState.value = NoteListUiState.Error(error.message ?: "Failed to load notes")
                }
            )
        }
    }

    fun triggerSync() {
        viewModelScope.launch(Dispatchers.IO) {
            _isRefreshing.value = true
            _syncStatus.value = "Syncing..."
            syncRepository.syncNow().fold(
                onSuccess = { report ->
                    _isRefreshing.value = false
                    vaultPreferences.saveLastSyncTime(System.currentTimeMillis())
                    _syncStatus.value = "Synced just now: ↑${report.uploads} ↓${report.downloads}"
                    vaultPreferences.saveLastSyncReport(
                        kotlinx.serialization.json.Json.encodeToString(
                            SyncReportDto.serializer(),
                            report
                        )
                    )
                    postSyncNotification(report)
                    loadNotes(FolderContext.currentFolder)
                },
                onFailure = { error ->
                    _isRefreshing.value = false
                    _syncStatus.value = "Sync failed"
                }
            )
        }
    }

    fun updateRelativeSyncStatus() {
        viewModelScope.launch(Dispatchers.IO) {
            syncRepository.getSyncStatus().fold(
                onSuccess = { status ->
                    if (status.is_syncing) {
                        _syncStatus.value = "Syncing..."
                    } else {
                        val lastSync = vaultPreferences.getLastSyncTime()
                        if (lastSync == 0L) {
                            _syncStatus.value = "Never synced"
                        } else {
                            val elapsed = System.currentTimeMillis() - lastSync
                            if (elapsed < 60000) {
                                _syncStatus.value = "Synced just now"
                            } else {
                                val mins = elapsed / 60000
                                if (mins < 60) {
                                    _syncStatus.value = "Synced $mins min${if (mins > 1) "s" else ""} ago"
                                } else {
                                    val hours = mins / 60
                                    if (hours < 24) {
                                        _syncStatus.value = "Synced $hours hour${if (hours > 1) "s" else ""} ago"
                                    } else {
                                        val sdf = java.text.SimpleDateFormat("MMM d, yyyy HH:mm", java.util.Locale.US)
                                        _syncStatus.value = "Synced: " + sdf.format(java.util.Date(lastSync))
                                    }
                                }
                            }
                        }
                    }
                },
                onFailure = {
                    val lastSync = vaultPreferences.getLastSyncTime()
                    if (lastSync == 0L) {
                        _syncStatus.value = "Never synced"
                    } else {
                        val elapsed = System.currentTimeMillis() - lastSync
                        if (elapsed < 60000) {
                            _syncStatus.value = "Synced just now"
                        } else {
                            val mins = elapsed / 60000
                            if (mins < 60) {
                                _syncStatus.value = "Synced $mins min${if (mins > 1) "s" else ""} ago"
                            } else {
                                val hours = mins / 60
                                if (hours < 24) {
                                    _syncStatus.value = "Synced $hours hour${if (hours > 1) "s" else ""} ago"
                                } else {
                                    val sdf = java.text.SimpleDateFormat("MMM d, yyyy HH:mm", java.util.Locale.US)
                                    _syncStatus.value = "Synced: " + sdf.format(java.util.Date(lastSync))
                                }
                            }
                        }
                    }
                }
            )
        }
    }

    fun triggerFilesystemScan() {
        viewModelScope.launch(Dispatchers.IO) {
            _isRefreshing.value = true
            _syncStatus.value = "Scanning filesystem..."
            com.bubi.nodanotes.data.repository.VaultRepository().refreshVault().fold(
                onSuccess = {
                    _isRefreshing.value = false
                    _syncStatus.value = "Filesystem scan complete"
                    loadNotes(FolderContext.currentFolder)
                },
                onFailure = { error ->
                    _isRefreshing.value = false
                    _syncStatus.value = "Scan failed: ${error.message}"
                }
            )
        }
    }

    private fun postSyncNotification(report: SyncReportDto) {
        val context = getApplication<Application>()
        try {
            val intent = Intent(context, Class.forName("com.bubi.nodanotes.MainActivity")).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
                putExtra("navigate_to", "sync_report")
            }
            val pendingIntent = PendingIntent.getActivity(
                context,
                0,
                intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )

            val message = "Uploaded ${report.uploads} notes, downloaded ${report.downloads} notes, ${report.conflicts} conflicts."
            val builder = androidx.core.app.NotificationCompat.Builder(context, "noda_sync")
                .setSmallIcon(android.R.drawable.stat_notify_sync)
                .setContentTitle("NodaNotes Sync Complete")
                .setContentText(message)
                .setPriority(androidx.core.app.NotificationCompat.PRIORITY_LOW)
                .setContentIntent(pendingIntent)
                .setAutoCancel(true)

            val notificationManager = context.getSystemService(android.content.Context.NOTIFICATION_SERVICE) as android.app.NotificationManager
            notificationManager.notify(1001, builder.build())
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    fun createNote(parentFolder: String?, onNoteCreated: (String) -> Unit) {
        viewModelScope.launch(Dispatchers.IO) {
            noteRepository.createNote("Untitled", parentFolder, emptyList()).fold(
                onSuccess = { newNote ->
                    viewModelScope.launch(Dispatchers.Main) {
                        onNoteCreated(newNote.id)
                    }
                },
                onFailure = { error ->
                    _uiState.value = NoteListUiState.Error(error.message ?: "Failed to create note")
                }
            )
        }
    }

    fun deleteNoteWithUndo(note: NoteListItemDto, onShowUndoSnackbar: (suspend () -> Unit) -> Unit) {
        recentlyDeletedNote = note
        recentlyDeletedPath = note.file_path

        viewModelScope.launch(Dispatchers.IO) {
            noteRepository.deleteNote(note.id).fold(
                onSuccess = {
                    loadNotes(FolderContext.currentFolder)
                    
                    // Trigger undo Snackbar
                    viewModelScope.launch(Dispatchers.Main) {
                        onShowUndoSnackbar {
                            restoreDeletedNote()
                        }
                    }
                },
                onFailure = { error ->
                    _uiState.value = NoteListUiState.Error(error.message ?: "Failed to delete note")
                }
            )
        }
    }

    private fun restoreDeletedNote() {
        val note = recentlyDeletedNote ?: return
        viewModelScope.launch(Dispatchers.IO) {
            val trashRepository = com.bubi.nodanotes.data.repository.TrashRepository()
            trashRepository.restoreFromTrash(note.id).fold(
                onSuccess = {
                    loadNotes(FolderContext.currentFolder)
                },
                onFailure = { error ->
                    _uiState.value = NoteListUiState.Error(error.message ?: "Failed to restore note")
                }
            )
        }
    }

    fun togglePinNote(note: NoteListItemDto) {
        viewModelScope.launch(Dispatchers.IO) {
            noteRepository.getNote(note.id).fold(
                onSuccess = { noteDto ->
                    noteRepository.updateNote(
                        noteId = noteDto.id,
                        title = noteDto.title,
                        body = noteDto.body,
                        tags = noteDto.tags,
                        color = noteDto.color,
                        pinned = !noteDto.pinned
                    ).fold(
                        onSuccess = {
                            loadNotes(FolderContext.currentFolder)
                        },
                        onFailure = { error ->
                            _uiState.value = NoteListUiState.Error(error.message ?: "Failed to update pin status")
                        }
                    )
                },
                onFailure = { error ->
                    _uiState.value = NoteListUiState.Error(error.message ?: "Failed to read note details")
                }
            )
        }
    }

    fun refreshOnResume() {
        loadNotes(FolderContext.currentFolder)
    }
}

sealed class NoteListUiState {
    object Loading : NoteListUiState()
    data class Success(val notes: List<NoteListItemDto>) : NoteListUiState()
    data class Error(val message: String) : NoteListUiState()
}
