package com.bubi.nodanotes.ui.screens.notelist

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

object FolderContext {
    private val _currentFolder = MutableStateFlow<String?>(null)
    val currentFolderState: StateFlow<String?> = _currentFolder.asStateFlow()

    var currentFolder: String?
        get() = _currentFolder.value
        set(value) {
            _currentFolder.value = value
        }
}
