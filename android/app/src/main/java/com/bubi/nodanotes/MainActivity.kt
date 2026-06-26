package com.bubi.nodanotes

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.os.Environment
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.ui.Modifier
import androidx.navigation.compose.rememberNavController
import com.bubi.nodanotes.data.preferences.VaultPreferences
import com.bubi.nodanotes.ui.navigation.NodaNavGraph
import com.bubi.nodanotes.ui.navigation.Screen
import com.bubi.nodanotes.ui.theme.NodaTheme
import java.io.File

import com.bubi.nodanotes.ui.components.NodaAppShell
import androidx.lifecycle.lifecycleScope
import androidx.compose.runtime.*
import android.content.SharedPreferences
import kotlinx.coroutines.launch
import kotlinx.coroutines.isActive
import kotlinx.coroutines.Dispatchers
import com.bubi.nodanotes.data.repository.UpdateManager
import com.bubi.nodanotes.data.model.UpdateState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.SystemUpdate
import androidx.compose.ui.Alignment
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.activity.compose.BackHandler
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.foundation.shape.RoundedCornerShape

class MainActivity : ComponentActivity() {

    private lateinit var vaultPreferences: VaultPreferences

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        vaultPreferences = VaultPreferences(this)

        // Initialize UpdateManager and trigger update check on startup
        UpdateManager.init(this)
        lifecycleScope.launch(Dispatchers.IO) {
            UpdateManager.checkForUpdates(this@MainActivity)
        }

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
            // Read theme preference from SharedPreferences reactively
            val context = androidx.compose.ui.platform.LocalContext.current
            val sharedPreferences = remember { context.getSharedPreferences("noda_prefs", android.content.Context.MODE_PRIVATE) }
            var darkModePref by remember { mutableStateOf(sharedPreferences.getString("dark_mode", "system") ?: "system") }
            
            DisposableEffect(sharedPreferences) {
                val listener = SharedPreferences.OnSharedPreferenceChangeListener { prefs, key ->
                    if (key == "dark_mode") {
                        darkModePref = prefs.getString("dark_mode", "system") ?: "system"
                    }
                }
                sharedPreferences.registerOnSharedPreferenceChangeListener(listener)
                onDispose {
                    sharedPreferences.unregisterOnSharedPreferenceChangeListener(listener)
                }
            }

            val darkTheme = when (darkModePref) {
                "dark" -> true
                "light" -> false
                else -> androidx.compose.foundation.isSystemInDarkTheme()
            }
            
            val updateState by UpdateManager.updateState.collectAsState()

            NodaTheme(darkTheme = darkTheme) {
                when (val state = updateState) {
                    is UpdateState.MandatoryUpdate -> {
                        ZenUpdateScreen(
                            tagName = state.tagName,
                            apkUrl = state.apkUrl
                        )
                    }
                    else -> {
                        val navController = rememberNavController()
                        NodaAppShell(
                            navController = navController,
                            startDestination = startDestination,
                            vaultPreferences = vaultPreferences
                        )
                    }
                }
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
                        val lastSync = vaultPreferences.getLastSyncTime()
                        val elapsedSecs = (System.currentTimeMillis() - lastSync) / 1000
                        if (elapsedSecs >= config.interval_secs) {
                            syncRepo.syncNow().onSuccess { report ->
                                vaultPreferences.saveLastSyncTime(System.currentTimeMillis())
                                vaultPreferences.saveLastSyncReport(
                                    kotlinx.serialization.json.Json.encodeToString(
                                        com.bubi.nodanotes.data.model.SyncReportDto.serializer(),
                                        report
                                    )
                                )
                                try {
                                    com.bubi.nodanotes.data.repository.VaultRepository().refreshVault()
                                } catch (e: Exception) {
                                    e.printStackTrace()
                                }
                            }
                        }
                    }
                }
                kotlinx.coroutines.delay(15000)
            }
        }
    }

    private fun stopAutoSyncLoop() {
        autoSyncJob?.cancel()
        autoSyncJob = null
    }

    @Composable
    private fun ZenUpdateScreen(tagName: String, apkUrl: String) {
        // Completely disable back button navigation
        BackHandler(enabled = true) {}

        val context = androidx.compose.ui.platform.LocalContext.current
        val scope = rememberCoroutineScope()
        var isDownloading by remember { mutableStateOf(false) }
        var errorMessage by remember { mutableStateOf<String?>(null) }

        Surface(
            modifier = Modifier.fillMaxSize(),
            color = MaterialTheme.colorScheme.background
        ) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(32.dp),
                contentAlignment = Alignment.Center
            ) {
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.Center
                ) {
                    Icon(
                        imageVector = Icons.Default.SystemUpdate,
                        contentDescription = "Security Update Required",
                        tint = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.size(72.dp)
                    )

                    Spacer(modifier = Modifier.height(24.dp))

                    Text(
                        text = "A mandatory security update is required to preserve local data integrity. Please update NodaNotes to continue.",
                        color = MaterialTheme.colorScheme.onBackground,
                        style = MaterialTheme.typography.titleMedium,
                        textAlign = TextAlign.Center,
                        fontWeight = FontWeight.Medium,
                        lineHeight = 24.sp
                    )

                    Spacer(modifier = Modifier.height(32.dp))

                    Button(
                        onClick = {
                            if (!isDownloading) {
                                isDownloading = true
                                errorMessage = null
                                scope.launch {
                                    val file = UpdateManager.downloadApk(context, apkUrl)
                                    isDownloading = false
                                    if (file != null) {
                                        UpdateManager.installApk(context, file)
                                    } else {
                                        errorMessage = "Download failed. Please check your network connection."
                                    }
                                }
                            }
                        },
                        enabled = !isDownloading,
                        colors = ButtonDefaults.buttonColors(
                            containerColor = MaterialTheme.colorScheme.primary,
                            contentColor = MaterialTheme.colorScheme.onPrimary
                        ),
                        modifier = Modifier
                            .fillMaxWidth(0.8f)
                            .height(48.dp),
                        shape = RoundedCornerShape(8.dp)
                    ) {
                        if (isDownloading) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(20.dp),
                                color = MaterialTheme.colorScheme.onPrimary,
                                strokeWidth = 2.dp
                            )
                            Spacer(modifier = Modifier.width(12.dp))
                            Text("Downloading Update...", fontSize = 15.sp)
                        } else {
                            Text("Download and Install Update", fontSize = 15.sp)
                        }
                    }

                    errorMessage?.let { error ->
                        Spacer(modifier = Modifier.height(16.dp))
                        Text(
                            text = error,
                            color = MaterialTheme.colorScheme.error,
                            style = MaterialTheme.typography.bodySmall,
                            textAlign = TextAlign.Center
                        )
                    }
                }
            }
        }
    }
}