package com.bubi.nodanotes.ui.screens.vault

import android.app.Application
import android.os.Environment
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.bubi.nodanotes.data.preferences.VaultPreferences
import com.bubi.nodanotes.data.repository.VaultRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import java.io.File

class VaultSelectorViewModel(application: Application) : AndroidViewModel(application) {

    private val vaultRepository = VaultRepository()
    private val vaultPreferences = VaultPreferences(application)

    private val _uiState = MutableStateFlow<VaultSelectorUiState>(VaultSelectorUiState.Idle)
    val uiState: StateFlow<VaultSelectorUiState> = _uiState.asStateFlow()

    private val _recentVaults = MutableStateFlow<List<String>>(emptyList())
    val recentVaults: StateFlow<List<String>> = _recentVaults.asStateFlow()

    init {
        loadRecentVaults()
    }

    fun loadRecentVaults() {
        _recentVaults.value = vaultPreferences.getRecentVaults()
    }

    fun checkStoragePermission(): Boolean {
        return Environment.isExternalStorageManager()
    }

    fun initVault(path: String) {
        viewModelScope.launch {
            _uiState.value = VaultSelectorUiState.Loading
            vaultRepository.initVault(path).fold(
                onSuccess = {
                    vaultPreferences.saveVaultPath(path)
                    loadRecentVaults()
                    _uiState.value = VaultSelectorUiState.Success(path)
                },
                onFailure = { error ->
                    _uiState.value = VaultSelectorUiState.Error(error.message ?: "Failed to initialize vault")
                }
            )
        }
    }

    fun createVault(parentPath: String, folderName: String) {
        viewModelScope.launch {
            _uiState.value = VaultSelectorUiState.Loading
            try {
                val parentFile = File(parentPath)
                if (!parentFile.exists()) {
                    parentFile.mkdirs()
                }
                val vaultFile = File(parentFile, folderName)
                if (!vaultFile.exists()) {
                    vaultFile.mkdir()
                }
                initVault(vaultFile.absolutePath)
            } catch (e: Exception) {
                _uiState.value = VaultSelectorUiState.Error(e.message ?: "Failed to create vault folder")
            }
        }
    }

    fun removeRecentVault(path: String) {
        val current = vaultPreferences.getRecentVaults().toMutableList()
        current.remove(path)
        val context = getApplication<Application>()
        context.getSharedPreferences("noda_prefs", android.content.Context.MODE_PRIVATE)
            .edit()
            .putString("recent_vaults", current.joinToString("|"))
            .apply()
        loadRecentVaults()
    }
}

sealed class VaultSelectorUiState {
    object Idle : VaultSelectorUiState()
    object Loading : VaultSelectorUiState()
    data class Success(val vaultPath: String) : VaultSelectorUiState()
    data class Error(val message: String) : VaultSelectorUiState()
}
