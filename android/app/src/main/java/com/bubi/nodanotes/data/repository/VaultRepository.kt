package com.bubi.nodanotes.data.repository

import com.bubi.nodanotes.RustCore
import com.bubi.nodanotes.data.model.VaultInfoDto
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class VaultRepository : BaseRepository() {

    suspend fun initVault(path: String): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.initVault(path)
            checkError(result)
        }
    }

    suspend fun getVaultInfo(): Result<VaultInfoDto> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.getVaultInfo("{}")
            parseRustResult(result, VaultInfoDto.serializer())
        }
    }

    suspend fun refreshVault(): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val result = RustCore.refreshVault("{}")
            checkError(result)
        }
    }
}
