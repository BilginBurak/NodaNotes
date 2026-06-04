package com.bubi.nodanotes.ui.screens.editor

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.bubi.nodanotes.data.model.NoteDto
import com.bubi.nodanotes.data.model.NoteMetadataDto
import com.bubi.nodanotes.data.model.AttachmentInfoDto
import com.bubi.nodanotes.data.repository.NoteRepository
import com.bubi.nodanotes.data.repository.SearchRepository
import com.bubi.nodanotes.data.preferences.VaultPreferences
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

class NoteEditorViewModel(application: Application) : AndroidViewModel(application) {

    private val noteRepository = NoteRepository()
    private val searchRepository = SearchRepository()

    private val attachmentRepository = com.bubi.nodanotes.data.repository.AttachmentRepository()

    private val _uiState = MutableStateFlow<NoteEditorUiState>(NoteEditorUiState.Loading)
    val uiState: StateFlow<NoteEditorUiState> = _uiState.asStateFlow()

    private val _saveState = MutableStateFlow<SaveState>(SaveState.Saved)
    val saveState: StateFlow<SaveState> = _saveState.asStateFlow()

    private val _metadata = MutableStateFlow<NoteMetadataDto?>(null)
    val metadata: StateFlow<NoteMetadataDto?> = _metadata.asStateFlow()

    private val _tagSuggestions = MutableStateFlow<List<String>>(emptyList())
    val tagSuggestions: StateFlow<List<String>> = _tagSuggestions.asStateFlow()

    private val _isReaderMode = MutableStateFlow(false)
    val isReaderMode: StateFlow<Boolean> = _isReaderMode.asStateFlow()

    private val _recentAttachments = MutableStateFlow<List<AttachmentInfoDto>>(emptyList())
    val recentAttachments: StateFlow<List<AttachmentInfoDto>> = _recentAttachments.asStateFlow()

    val vaultPath: String? = VaultPreferences(application).getVaultPath()

    private var activeNoteId: String = ""
    private var currentNoteDto: NoteDto? = null
    private var autoSaveJob: Job? = null
    private var sessionModified = false
    private var lastSnapshotTime = System.currentTimeMillis()

    init {
        com.bubi.nodanotes.data.repository.ActiveNoteTracker.activeSaveAction = { reason ->
            val successState = _uiState.value as? NoteEditorUiState.Success
            if (successState != null && sessionModified) {
                saveWithReason(successState.note, reason)
            }
        }
    }

    fun loadNote(noteId: String) {
        activeNoteId = noteId
        sessionModified = false
        lastSnapshotTime = System.currentTimeMillis()
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = NoteEditorUiState.Loading
            noteRepository.getNote(noteId).fold(
                onSuccess = { note ->
                    currentNoteDto = note
                    _uiState.value = NoteEditorUiState.Success(note)
                    loadMetadata(noteId)
                },
                onFailure = { error ->
                    _uiState.value = NoteEditorUiState.Error(error.message ?: "Failed to load note")
                }
            )
        }
    }

    fun onTitleChanged(newTitle: String) {
        val successState = _uiState.value as? NoteEditorUiState.Success ?: return
        if (successState.note.title == newTitle) return
        sessionModified = true
        val updatedNote = successState.note.copy(title = newTitle)
        _uiState.value = NoteEditorUiState.Success(updatedNote)
        triggerAutoSave(updatedNote)
    }

    fun onContentChanged(newContent: String) {
        val successState = _uiState.value as? NoteEditorUiState.Success ?: return
        if (successState.note.body == newContent) return
        sessionModified = true
        val updatedNote = successState.note.copy(body = newContent)
        _uiState.value = NoteEditorUiState.Success(updatedNote)
        triggerAutoSave(updatedNote)
    }

    fun onTagsChanged(newTags: List<String>) {
        val successState = _uiState.value as? NoteEditorUiState.Success ?: return
        if (successState.note.tags == newTags) return
        sessionModified = true
        val updatedNote = successState.note.copy(tags = newTags)
        _uiState.value = NoteEditorUiState.Success(updatedNote)
        triggerAutoSave(updatedNote)
    }

    private fun triggerAutoSave(note: NoteDto) {
        _saveState.value = SaveState.Unsaved
        autoSaveJob?.cancel()
        autoSaveJob = viewModelScope.launch(Dispatchers.IO) {
            val settings = com.bubi.nodanotes.data.repository.SettingsRepository().getSettings().getOrNull()
            val delayMs = settings?.editor?.auto_save_delay_ms?.toLong() ?: 3000L
            delay(delayMs)
            saveNoteImmediately(note)
        }
    }

    fun saveNoteImmediately() {
        if (_saveState.value != SaveState.Unsaved) return
        val successState = _uiState.value as? NoteEditorUiState.Success ?: return
        autoSaveJob?.cancel()
        viewModelScope.launch(Dispatchers.IO) {
            saveNoteImmediately(successState.note)
        }
    }

    private suspend fun saveNoteImmediately(note: NoteDto) {
        _saveState.value = SaveState.Saving
        val settings = com.bubi.nodanotes.data.repository.SettingsRepository().getSettings().getOrNull()
        val intervalMins = settings?.history?.snapshot_interval_mins ?: 5
        val now = System.currentTimeMillis()
        val shouldSnapshot = sessionModified && (now - lastSnapshotTime >= intervalMins * 60 * 1000)

        noteRepository.updateNote(
            noteId = note.id,
            title = note.title,
            body = note.body,
            tags = note.tags,
            color = note.color,
            pinned = note.pinned,
            triggerSnapshot = shouldSnapshot,
            snapshotReason = if (shouldSnapshot) "AutoSave" else null
        ).fold(
            onSuccess = {
                if (shouldSnapshot) {
                    lastSnapshotTime = now
                }
                sessionModified = false
                _saveState.value = SaveState.Saved
                loadMetadata(note.id)
            },
            onFailure = { error ->
                _saveState.value = SaveState.Error(error.message ?: "Auto-save failed")
            }
        )
    }

    fun loadSuggestions(prefix: String) {
        viewModelScope.launch(Dispatchers.IO) {
            noteRepository.getAllTags().fold(
                onSuccess = { tags ->
                    _tagSuggestions.value = if (prefix.isEmpty()) {
                        tags
                    } else {
                        tags.filter { it.startsWith(prefix, ignoreCase = true) }
                    }
                },
                onFailure = {
                    _tagSuggestions.value = emptyList()
                }
            )
        }
    }

    fun loadMetadata(noteId: String) {
        viewModelScope.launch(Dispatchers.IO) {
            noteRepository.getNoteMetadata(noteId).fold(
                onSuccess = { meta ->
                    _metadata.value = meta
                },
                onFailure = {
                    _metadata.value = null
                }
            )
        }
    }

    fun addAttachment(sourcePath: String, onLinkGenerated: (String) -> Unit) {
        viewModelScope.launch(Dispatchers.IO) {
            attachmentRepository.addAttachment(sourcePath).fold(
                onSuccess = { result ->
                    onLinkGenerated(result.markdown_link)
                    loadRecentAttachments() // reload recent attachments after addition
                },
                onFailure = { error ->
                    _uiState.value = NoteEditorUiState.Error(error.message ?: "Failed to add attachment")
                }
            )
        }
    }

    fun loadRecentAttachments() {
        viewModelScope.launch(Dispatchers.IO) {
            attachmentRepository.listAttachments().fold(
                onSuccess = { list ->
                    val sorted = list.sortedByDescending { it.modified_at }.take(20)
                    _recentAttachments.value = sorted
                },
                onFailure = {
                    _recentAttachments.value = emptyList()
                }
            )
        }
    }

    fun toggleReaderMode() {
        _isReaderMode.value = !_isReaderMode.value
    }

    fun togglePinNote() {
        val successState = _uiState.value as? NoteEditorUiState.Success ?: return
        val updatedNote = successState.note.copy(pinned = !successState.note.pinned)
        _uiState.value = NoteEditorUiState.Success(updatedNote)
        viewModelScope.launch(Dispatchers.IO) {
            noteRepository.updateNote(
                noteId = updatedNote.id,
                title = updatedNote.title,
                body = updatedNote.body,
                tags = updatedNote.tags,
                color = updatedNote.color,
                pinned = updatedNote.pinned
            ).fold(
                onSuccess = {
                    loadMetadata(updatedNote.id)
                },
                onFailure = { error ->
                    _uiState.value = NoteEditorUiState.Error(error.message ?: "Failed to update pin status")
                }
            )
        }
    }

    fun deleteNote(onDeleted: () -> Unit) {
        val successState = _uiState.value as? NoteEditorUiState.Success ?: return
        viewModelScope.launch(Dispatchers.IO) {
            noteRepository.deleteNote(successState.note.id).fold(
                onSuccess = {
                    viewModelScope.launch(Dispatchers.Main) {
                        onDeleted()
                    }
                },
                onFailure = { error ->
                    _uiState.value = NoteEditorUiState.Error(error.message ?: "Failed to delete note")
                }
            )
        }
    }

    fun toggleTaskStatus(noteId: String, lineContent: String) {
        viewModelScope.launch(Dispatchers.IO) {
            noteRepository.toggleTaskStatus(noteId, lineContent).fold(
                onSuccess = { updatedNote ->
                    currentNoteDto = updatedNote
                    _uiState.value = NoteEditorUiState.Success(updatedNote)
                    loadMetadata(noteId)
                },
                onFailure = {
                    // Ignore or log error
                }
            )
        }
    }

    private suspend fun saveWithReason(note: NoteDto, reason: String) {
        _saveState.value = SaveState.Saving
        noteRepository.updateNote(
            noteId = note.id,
            title = note.title,
            body = note.body,
            tags = note.tags,
            color = note.color,
            pinned = note.pinned,
            triggerSnapshot = true,
            snapshotReason = reason
        ).fold(
            onSuccess = {
                lastSnapshotTime = System.currentTimeMillis()
                sessionModified = false
                _saveState.value = SaveState.Saved
                loadMetadata(note.id)
            },
            onFailure = { error ->
                _saveState.value = SaveState.Error(error.message ?: "Save failed")
            }
        )
    }

    override fun onCleared() {
        com.bubi.nodanotes.data.repository.ActiveNoteTracker.activeSaveAction = null
        saveNoteOnExitSync("Blur")
        super.onCleared()
    }

    fun saveNoteOnAppExit() {
        val successState = _uiState.value as? NoteEditorUiState.Success ?: return
        if (!sessionModified) return
        viewModelScope.launch(Dispatchers.IO) {
            noteRepository.updateNote(
                noteId = successState.note.id,
                title = successState.note.title,
                body = successState.note.body,
                tags = successState.note.tags,
                color = successState.note.color,
                pinned = successState.note.pinned,
                triggerSnapshot = true,
                snapshotReason = "App-Exit"
            ).onSuccess {
                sessionModified = false
                lastSnapshotTime = System.currentTimeMillis()
                _saveState.value = SaveState.Saved
            }
        }
    }

    private fun saveNoteOnExitSync(reason: String) {
        val successState = _uiState.value as? NoteEditorUiState.Success ?: return
        if (!sessionModified) return
        autoSaveJob?.cancel()
        try {
            kotlinx.coroutines.runBlocking(Dispatchers.IO) {
                noteRepository.updateNote(
                    noteId = successState.note.id,
                    title = successState.note.title,
                    body = successState.note.body,
                    tags = successState.note.tags,
                    color = successState.note.color,
                    pinned = successState.note.pinned,
                    triggerSnapshot = true,
                    snapshotReason = reason
                )
            }
            sessionModified = false
            lastSnapshotTime = System.currentTimeMillis()
        } catch (e: Exception) {
            // Log or ignore during destruction
        }
    }
}

sealed class NoteEditorUiState {
    object Loading : NoteEditorUiState()
    data class Success(val note: NoteDto) : NoteEditorUiState()
    data class Error(val message: String) : NoteEditorUiState()
}

sealed class SaveState {
    object Saved : SaveState()
    object Unsaved : SaveState()
    object Saving : SaveState()
    data class Error(val message: String) : SaveState()
}
