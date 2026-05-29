package com.bubi.nodanotes.data.preferences

import android.content.Context
import android.content.SharedPreferences
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import java.io.IOException
import java.security.GeneralSecurityException

class VaultPreferences(private val context: Context) {

    private val sharedPrefs: SharedPreferences = context.getSharedPreferences(
        "noda_prefs",
        Context.MODE_PRIVATE
    )

    private val encryptedPrefs: SharedPreferences by lazy {
        try {
            val masterKey = MasterKey.Builder(context)
                .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                .build()

            EncryptedSharedPreferences.create(
                context,
                "secure_noda_prefs",
                masterKey,
                EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
            )
        } catch (e: Exception) {
            // Fallback to normal shared preferences under security exceptions (e.g. key store issues in tests/emulators)
            context.getSharedPreferences("secure_noda_prefs_fallback", Context.MODE_PRIVATE)
        }
    }

    companion object {
        private const val KEY_VAULT_PATH = "vault_path"
        private const val KEY_LAST_SYNC_REPORT = "last_sync_report"
        private const val KEY_DARK_MODE = "dark_mode"
        private const val KEY_WEBDAV_PASSWORD = "webdav_password"
        private const val KEY_RECENT_VAULTS = "recent_vaults"
        private const val KEY_RECENT_SEARCHES = "recent_searches"
    }

    fun saveVaultPath(path: String) {
        sharedPrefs.edit().putString(KEY_VAULT_PATH, path).apply()
        addRecentVault(path)
    }

    fun addRecentVault(path: String) {
        val recents = getRecentVaults().toMutableList()
        if (recents.contains(path)) {
            recents.remove(path)
        }
        recents.add(0, path)
        val limit = 5
        val trimmed = if (recents.size > limit) recents.take(limit) else recents
        sharedPrefs.edit().putString(KEY_RECENT_VAULTS, trimmed.joinToString("|")).apply()
    }

    fun getRecentVaults(): List<String> {
        val raw = sharedPrefs.getString(KEY_RECENT_VAULTS, null) ?: return emptyList()
        return raw.split("|").filter { it.isNotEmpty() }
    }

    fun getVaultPath(): String? {
        return sharedPrefs.getString(KEY_VAULT_PATH, null)
    }

    fun saveLastSyncReport(reportJson: String) {
        sharedPrefs.edit().putString(KEY_LAST_SYNC_REPORT, reportJson).apply()
    }

    fun getLastSyncReport(): String? {
        return sharedPrefs.getString(KEY_LAST_SYNC_REPORT, null)
    }

    fun saveDarkModePreference(pref: String) {
        sharedPrefs.edit().putString(KEY_DARK_MODE, pref).apply()
    }

    fun getDarkModePreference(): String {
        return sharedPrefs.getString(KEY_DARK_MODE, "system") ?: "system"
    }

    fun saveWebdavPassword(password: String?) {
        if (password == null) {
            encryptedPrefs.edit().remove(KEY_WEBDAV_PASSWORD).apply()
        } else {
            encryptedPrefs.edit().putString(KEY_WEBDAV_PASSWORD, password).apply()
        }
    }

    fun getWebdavPassword(): String? {
        return encryptedPrefs.getString(KEY_WEBDAV_PASSWORD, null)
    }

    fun getRecentSearches(): List<String> {
        val raw = sharedPrefs.getString(KEY_RECENT_SEARCHES, null) ?: return emptyList()
        return raw.split("|").filter { it.isNotEmpty() }
    }

    fun addRecentSearch(query: String) {
        if (query.trim().isEmpty()) return
        val recents = getRecentSearches().toMutableList()
        if (recents.contains(query)) {
            recents.remove(query)
        }
        recents.add(0, query)
        val limit = 10
        val trimmed = if (recents.size > limit) recents.take(limit) else recents
        sharedPrefs.edit().putString(KEY_RECENT_SEARCHES, trimmed.joinToString("|")).apply()
    }

    fun clearRecentSearches() {
        sharedPrefs.edit().remove(KEY_RECENT_SEARCHES).apply()
    }
}
