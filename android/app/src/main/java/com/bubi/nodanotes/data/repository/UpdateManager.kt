package com.bubi.nodanotes.data.repository

import android.content.Context
import android.content.Intent
import android.net.Uri
import com.bubi.nodanotes.data.model.GitHubReleaseResponse
import com.bubi.nodanotes.data.model.UpdateState
import com.bubi.nodanotes.data.preferences.VaultPreferences
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.withContext
import java.io.File
import java.net.HttpURLConnection
import java.net.URL

object UpdateManager {

    private val jsonParser = kotlinx.serialization.json.Json { ignoreUnknownKeys = true }

    private val _updateState = MutableStateFlow<UpdateState>(UpdateState.NoUpdate)
    val updateState: StateFlow<UpdateState> = _updateState

    /**
     * Initializes the UpdateManager by clearing any leftover APKs in the cache folder.
     */
    fun init(context: Context) {
        cleanCacheApks(context)
    }

    /**
     * Semantic version comparison: returns true if current version is strictly less than target version.
     */
    fun isVersionLessThan(current: String, target: String): Boolean {
        val cleanCurrent = current.removePrefix("v").substringBefore("-")
        val cleanTarget = target.removePrefix("v").substringBefore("-")

        val currentParts = cleanCurrent.split(".").mapNotNull { it.toIntOrNull() }
        val targetParts = cleanTarget.split(".").mapNotNull { it.toIntOrNull() }

        val maxLen = maxOf(currentParts.size, targetParts.size)
        for (i in 0 until maxLen) {
            val curr = currentParts.getOrElse(i) { 0 }
            val targ = targetParts.getOrElse(i) { 0 }
            if (curr < targ) return true
            if (curr > targ) return false
        }
        return false
    }

    fun shouldCheckForUpdate(context: Context): Boolean {
        val prefs = VaultPreferences(context)
        val interval = prefs.getUpdateInterval()
        if (interval == "Every Entry") return true

        val lastCheck = prefs.getLastUpdateCheckTime()
        val now = System.currentTimeMillis()
        val elapsed = now - lastCheck

        val limit = when (interval) {
            "Hourly" -> 60 * 60 * 1000L
            "Weekly" -> 7 * 24 * 60 * 60 * 1000L
            else -> 24 * 60 * 60 * 1000L // "Daily"
        }
        return elapsed >= limit
    }

    suspend fun checkForUpdates(context: Context, force: Boolean = false): UpdateState = withContext(Dispatchers.IO) {
        if (!force && !shouldCheckForUpdate(context)) {
            return@withContext _updateState.value
        }

        try {
            val url = URL("https://nodanotes.netlify.app/api/update")
            val connection = url.openConnection() as HttpURLConnection
            connection.requestMethod = "GET"
            connection.connectTimeout = 10000
            connection.readTimeout = 10000
            connection.setRequestProperty("Accept", "application/json")
            connection.setRequestProperty("User-Agent", "NodaNotes-Android")
            connection.connect()

            if (connection.responseCode == HttpURLConnection.HTTP_OK) {
                val responseText = connection.inputStream.bufferedReader().use { it.readText() }
                val response = jsonParser.decodeFromString<GitHubReleaseResponse>(responseText)

                val prefs = VaultPreferences(context)
                prefs.saveLastUpdateCheckTime(System.currentTimeMillis())

                val pInfo = context.packageManager.getPackageInfo(context.packageName, 0)
                val currentVersion = pInfo.versionName ?: "1.0"

                // Extract min_required from release body HTML comment: <!-- min_required: "1.0.0" -->
                val regex = "<!--\\s*min_required:\\s*\"?([^\"]+?)\"?\\s*-->".toRegex()
                val matchResult = regex.find(response.body)
                val minRequired = matchResult?.groupValues?.get(1) ?: "1.0.0"

                val apkUrl = selectBestAsset(response.assets)

                val state = when {
                    isVersionLessThan(currentVersion, minRequired) -> {
                        UpdateState.MandatoryUpdate(response.tag_name, apkUrl)
                    }
                    isVersionLessThan(currentVersion, response.tag_name) -> {
                        UpdateState.FlexibleUpdate(response.tag_name, apkUrl)
                    }
                    else -> {
                        UpdateState.NoUpdate
                    }
                }

                _updateState.value = state
                return@withContext state
            }
        } catch (e: Exception) {
            e.printStackTrace()
        }

        return@withContext _updateState.value
    }

    suspend fun downloadApk(context: Context, apkUrl: String): File? = withContext(Dispatchers.IO) {
        cleanCacheApks(context)

        try {
            val url = URL(apkUrl)
            val connection = url.openConnection() as HttpURLConnection
            connection.requestMethod = "GET"
            connection.connectTimeout = 20000
            connection.readTimeout = 20000
            connection.connect()

            if (connection.responseCode == HttpURLConnection.HTTP_OK) {
                val cacheFile = File(context.cacheDir, "update_${System.currentTimeMillis()}.apk")
                connection.inputStream.use { input ->
                    cacheFile.outputStream().use { output ->
                        input.copyTo(output)
                    }
                }
                return@withContext cacheFile
            }
        } catch (e: Exception) {
            e.printStackTrace()
        }
        return@withContext null
    }

    private fun selectBestAsset(assets: List<com.bubi.nodanotes.data.model.GitHubAsset>): String {
        if (assets.isEmpty()) return ""

        val supportedAbis = android.os.Build.SUPPORTED_ABIS.map { it.lowercase() }

        // Tier 1: Match by device CPU architectures in order of preference (e.g. arm64-v8a, armeabi-v7a)
        for (abi in supportedAbis) {
            val match = assets.find { asset ->
                val nameLower = asset.name.lowercase()
                nameLower.endsWith(".apk") && nameLower.contains(abi)
            }
            if (match != null) {
                return match.browser_download_url
            }
        }

        // Tier 2: Universal Fallback
        val universalMatch = assets.find { asset ->
            val nameLower = asset.name.lowercase()
            nameLower.endsWith(".apk") && nameLower.contains("universal")
        }
        if (universalMatch != null) {
            return universalMatch.browser_download_url
        }

        // Tier 3: First package matching the .apk extension suffix
        val firstApkMatch = assets.find { asset ->
            asset.name.lowercase().endsWith(".apk")
        }
        if (firstApkMatch != null) {
            return firstApkMatch.browser_download_url
        }

        // Tier 4: Ultimate raw fallback to first available asset url
        return assets.firstOrNull()?.browser_download_url ?: ""
    }

    fun installApk(context: Context, apkFile: File) {
        try {
            val authority = "com.bubi.nodanotes.fileprovider"
            val uri: Uri = androidx.core.content.FileProvider.getUriForFile(context, authority, apkFile)
            val intent = Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(uri, "application/vnd.android.package-archive")
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            context.startActivity(intent)
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    fun cleanCacheApks(context: Context) {
        try {
            val cacheDir = context.cacheDir
            val files = cacheDir.listFiles()
            if (files != null) {
                for (file in files) {
                    if (file.name.endsWith(".apk")) {
                        file.delete()
                    }
                }
            }
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }
}
