package com.bubi.nodanotes.ui.screens.conflict

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.bubi.nodanotes.data.model.ConflictEntryDto
import com.bubi.nodanotes.data.model.NoteDto
import com.bubi.nodanotes.data.repository.ConflictRepository
import com.bubi.nodanotes.data.repository.NoteRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

sealed interface ConflictUiState {
    object Loading : ConflictUiState
    data class Success(val conflicts: List<ConflictEntryDto>) : ConflictUiState
    data class Error(val message: String) : ConflictUiState
}

class ConflictViewModel(application: Application) : AndroidViewModel(application) {

    private val conflictRepository = ConflictRepository()
    private val noteRepository = NoteRepository()

    private val _uiState = MutableStateFlow<ConflictUiState>(ConflictUiState.Loading)
    val uiState: StateFlow<ConflictUiState> = _uiState.asStateFlow()

    private val _activeConflictDetail = MutableStateFlow<Pair<NoteDto, NoteDto>?>(null) // Local to Remote pair
    val activeConflictDetail: StateFlow<Pair<NoteDto, NoteDto>?> = _activeConflictDetail.asStateFlow()

    private val _isLoadingDetail = MutableStateFlow(false)
    val isLoadingDetail: StateFlow<Boolean> = _isLoadingDetail.asStateFlow()

    init {
        loadConflicts()
    }

    fun loadConflicts() {
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = ConflictUiState.Loading
            conflictRepository.listConflicts().fold(
                onSuccess = { list ->
                    _uiState.value = ConflictUiState.Success(list)
                },
                onFailure = { error ->
                    _uiState.value = ConflictUiState.Error(error.message ?: "Failed to list conflicts")
                }
            )
        }
    }

    fun loadConflictDetail(conflict: ConflictEntryDto) {
        viewModelScope.launch(Dispatchers.IO) {
            _isLoadingDetail.value = true
            _activeConflictDetail.value = null

            val localResult = noteRepository.getNote(conflict.id)
            val remoteResult = conflictRepository.getConflictNote(conflict.archived_path)

            if (localResult.isSuccess && remoteResult.isSuccess) {
                _activeConflictDetail.value = Pair(localResult.getOrThrow(), remoteResult.getOrThrow())
            }
            _isLoadingDetail.value = false
        }
    }

    fun resolveConflict(noteId: String, resolution: String) {
        viewModelScope.launch(Dispatchers.IO) {
            _isLoadingDetail.value = true
            conflictRepository.resolveConflict(noteId, resolution).fold(
                onSuccess = {
                    _activeConflictDetail.value = null
                    loadConflicts()
                },
                onFailure = {
                    _isLoadingDetail.value = false
                }
            )
        }
    }

    fun clearActiveConflict() {
        _activeConflictDetail.value = null
    }
}
