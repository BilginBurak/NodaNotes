package com.bubi.nodanotes.ui.screens.maintenance

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.bubi.nodanotes.data.model.DuplicateNoteGroupDto
import com.bubi.nodanotes.data.model.OrphanedFileDto
import com.bubi.nodanotes.data.repository.DiagnosticsRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

sealed class MaintenanceUiState {
    object Idle : MaintenanceUiState()
    object Loading : MaintenanceUiState()
    data class Success(val message: String) : MaintenanceUiState()
    data class Error(val message: String) : MaintenanceUiState()
}

class MaintenanceViewModel(application: Application) : AndroidViewModel(application) {
    private val diagnosticsRepository = DiagnosticsRepository()
    val vaultPath: String? = com.bubi.nodanotes.data.preferences.VaultPreferences(application).getVaultPath()

    private val _uiState = MutableStateFlow<MaintenanceUiState>(MaintenanceUiState.Idle)
    val uiState: StateFlow<MaintenanceUiState> = _uiState.asStateFlow()

    private val _duplicates = MutableStateFlow<List<DuplicateNoteGroupDto>>(emptyList())
    val duplicates: StateFlow<List<DuplicateNoteGroupDto>> = _duplicates.asStateFlow()

    private val _orphanedRemnants = MutableStateFlow<List<OrphanedFileDto>>(emptyList())
    val orphanedRemnants: StateFlow<List<OrphanedFileDto>> = _orphanedRemnants.asStateFlow()

    private val _orphanedAttachments = MutableStateFlow<List<DiagnosticsRepository.OrphanedAttachmentDto>>(emptyList())
    val orphanedAttachments: StateFlow<List<DiagnosticsRepository.OrphanedAttachmentDto>> = _orphanedAttachments.asStateFlow()

    fun rebuildCache() {
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = MaintenanceUiState.Loading
            diagnosticsRepository.rebuildCache().fold(
                onSuccess = {
                    _uiState.value = MaintenanceUiState.Success("Database Cache successfully rebuilt from markdown files.")
                },
                onFailure = { error ->
                    _uiState.value = MaintenanceUiState.Error(error.message ?: "Failed to rebuild cache")
                }
            )
        }
    }

    fun optimizeFts() {
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = MaintenanceUiState.Loading
            diagnosticsRepository.optimizeFts().fold(
                onSuccess = {
                    _uiState.value = MaintenanceUiState.Success("FTS5 Search index optimized successfully.")
                },
                onFailure = { error ->
                    _uiState.value = MaintenanceUiState.Error(error.message ?: "Failed to optimize search index")
                }
            )
        }
    }

    fun scanDuplicates() {
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = MaintenanceUiState.Loading
            diagnosticsRepository.getDuplicateNotes().fold(
                onSuccess = { list ->
                    _duplicates.value = list
                    _uiState.value = MaintenanceUiState.Idle
                },
                onFailure = { error ->
                    _uiState.value = MaintenanceUiState.Error(error.message ?: "Failed to find duplicate notes")
                }
            )
        }
    }

    fun deleteDuplicateFile(filePath: String) {
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = MaintenanceUiState.Loading
            diagnosticsRepository.deleteDuplicateFile(filePath).fold(
                onSuccess = {
                    scanDuplicates()
                },
                onFailure = { error ->
                    _uiState.value = MaintenanceUiState.Error(error.message ?: "Failed to delete duplicate file")
                }
            )
        }
    }

    fun scanOrphanedRemnants() {
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = MaintenanceUiState.Loading
            diagnosticsRepository.getOrphanedRemnants().fold(
                onSuccess = { result ->
                    _orphanedRemnants.value = result.files
                    _uiState.value = MaintenanceUiState.Idle
                },
                onFailure = { error ->
                    _uiState.value = MaintenanceUiState.Error(error.message ?: "Failed to scan orphaned remnants")
                }
            )
        }
    }

    fun deleteOrphanedFile(filePath: String) {
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = MaintenanceUiState.Loading
            diagnosticsRepository.deleteOrphanedFile(filePath).fold(
                onSuccess = {
                    scanOrphanedRemnants()
                },
                onFailure = { error ->
                    _uiState.value = MaintenanceUiState.Error(error.message ?: "Failed to delete orphaned remnant")
                }
            )
        }
    }

    fun deleteAllOrphanedRemnants() {
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = MaintenanceUiState.Loading
            diagnosticsRepository.deleteAllOrphanedRemnants().fold(
                onSuccess = {
                    _orphanedRemnants.value = emptyList()
                    _uiState.value = MaintenanceUiState.Success("All orphaned history and conflict remnants deleted.")
                },
                onFailure = { error ->
                    _uiState.value = MaintenanceUiState.Error(error.message ?: "Failed to delete all remnants")
                }
            )
        }
    }

    fun scanOrphanedAttachments() {
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = MaintenanceUiState.Loading
            diagnosticsRepository.getOrphanedAttachments().fold(
                onSuccess = { list ->
                    _orphanedAttachments.value = list
                    _uiState.value = MaintenanceUiState.Idle
                },
                onFailure = { error ->
                    _uiState.value = MaintenanceUiState.Error(error.message ?: "Failed to scan orphaned attachments")
                }
            )
        }
    }

    fun clearRemoteCache() {
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = MaintenanceUiState.Loading
            diagnosticsRepository.clearRemoteTrackingCache().fold(
                onSuccess = {
                    _uiState.value = MaintenanceUiState.Success("Sync tracking cache cleared successfully.")
                },
                onFailure = { error ->
                    _uiState.value = MaintenanceUiState.Error(error.message ?: "Failed to clear sync tracking cache")
                }
            )
        }
    }

    fun resetSyncQueue() {
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = MaintenanceUiState.Loading
            diagnosticsRepository.resetSyncQueue().fold(
                onSuccess = {
                    _uiState.value = MaintenanceUiState.Success("Sync queue reset successfully.")
                },
                onFailure = { error ->
                    _uiState.value = MaintenanceUiState.Error(error.message ?: "Failed to reset sync queue")
                }
            )
        }
    }
}
