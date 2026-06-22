package com.bubi.nodanotes.data.repository

import com.bubi.nodanotes.RustCore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable

class SecurityRepository : BaseRepository() {

    @Serializable
    private data class SetMasterPasswordParams(val password: String)

    @Serializable
    private data class UnlockVaultSessionParams(val password: String)

    @Serializable
    private data class ToggleNoteEncryptionParams(val note_id: String)

    @Serializable
    private data class SetVaultTimeoutSettingParams(val timeout: String)

    @Serializable
    private data class ChangeMasterPasswordParams(val old_password: String, val new_password: String)

    suspend fun isVaultConfigured(): Result<Boolean> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.isVaultConfigured("{}")
            // result is like {"configured": true}
            @Serializable
            data class ConfiguredResponse(val configured: Boolean)
            json.decodeFromString(ConfiguredResponse.serializer(), result).configured
        }
    }

    suspend fun setMasterPassword(password: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = SetMasterPasswordParams(password)
            val jsonInput = json.encodeToString(SetMasterPasswordParams.serializer(), params)
            val result = RustCore.setMasterPassword(jsonInput)
            checkError(result)
        }
    }

    suspend fun checkVaultStatus(): Result<String> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.checkVaultStatus("{}")
            // result is like {"status": "Unlocked"}
            @Serializable
            data class StatusResponse(val status: String)
            json.decodeFromString(StatusResponse.serializer(), result).status
        }
    }

    suspend fun unlockVaultSession(password: String): Result<Boolean> = withContext(Dispatchers.IO) {
        runCatching {
            val params = UnlockVaultSessionParams(password)
            val jsonInput = json.encodeToString(UnlockVaultSessionParams.serializer(), params)
            val result = RustCore.unlockVaultSession(jsonInput)
            @Serializable
            data class UnlockResponse(val unlocked: Boolean?, val error: String? = null)
            val res = json.decodeFromString(UnlockResponse.serializer(), result)
            if (res.error != null) {
                throw Exception(res.error)
            }
            res.unlocked ?: false
        }
    }

    suspend fun lockVaultInstantly(): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.lockVaultInstantly("{}")
            checkError(result)
        }
    }

    suspend fun toggleNoteEncryption(noteId: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = ToggleNoteEncryptionParams(noteId)
            val jsonInput = json.encodeToString(ToggleNoteEncryptionParams.serializer(), params)
            val result = RustCore.toggleNoteEncryption(jsonInput)
            checkError(result)
        }
    }

    suspend fun getVaultTimeoutSetting(): Result<String> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.getVaultTimeoutSetting("{}")
            @Serializable
            data class TimeoutResponse(val timeout: String)
            json.decodeFromString(TimeoutResponse.serializer(), result).timeout
        }
    }

    suspend fun setVaultTimeoutSetting(timeout: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = SetVaultTimeoutSettingParams(timeout)
            val jsonInput = json.encodeToString(SetVaultTimeoutSettingParams.serializer(), params)
            val result = RustCore.setVaultTimeoutSetting(jsonInput)
            checkError(result)
        }
    }

    suspend fun changeMasterPassword(oldPassword: String, newPassword: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val params = ChangeMasterPasswordParams(oldPassword, newPassword)
            val jsonInput = json.encodeToString(ChangeMasterPasswordParams.serializer(), params)
            val result = RustCore.changeMasterPassword(jsonInput)
            checkError(result)
        }
    }
}
