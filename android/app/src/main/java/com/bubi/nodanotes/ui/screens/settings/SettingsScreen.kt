package com.bubi.nodanotes.ui.screens.settings

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.bubi.nodanotes.data.model.SettingsDto

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    onBackClick: () -> Unit,
    onNavigateToMaintenance: () -> Unit,
    onNavigateToVaultSelector: () -> Unit,
    viewModel: SettingsViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsState()
    var selectedTab by remember { mutableStateOf(0) }
    val tabTitles = listOf("Appearance", "Editor", "Sync", "History", "Vaults", "Maintenance")

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Settings", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold) },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(imageVector = Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surface)
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            ScrollableTabRow(
                selectedTabIndex = selectedTab,
                edgePadding = 16.dp,
                containerColor = MaterialTheme.colorScheme.surface,
                contentColor = MaterialTheme.colorScheme.primary
            ) {
                tabTitles.forEachIndexed { index, title ->
                    Tab(
                        selected = selectedTab == index,
                        onClick = { selectedTab = index },
                        text = { Text(title, fontWeight = if (selectedTab == index) FontWeight.Bold else FontWeight.Normal) }
                    )
                }
            }

            Surface(
                modifier = Modifier
                    .fillMaxWidth()
                    .weight(1f),
                color = MaterialTheme.colorScheme.background
            ) {
                when (val state = uiState) {
                    is SettingsUiState.Loading -> {
                        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                            CircularProgressIndicator()
                        }
                    }
                    is SettingsUiState.Success -> {
                        when (selectedTab) {
                            0 -> AppearanceTab(state.settings, viewModel)
                            1 -> EditorTab(state.settings, viewModel)
                            2 -> SyncTab(state.settings, viewModel)
                            3 -> HistoryTab(state.settings, viewModel)
                            4 -> VaultsTab(state.recentVaults, state.currentVault, onNavigateToVaultSelector)
                            5 -> MaintenanceTab(onNavigateToMaintenance)
                        }
                    }
                    is SettingsUiState.Error -> {
                        Box(modifier = Modifier.fillMaxSize().padding(16.dp), contentAlignment = Alignment.Center) {
                            Text(state.message, color = MaterialTheme.colorScheme.error)
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun AppearanceTab(settings: SettingsDto, viewModel: SettingsViewModel) {
    val scrollState = rememberScrollState()
    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(scrollState)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp)
    ) {
        Text("Theme Mode", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
        
        val themes = listOf("system" to "System Default", "light" to "Light", "dark" to "Dark")
        themes.forEach { (mode, label) ->
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { viewModel.updateAppearance(mode, settings.appearance.accent_color) }
                    .padding(vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                RadioButton(
                    selected = settings.appearance.theme == mode,
                    onClick = { viewModel.updateAppearance(mode, settings.appearance.accent_color) }
                )
                Spacer(modifier = Modifier.width(16.dp))
                Text(label, style = MaterialTheme.typography.bodyLarge)
            }
        }

        HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

        Text("Accent Color", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
        val colors = listOf("default" to "Default Blue", "purple" to "Deep Purple", "green" to "Forest Green", "amber" to "Amber Gold")
        colors.forEach { (col, name) ->
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { viewModel.updateAppearance(settings.appearance.theme, col) }
                    .padding(vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                RadioButton(
                    selected = settings.appearance.accent_color == col,
                    onClick = { viewModel.updateAppearance(settings.appearance.theme, col) }
                )
                Spacer(modifier = Modifier.width(16.dp))
                Text(name, style = MaterialTheme.typography.bodyLarge)
            }
        }
    }
}

@Composable
fun EditorTab(settings: SettingsDto, viewModel: SettingsViewModel) {
    val scrollState = rememberScrollState()
    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(scrollState)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp)
    ) {
        Text("Font Configuration", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
        
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text("Font Size (${settings.editor.font_size} sp)", style = MaterialTheme.typography.bodyLarge)
            Row {
                IconButton(onClick = {
                    if (settings.editor.font_size > 10) {
                        viewModel.updateEditor(settings.editor.font_size - 1, settings.editor.typography, settings.editor.show_word_count, settings.editor.auto_save_delay_ms)
                    }
                }) {
                    Icon(imageVector = Icons.Default.Remove, contentDescription = "Decrease font")
                }
                IconButton(onClick = {
                    if (settings.editor.font_size < 30) {
                        viewModel.updateEditor(settings.editor.font_size + 1, settings.editor.typography, settings.editor.show_word_count, settings.editor.auto_save_delay_ms)
                    }
                }) {
                    Icon(imageVector = Icons.Default.Add, contentDescription = "Increase font")
                }
            }
        }

        HorizontalDivider()

        Text("Typography style", style = MaterialTheme.typography.bodyLarge)
        val typographies = listOf("monospace" to "Monospace Code", "sans-serif" to "Modern Sans-Serif", "serif" to "Classic Serif")
        typographies.forEach { (type, label) ->
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { viewModel.updateEditor(settings.editor.font_size, type, settings.editor.show_word_count, settings.editor.auto_save_delay_ms) }
                    .padding(vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                RadioButton(
                    selected = settings.editor.typography == type,
                    onClick = { viewModel.updateEditor(settings.editor.font_size, type, settings.editor.show_word_count, settings.editor.auto_save_delay_ms) }
                )
                Spacer(modifier = Modifier.width(16.dp))
                Text(label, style = MaterialTheme.typography.bodyLarge, fontFamily = if (type == "monospace") FontFamily.Monospace else null)
            }
        }

        HorizontalDivider()

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column {
                Text("Show Word Count", style = MaterialTheme.typography.bodyLarge)
                Text("Displays word counts below the note title in the editor", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Switch(
                checked = settings.editor.show_word_count,
                onCheckedChange = { viewModel.updateEditor(settings.editor.font_size, settings.editor.typography, it, settings.editor.auto_save_delay_ms) }
            )
        }

        HorizontalDivider()

        Column {
            Text("Auto Save Delay (ms)", style = MaterialTheme.typography.bodyLarge)
            Spacer(modifier = Modifier.height(8.dp))
            OutlinedTextField(
                value = settings.editor.auto_save_delay_ms.toString(),
                onValueChange = {
                    val value = it.toIntOrNull() ?: 1500
                    viewModel.updateEditor(settings.editor.font_size, settings.editor.typography, settings.editor.show_word_count, value)
                },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.fillMaxWidth()
            )
        }
    }
}

@Composable
fun SyncTab(settings: SettingsDto, viewModel: SettingsViewModel) {
    val scrollState = rememberScrollState()
    var webdavUrl by remember { mutableStateOf(settings.sync.webdav_url) }
    var username by remember { mutableStateOf(settings.sync.webdav_username) }
    var password by remember { mutableStateOf(settings.sync.webdav_password ?: "") }
    var interval by remember { mutableStateOf(settings.sync.interval_secs.toString()) }
    var showPassword by remember { mutableStateOf(false) }

    val testing by viewModel.isTestingConnection.collectAsState()
    val testResult by viewModel.connectionTestResult.collectAsState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(scrollState)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text("WebDAV Sync Config", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)

        OutlinedTextField(
            value = webdavUrl,
            onValueChange = { webdavUrl = it },
            label = { Text("WebDAV Server URL") },
            placeholder = { Text("https://example.com/nextcloud/remote.php/dav/files/user/") },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true
        )

        OutlinedTextField(
            value = username,
            onValueChange = { username = it },
            label = { Text("Username") },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true
        )

        OutlinedTextField(
            value = password,
            onValueChange = { password = it },
            label = { Text("Password / App Password") },
            visualTransformation = if (showPassword) VisualTransformation.None else PasswordVisualTransformation(),
            trailingIcon = {
                IconButton(onClick = { showPassword = !showPassword }) {
                    Icon(imageVector = if (showPassword) Icons.Default.VisibilityOff else Icons.Default.Visibility, contentDescription = "Toggle password")
                }
            },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true
        )

        OutlinedTextField(
            value = interval,
            onValueChange = { interval = it },
            label = { Text("Auto Sync Interval (Seconds)") },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            modifier = Modifier.fillMaxWidth(),
            singleLine = true
        )

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Button(
                onClick = {
                    viewModel.updateSync(webdavUrl, username, password.ifEmpty { null }, interval.toLongOrNull() ?: 0L)
                },
                modifier = Modifier.weight(1f)
            ) {
                Text("Save Config")
            }

            OutlinedButton(
                onClick = {
                    viewModel.testWebdavConnection(webdavUrl, username, password.ifEmpty { null })
                },
                enabled = !testing,
                modifier = Modifier.weight(1f)
            ) {
                if (testing) {
                    CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
                } else {
                    Text("Test Connection")
                }
            }
        }

        testResult?.let { result ->
            LaunchedEffect(result) {
                // Clear state after reading it or displaying snackbar/alert in settings UI
            }
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = if (result.isSuccess) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.errorContainer
                ),
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    modifier = Modifier.padding(16.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(
                        imageVector = if (result.isSuccess) Icons.Default.CheckCircle else Icons.Default.Error,
                        contentDescription = null,
                        tint = if (result.isSuccess) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onErrorContainer
                    )
                    Spacer(modifier = Modifier.width(12.dp))
                    Text(
                        text = if (result.isSuccess) "Connection test succeeded!" else "Connection failed: ${result.exceptionOrNull()?.message}",
                        color = if (result.isSuccess) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onErrorContainer,
                        style = MaterialTheme.typography.bodyMedium
                    )
                }
            }
        }
    }
}

@Composable
fun HistoryTab(settings: SettingsDto, viewModel: SettingsViewModel) {
    val scrollState = rememberScrollState()
    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(scrollState)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp)
    ) {
        Text("Backup & History Options", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)

        Column {
            Text("Snapshot Retention Days (${settings.history.retention_days} days)", style = MaterialTheme.typography.bodyLarge)
            Slider(
                value = settings.history.retention_days.toFloat(),
                onValueChange = {
                    viewModel.updateHistory(it.toInt(), settings.history.max_snapshots_per_note, settings.history.empty_trash_after_days)
                },
                valueRange = 1f..365f,
                steps = 364
            )
        }

        HorizontalDivider()

        Column {
            Text("Max Snapshots Per Note (${settings.history.max_snapshots_per_note} versions)", style = MaterialTheme.typography.bodyLarge)
            Slider(
                value = settings.history.max_snapshots_per_note.toFloat(),
                onValueChange = {
                    viewModel.updateHistory(settings.history.retention_days, it.toInt(), settings.history.empty_trash_after_days)
                },
                valueRange = 5f..100f,
                steps = 95
            )
        }

        HorizontalDivider()

        Column {
            Text("Empty Trash Automatically After (${settings.history.empty_trash_after_days} days)", style = MaterialTheme.typography.bodyLarge)
            Slider(
                value = settings.history.empty_trash_after_days.toFloat(),
                onValueChange = {
                    viewModel.updateHistory(settings.history.retention_days, settings.history.max_snapshots_per_note, it.toInt())
                },
                valueRange = 1f..90f,
                steps = 89
            )
        }
    }
}

@Composable
fun VaultsTab(recentVaults: List<String>, currentVault: String, onChangeVaultClick: () -> Unit) {
    val scrollState = rememberScrollState()
    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(scrollState)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text("Current Vault", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
        Card(
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text("Path:", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(currentVault, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.SemiBold)
                Spacer(modifier = Modifier.height(16.dp))
                Button(onClick = onChangeVaultClick) {
                    Text("Change Active Vault")
                }
            }
        }

        Spacer(modifier = Modifier.height(8.dp))
        Text("Recent Vaults", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
        recentVaults.forEach { vault ->
            Card(
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f)),
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    modifier = Modifier.padding(12.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(imageVector = Icons.Default.Folder, contentDescription = null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
                    Spacer(modifier = Modifier.width(12.dp))
                    Text(vault, style = MaterialTheme.typography.bodyMedium)
                }
            }
        }
    }
}

@Composable
fun MaintenanceTab(onNavigateToMaintenance: () -> Unit) {
    Box(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        contentAlignment = Alignment.Center
    ) {
        Card(
            modifier = Modifier
                .fillMaxWidth()
                .clickable { onNavigateToMaintenance() }
        ) {
            Column(
                modifier = Modifier.padding(32.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Icon(imageVector = Icons.Default.Build, contentDescription = null, modifier = Modifier.size(48.dp), tint = MaterialTheme.colorScheme.primary)
                Spacer(modifier = Modifier.height(16.dp))
                Text("Open Maintenance Panel", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    "Rebuild index cache, run full-text search optimization, clean up duplicate notes, orphaned remnants or attachments.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = androidx.compose.ui.text.style.TextAlign.Center
                )
            }
        }
    }
}
