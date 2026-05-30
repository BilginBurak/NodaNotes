package com.bubi.nodanotes.ui.screens.attachments

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.bubi.nodanotes.data.model.AttachmentInfoDto
import com.bubi.nodanotes.data.preferences.VaultPreferences
import com.bubi.nodanotes.data.repository.AttachmentRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

class AttachmentsViewModel(application: Application) : AndroidViewModel(application) {

    private val attachmentRepository = AttachmentRepository()
    private val vaultPreferences = VaultPreferences(application)

    private val _uiState = MutableStateFlow<AttachmentsUiState>(AttachmentsUiState.Loading)
    val uiState: StateFlow<AttachmentsUiState> = _uiState.asStateFlow()

    val vaultPath: String? = vaultPreferences.getVaultPath()

    init {
        loadAttachments()
    }

    fun loadAttachments() {
        viewModelScope.launch(Dispatchers.IO) {
            _uiState.value = AttachmentsUiState.Loading
            attachmentRepository.listAttachments().fold(
                onSuccess = { list ->
                    _uiState.value = AttachmentsUiState.Success(list)
                },
                onFailure = { error ->
                    _uiState.value = AttachmentsUiState.Error(error.message ?: "Failed to list attachments")
                }
            )
        }
    }

    fun deleteAttachment(name: String) {
        viewModelScope.launch(Dispatchers.IO) {
            attachmentRepository.deleteAttachment(name).fold(
                onSuccess = {
                    loadAttachments()
                },
                onFailure = { error ->
                    _uiState.value = AttachmentsUiState.Error(error.message ?: "Failed to delete attachment")
                }
            )
        }
    }
}

sealed class AttachmentsUiState {
    object Loading : AttachmentsUiState()
    data class Success(val attachments: List<AttachmentInfoDto>) : AttachmentsUiState()
    data class Error(val message: String) : AttachmentsUiState()
}
