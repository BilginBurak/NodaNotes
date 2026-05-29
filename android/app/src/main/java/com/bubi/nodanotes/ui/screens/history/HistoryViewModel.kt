package com.bubi.nodanotes.ui.screens.history

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.bubi.nodanotes.data.model.DiffChunk
import com.bubi.nodanotes.data.model.SnapshotDto
import com.bubi.nodanotes.data.repository.HistoryRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

class HistoryViewModel(application: Application) : AndroidViewModel(application) {

    private val historyRepository = HistoryRepository()

    private val _uiState = MutableStateFlow<HistoryUiState>(HistoryUiState.Loading)
    val uiState: StateFlow<HistoryUiState> = _uiState.asStateFlow()

    private val _diffState = MutableStateFlow<DiffUiState>(DiffUiState.Idle)
    val diffState: StateFlow<DiffUiState> = _diffState.asStateFlow()

    private var activeNoteId: String = ""

    fun loadSnapshots(noteId: String) {
        activeNoteId = noteId
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = HistoryUiState.Loading
            historyRepository.listSnapshots(noteId).fold(
                onSuccess = { snapshots ->
                    // Sort snapshots descending by timestamp (newest first)
                    val sorted = snapshots.sortedByDescending { it.timestamp }
                    _uiState.value = HistoryUiState.Success(sorted)
                },
                onFailure = { error ->
                    _uiState.value = HistoryUiState.Error(error.message ?: "Failed to load version history")
                }
            )
        }
    }

    fun loadDiff(timestamp: String) {
        if (activeNoteId.isEmpty()) return
        viewModelScope.launch(Dispatchers.IO) {
            _diffState.value = DiffUiState.Loading
            historyRepository.getSnapshotDiff(activeNoteId, timestamp).fold(
                onSuccess = { diffDto ->
                    _diffState.value = DiffUiState.Success(diffDto.body_chunks)
                },
                onFailure = { error ->
                    _diffState.value = DiffUiState.Error(error.message ?: "Failed to load snapshot changes")
                }
            )
        }
    }

    fun clearDiff() {
        _diffState.value = DiffUiState.Idle
    }

    fun restoreSnapshot(timestamp: String, onSuccess: () -> Unit) {
        if (activeNoteId.isEmpty()) return
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = HistoryUiState.Loading
            historyRepository.restoreSnapshot(activeNoteId, timestamp).fold(
                onSuccess = {
                    viewModelScope.launch(Dispatchers.Main) {
                        onSuccess()
                    }
                },
                onFailure = { error ->
                    _uiState.value = HistoryUiState.Error(error.message ?: "Failed to restore version")
                }
            )
        }
    }

    fun deleteSnapshot(timestamp: String) {
        if (activeNoteId.isEmpty()) return
        viewModelScope.launch(Dispatchers.IO) {
            historyRepository.deleteSnapshot(activeNoteId, timestamp).fold(
                onSuccess = {
                    // Reload snapshot list
                    loadSnapshots(activeNoteId)
                },
                onFailure = { error ->
                    _uiState.value = HistoryUiState.Error(error.message ?: "Failed to delete snapshot")
                }
            )
        }
    }
}

sealed class HistoryUiState {
    object Loading : HistoryUiState()
    data class Success(val snapshots: List<SnapshotDto>) : HistoryUiState()
    data class Error(val message: String) : HistoryUiState()
}

sealed class DiffUiState {
    object Idle : DiffUiState()
    object Loading : DiffUiState()
    data class Success(val chunks: List<DiffChunk>) : DiffUiState()
    data class Error(val message: String) : DiffUiState()
}
