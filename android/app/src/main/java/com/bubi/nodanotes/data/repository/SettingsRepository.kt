package com.bubi.nodanotes.data.repository

import com.bubi.nodanotes.RustCore
import com.bubi.nodanotes.data.model.SettingsDto
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class SettingsRepository : BaseRepository() {

    suspend fun getSettings(): Result<SettingsDto> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.getSettings("{}")
            parseRustResult(result, SettingsDto.serializer())
        }
    }

    suspend fun updateSettings(settings: SettingsDto): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val jsonInput = json.encodeToString(SettingsDto.serializer(), settings)
            val result = RustCore.updateSettings(jsonInput)
            checkError(result)
        }
    }
}
