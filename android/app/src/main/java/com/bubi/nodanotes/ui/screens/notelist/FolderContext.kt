package com.bubi.nodanotes.ui.screens.notelist

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

object FolderContext {
    private val _currentFolder = MutableStateFlow<String?>(null)
    val currentFolderState: StateFlow<String?> = _currentFolder.asStateFlow()

    private val _selectedTag = MutableStateFlow<String?>(null)
    val selectedTagState: StateFlow<String?> = _selectedTag.asStateFlow()

    var currentFolder: String?
        get() = _currentFolder.value
        set(value) {
            _currentFolder.value = value
        }

    var selectedTag: String?
        get() = _selectedTag.value
        set(value) {
            _selectedTag.value = value
        }
}
