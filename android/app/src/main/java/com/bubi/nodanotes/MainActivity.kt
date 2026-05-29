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

    override fun onResume() {
        super.onResume()
        val savedVaultPath = if (::vaultPreferences.isInitialized) vaultPreferences.getVaultPath() else null
        if (savedVaultPath != null) {
            lifecycleScope.launch(Dispatchers.IO) {
                try {
                    com.bubi.nodanotes.data.repository.VaultRepository().refreshVault()
                } catch (e: Exception) {
                    e.printStackTrace()
                }
            }
        }
    }
}