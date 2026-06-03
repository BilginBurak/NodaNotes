package com.bubi.nodanotes.ui.screens.settings

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.bubi.nodanotes.data.model.*
import com.bubi.nodanotes.data.preferences.VaultPreferences
import com.bubi.nodanotes.data.repository.SettingsRepository
import com.bubi.nodanotes.data.repository.SyncRepository
import com.bubi.nodanotes.data.repository.VaultRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

sealed interface SettingsUiState {
    object Loading : SettingsUiState
    data class Success(val settings: SettingsDto, val recentVaults: List<String>, val currentVault: String) : SettingsUiState
    data class Error(val message: String) : SettingsUiState
}

class SettingsViewModel(application: Application) : AndroidViewModel(application) {

    private val settingsRepository = SettingsRepository()
    private val syncRepository = SyncRepository()
    private val preferences = VaultPreferences(application)
    private val noteRepository = com.bubi.nodanotes.data.repository.NoteRepository()

    private val _uiState = MutableStateFlow<SettingsUiState>(SettingsUiState.Loading)
    val uiState: StateFlow<SettingsUiState> = _uiState.asStateFlow()

    private val _templates = MutableStateFlow<List<NoteListItemDto>>(emptyList())
    val templates: StateFlow<List<NoteListItemDto>> = _templates.asStateFlow()

    private val _connectionTestResult = MutableStateFlow<Result<Unit>?>(null)
    val connectionTestResult: StateFlow<Result<Unit>?> = _connectionTestResult.asStateFlow()

    private val _isTestingConnection = MutableStateFlow(false)
    val isTestingConnection: StateFlow<Boolean> = _isTestingConnection.asStateFlow()

    init {
        loadSettings()
        loadTemplates()
    }

    fun loadTemplates() {
        viewModelScope.launch {
            noteRepository.listNotes(".templates").onSuccess { list ->
                _templates.value = list
            }.onFailure {
                _templates.value = emptyList()
            }
        }
    }

    fun loadSettings() {
        viewModelScope.launch {
            _uiState.value = SettingsUiState.Loading
            settingsRepository.getSettings()
                .onSuccess { settings ->
                    val recents = preferences.getRecentVaults()
                    val current = preferences.getVaultPath() ?: ""
                    _uiState.value = SettingsUiState.Success(settings, recents, current)
                }
                .onFailure { error ->
                    _uiState.value = SettingsUiState.Error(error.message ?: "Failed to load settings")
                }
        }
    }

    fun updateAppearance(theme: String, accentColor: String) {
        val currentState = _uiState.value
        if (currentState is SettingsUiState.Success) {
            val newSettings = currentState.settings.copy(
                appearance = AppearanceSettingsDto(theme, accentColor)
            )
            // Save local preference too
            preferences.saveDarkModePreference(theme)
            saveSettings(newSettings)
        }
    }

    fun updateEditor(fontSize: Int, typography: String, showWordCount: Boolean, autoSaveDelayMs: Int) {
        val currentState = _uiState.value
        if (currentState is SettingsUiState.Success) {
            val newSettings = currentState.settings.copy(
                editor = EditorSettingsDto(fontSize, typography, showWordCount, autoSaveDelayMs)
            )
            saveSettings(newSettings)
        }
    }

    fun updateSync(webdavUrl: String, username: String, password: String?, intervalSecs: Long) {
        val currentState = _uiState.value
        if (currentState is SettingsUiState.Success) {
            val newSettings = currentState.settings.copy(
                sync = SyncConfigDto(webdavUrl, username, password, intervalSecs)
            )
            if (password != null) {
                preferences.saveWebdavPassword(password)
            }
            saveSettings(newSettings)
        }
    }

    fun updateHistory(retentionDays: Int, maxSnapshotsPerNote: Int, emptyTrashAfterDays: Int, snapshotIntervalMins: Int) {
        val currentState = _uiState.value
        if (currentState is SettingsUiState.Success) {
            val newSettings = currentState.settings.copy(
                history = HistorySettingsDto(retentionDays, maxSnapshotsPerNote, emptyTrashAfterDays, snapshotIntervalMins)
            )
            saveSettings(newSettings)
        }
    }

    fun updateDefaultDailyTemplate(templateNoteId: String?) {
        val currentState = _uiState.value
        if (currentState is SettingsUiState.Success) {
            val newSettings = currentState.settings.copy(
                editor = currentState.settings.editor.copy(
                    default_daily_template = templateNoteId
                )
            )
            saveSettings(newSettings)
        }
    }

    private fun saveSettings(newSettings: SettingsDto) {
        viewModelScope.launch {
            settingsRepository.updateSettings(newSettings)
                .onSuccess {
                    val recents = preferences.getRecentVaults()
                    val current = preferences.getVaultPath() ?: ""
                    _uiState.value = SettingsUiState.Success(newSettings, recents, current)
                }
                .onFailure { error ->
                    _uiState.value = SettingsUiState.Error(error.message ?: "Failed to save settings")
                }
        }
    }

    fun testWebdavConnection(webdavUrl: String, username: String, password: String?) {
        viewModelScope.launch {
            _isTestingConnection.value = true
            _connectionTestResult.value = null
            syncRepository.testWebdavConnection(webdavUrl, username, password)
                .onSuccess {
                    _connectionTestResult.value = Result.success(Unit)
                    _isTestingConnection.value = false
                }
                .onFailure { error ->
                    _connectionTestResult.value = Result.failure(error)
                    _isTestingConnection.value = false
                }
        }
    }

    fun clearConnectionTestResult() {
        _connectionTestResult.value = null
    }
}
