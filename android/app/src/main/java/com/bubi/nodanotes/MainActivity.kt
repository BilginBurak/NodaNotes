package com.bubi.nodanotes

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.os.Environment
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.ui.Modifier
import androidx.navigation.compose.rememberNavController
import com.bubi.nodanotes.data.preferences.VaultPreferences
import com.bubi.nodanotes.ui.navigation.NodaNavGraph
import com.bubi.nodanotes.ui.navigation.Screen
import com.bubi.nodanotes.ui.theme.NodaTheme
import java.io.File

import com.bubi.nodanotes.ui.components.NodaAppShell
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.isActive
import kotlinx.coroutines.Dispatchers

class MainActivity : ComponentActivity() {

    private lateinit var vaultPreferences: VaultPreferences

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        vaultPreferences = VaultPreferences(this)

        // Request "All Files Access" if not already granted (required for vault access)
        if (!Environment.isExternalStorageManager()) {
            try {
                val intent = Intent(Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION).apply {
                    data = Uri.parse("package:${packageName}")
                }
                startActivity(intent)
            } catch (e: Exception) {
                val intent = Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION)
                startActivity(intent)
            }
        }

        // Determine start destination and initialize vault if path is already saved
        val savedVaultPath = vaultPreferences.getVaultPath()
        val startDestination = if (Environment.isExternalStorageManager() && savedVaultPath != null) {
            try {
                RustCore.initVault(savedVaultPath)
                Screen.NoteList.route
            } catch (e: Exception) {
                // If JNI init fails, fallback to selector
                Screen.VaultSelector.route
            }
        } else {
            Screen.VaultSelector.route
        }

        setContent {
            NodaTheme {
                val navController = rememberNavController()
                NodaAppShell(
                    navController = navController,
                    startDestination = startDestination,
                    vaultPreferences = vaultPreferences
                )
            }
        }
    }

    private var autoSyncJob: kotlinx.coroutines.Job? = null

    override fun onResume() {
        super.onResume()
        val savedVaultPath = if (::vaultPreferences.isInitialized) vaultPreferences.getVaultPath() else null
        if (savedVaultPath != null) {
            // Re-sync vault for external files immediately
            lifecycleScope.launch(Dispatchers.IO) {
                try {
                    com.bubi.nodanotes.data.repository.VaultRepository().refreshVault()
                } catch (e: Exception) {
                    e.printStackTrace()
                }
            }

            // Start foreground auto-sync timer loop based on config interval
            startAutoSyncLoop()
        }
    }

    override fun onPause() {
        super.onPause()
        stopAutoSyncLoop()
    }

    private fun startAutoSyncLoop() {
        stopAutoSyncLoop()
        autoSyncJob = lifecycleScope.launch(Dispatchers.IO) {
            val syncRepo = com.bubi.nodanotes.data.repository.SyncRepository()
            while (isActive) {
                val configResult = syncRepo.loadSyncConfig()
                if (configResult.isSuccess) {
                    val config = configResult.getOrThrow()
                    if (config.is_configured && config.interval_secs > 0) {
                        // Wait for configured interval
                        kotlinx.coroutines.delay(config.interval_secs * 1000)
                        
                        // Execute Sync in background
                        syncRepo.syncNow().onSuccess { report ->
                            // Update shared state/preferences
                            vaultPreferences.saveLastSyncReport(
                                kotlinx.serialization.json.Json.encodeToString(
                                    com.bubi.nodanotes.data.model.SyncReportDto.serializer(),
                                    report
                                )
                            )
                            // Post local status notification
                            val noteListViewModelClass = Class.forName("com.bubi.nodanotes.ui.screens.notelist.NoteListViewModel")
                            // Broadcast or refresh core vault
                            com.bubi.nodanotes.data.repository.VaultRepository().refreshVault()
                        }
                    } else {
                        // Check again in 30 seconds if sync not fully configured
                        kotlinx.coroutines.delay(30000)
                    }
                } else {
                    kotlinx.coroutines.delay(30000)
                }
            }
        }
    }

    private fun stopAutoSyncLoop() {
        autoSyncJob?.cancel()
        autoSyncJob = null
    }
}