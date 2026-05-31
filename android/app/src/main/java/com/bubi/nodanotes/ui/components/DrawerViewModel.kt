package com.bubi.nodanotes.ui.components

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.bubi.nodanotes.data.model.FolderDto
import com.bubi.nodanotes.data.repository.FolderRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

class DrawerViewModel(application: Application) : AndroidViewModel(application) {

    private val folderRepository = FolderRepository()

    private val _folders = MutableStateFlow<List<FolderDto>>(emptyList())
    val folders: StateFlow<List<FolderDto>> = _folders.asStateFlow()

    private val _expandedPaths = MutableStateFlow<Set<String>>(emptySet())
    val expandedPaths: StateFlow<Set<String>> = _expandedPaths.asStateFlow()

    private val _isLoading = MutableStateFlow(false)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _conflictCount = MutableStateFlow(0)
    val conflictCount: StateFlow<Int> = _conflictCount.asStateFlow()

    private val _trashCount = MutableStateFlow(0)
    val trashCount: StateFlow<Int> = _trashCount.asStateFlow()

    private val conflictRepository = com.bubi.nodanotes.data.repository.ConflictRepository()
    private val trashRepository = com.bubi.nodanotes.data.repository.TrashRepository()


    private val _attachmentCount = MutableStateFlow(0)
    val attachmentCount: StateFlow<Int> = _attachmentCount.asStateFlow()

    private val attachmentRepository = com.bubi.nodanotes.data.repository.AttachmentRepository()

    init {
        loadFolders()
    }

    fun loadFolders() {
        viewModelScope.launch(Dispatchers.IO) {
            // Only show full loading spinner on initial empty load to prevent sidebar items from blinking
            if (_folders.value.isEmpty()) {
                _isLoading.value = true
            }
            folderRepository.listFolders().fold(
                onSuccess = { folderList ->
                    _folders.value = folderList
                    _isLoading.value = false
                },
                onFailure = {
                    _isLoading.value = false
                }
            )
            conflictRepository.listConflicts().onSuccess { list ->
                _conflictCount.value = list.size
            }
            trashRepository.listTrash().onSuccess { list ->
                _trashCount.value = list.size
            }
            attachmentRepository.listAttachments().onSuccess { list ->
                _attachmentCount.value = list.size
            }
        }
    }

    fun toggleExpanded(path: String) {
        val current = _expandedPaths.value.toMutableSet()
        if (current.contains(path)) {
            current.remove(path)
        } else {
            current.add(path)
        }
        _expandedPaths.value = current
    }

    fun createFolder(parentPath: String?, name: String) {
        viewModelScope.launch(Dispatchers.IO) {
            folderRepository.createFolder(parentPath, name).onSuccess {
                loadFolders()
            }
        }
    }

    fun renameFolder(folderPath: String, newName: String) {
        viewModelScope.launch(Dispatchers.IO) {
            folderRepository.renameFolder(folderPath, newName).onSuccess {
                loadFolders()
            }
        }
    }

    fun deleteFolder(folderPath: String) {
        viewModelScope.launch(Dispatchers.IO) {
            folderRepository.deleteFolder(folderPath).onSuccess {
                loadFolders()
            }
        }
    }
}
