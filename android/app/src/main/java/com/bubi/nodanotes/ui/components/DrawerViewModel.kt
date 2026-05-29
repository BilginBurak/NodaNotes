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

    init {
        loadFolders()
    }

    fun loadFolders() {
        viewModelScope.launch(Dispatchers.IO) {
            _isLoading.value = true
            folderRepository.listFolders().fold(
                onSuccess = { folderList ->
                    _folders.value = folderList
                    _isLoading.value = false
                },
                onFailure = {
                    _isLoading.value = false
                }
            )
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
