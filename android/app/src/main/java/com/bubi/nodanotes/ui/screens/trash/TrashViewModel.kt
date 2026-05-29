package com.bubi.nodanotes.ui.screens.trash

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.bubi.nodanotes.data.model.TrashEntryDto
import com.bubi.nodanotes.data.repository.TrashRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

sealed class TrashUiState {
    object Loading : TrashUiState()
    data class Success(val entries: List<TrashEntryDto>) : TrashUiState()
    data class Error(val message: String) : TrashUiState()
}

class TrashViewModel(application: Application) : AndroidViewModel(application) {
    private val trashRepository = TrashRepository()

    private val _uiState = MutableStateFlow<TrashUiState>(TrashUiState.Loading)
    val uiState: StateFlow<TrashUiState> = _uiState.asStateFlow()

    init {
        loadTrash()
    }

    fun loadTrash() {
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = TrashUiState.Loading
            trashRepository.listTrash().fold(
                onSuccess = { list ->
                    _uiState.value = TrashUiState.Success(list)
                },
                onFailure = { error ->
                    _uiState.value = TrashUiState.Error(error.message ?: "Failed to load trash")
                }
            )
        }
    }

    fun restore(noteId: String, onSuccess: () -> Unit = {}) {
        viewModelScope.launch(Dispatchers.IO) {
            trashRepository.restoreFromTrash(noteId).fold(
                onSuccess = {
                    loadTrash()
                    onSuccess()
                },
                onFailure = { error ->
                    _uiState.value = TrashUiState.Error(error.message ?: "Failed to restore note")
                }
            )
        }
    }

    fun permanentDelete(noteId: String, onSuccess: () -> Unit = {}) {
        viewModelScope.launch(Dispatchers.IO) {
            trashRepository.permanentDelete(noteId).fold(
                onSuccess = {
                    loadTrash()
                    onSuccess()
                },
                onFailure = { error ->
                    _uiState.value = TrashUiState.Error(error.message ?: "Failed to permanently delete note")
                }
            )
        }
    }

    fun emptyTrash(onSuccess: () -> Unit = {}) {
        viewModelScope.launch(Dispatchers.IO) {
            trashRepository.emptyTrash().fold(
                onSuccess = {
                    loadTrash()
                    onSuccess()
                },
                onFailure = { error ->
                    _uiState.value = TrashUiState.Error(error.message ?: "Failed to empty trash")
                }
            )
        }
    }
}
