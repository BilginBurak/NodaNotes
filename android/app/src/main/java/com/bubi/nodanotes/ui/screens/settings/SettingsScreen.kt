package com.bubi.nodanotes.ui.screens.settings

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.shape.RoundedCornerShape
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
                            5 -> MaintenanceTab()
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

        var autoSaveExpanded by remember { mutableStateOf(false) }
        val autoSaveOptions = listOf(
            1000 to "1 sec",
            3000 to "3 sec",
            5000 to "5 sec",
            10000 to "10 sec",
            30000 to "30 sec",
            60000 to "1 min"
        )
        val currentAutoSaveLabel = autoSaveOptions.find { it.first == settings.editor.auto_save_delay_ms }?.second ?: "${settings.editor.auto_save_delay_ms / 1000} sec"

        Column {
            Text("Auto Save Delay", style = MaterialTheme.typography.bodyLarge)
            Text("Select how long to wait after typing stops before saving changes", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(modifier = Modifier.height(8.dp))
            Box {
                OutlinedCard(
                    modifier = Modifier.fillMaxWidth().clickable { autoSaveExpanded = true }
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(16.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(currentAutoSaveLabel, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.SemiBold)
                        Icon(Icons.Default.ArrowDropDown, contentDescription = null)
                    }
                }
                DropdownMenu(
                    expanded = autoSaveExpanded,
                    onDismissRequest = { autoSaveExpanded = false }
                ) {
                    autoSaveOptions.forEach { (delayMs, label) ->
                        DropdownMenuItem(
                            text = { Text(label) },
                            onClick = {
                                viewModel.updateEditor(settings.editor.font_size, settings.editor.typography, settings.editor.show_word_count, delayMs)
                                autoSaveExpanded = false
                            }
                        )
                    }
                }
            }
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

        var autoSyncExpanded by remember { mutableStateOf(false) }
        val autoSyncOptions = listOf(
            0L to "Disabled",
            120L to "2 mins",
            300L to "5 mins",
            600L to "10 mins",
            1800L to "30 mins",
            3600L to "1 hour"
        )
        val currentIntervalSecs = interval.toLongOrNull() ?: 0L
        val currentAutoSyncLabel = autoSyncOptions.find { it.first == currentIntervalSecs }?.second ?: "${currentIntervalSecs / 60} mins"

        var showSaveSuccess by remember { mutableStateOf(false) }
        if (showSaveSuccess) {
            LaunchedEffect(Unit) {
                kotlinx.coroutines.delay(3000)
                showSaveSuccess = false
            }
        }

        Column {
            Text("Auto Sync Interval", style = MaterialTheme.typography.bodyLarge)
            Text("Select how often the background WebDAV synchronization should run", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(modifier = Modifier.height(8.dp))
            Box {
                OutlinedCard(
                    modifier = Modifier.fillMaxWidth().clickable { autoSyncExpanded = true }
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(16.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(currentAutoSyncLabel, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.SemiBold)
                        Icon(Icons.Default.ArrowDropDown, contentDescription = null)
                    }
                }
                DropdownMenu(
                    expanded = autoSyncExpanded,
                    onDismissRequest = { autoSyncExpanded = false }
                ) {
                    autoSyncOptions.forEach { (secs, label) ->
                        DropdownMenuItem(
                            text = { Text(label) },
                            onClick = {
                                interval = secs.toString()
                                autoSyncExpanded = false
                            }
                        )
                    }
                }
            }
        }

        if (showSaveSuccess) {
            Card(
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.primaryContainer),
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    modifier = Modifier.padding(16.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(Icons.Default.CheckCircle, contentDescription = null, tint = MaterialTheme.colorScheme.onPrimaryContainer)
                    Spacer(modifier = Modifier.width(12.dp))
                    Text("Configuration saved successfully!", color = MaterialTheme.colorScheme.onPrimaryContainer, style = MaterialTheme.typography.bodyMedium)
                }
            }
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Button(
                onClick = {
                    viewModel.updateSync(webdavUrl, username, password.ifEmpty { null }, interval.toLongOrNull() ?: 0L)
                    showSaveSuccess = true
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

    var localRetentionDays by remember(settings.history.retention_days) { mutableStateOf(settings.history.retention_days.toFloat()) }
    var localMaxSnapshots by remember(settings.history.max_snapshots_per_note) { mutableStateOf(settings.history.max_snapshots_per_note.toFloat()) }
    var localEmptyTrashDays by remember(settings.history.empty_trash_after_days) { mutableStateOf(settings.history.empty_trash_after_days.toFloat()) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(scrollState)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp)
    ) {
        Text("Backup & History Options", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)

        Column {
            Text("Snapshot Retention Days (${localRetentionDays.toInt()} days)", style = MaterialTheme.typography.bodyLarge)
            Slider(
                value = localRetentionDays,
                onValueChange = { localRetentionDays = it },
                onValueChangeFinished = {
                    viewModel.updateHistory(localRetentionDays.toInt(), settings.history.max_snapshots_per_note, settings.history.empty_trash_after_days)
                },
                valueRange = 1f..365f,
                steps = 364
            )
        }

        HorizontalDivider()

        Column {
            Text("Max Snapshots Per Note (${localMaxSnapshots.toInt()} versions)", style = MaterialTheme.typography.bodyLarge)
            Slider(
                value = localMaxSnapshots,
                onValueChange = { localMaxSnapshots = it },
                onValueChangeFinished = {
                    viewModel.updateHistory(settings.history.retention_days, localMaxSnapshots.toInt(), settings.history.empty_trash_after_days)
                },
                valueRange = 5f..100f,
                steps = 95
            )
        }

        HorizontalDivider()

        Column {
            Text("Empty Trash Automatically After (${localEmptyTrashDays.toInt()} days)", style = MaterialTheme.typography.bodyLarge)
            Slider(
                value = localEmptyTrashDays,
                onValueChange = { localEmptyTrashDays = it },
                onValueChangeFinished = {
                    viewModel.updateHistory(settings.history.retention_days, settings.history.max_snapshots_per_note, localEmptyTrashDays.toInt())
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
fun MaintenanceTab(
    viewModel: com.bubi.nodanotes.ui.screens.maintenance.MaintenanceViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsState()
    val duplicates by viewModel.duplicates.collectAsState()
    val remnants by viewModel.orphanedRemnants.collectAsState()
    val attachments by viewModel.orphanedAttachments.collectAsState()
    val vaultPath = viewModel.vaultPath

    var previewFileTitle by remember { mutableStateOf<String?>(null) }
    var previewFilePath by remember { mutableStateOf<String?>(null) }
    var previewFileContent by remember { mutableStateOf<String?>(null) }
    var previewFileMime by remember { mutableStateOf<String?>(null) }

    val scrollState = rememberScrollState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(scrollState)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text("Vault Maintenance & Diagnostics", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)

        when (val state = uiState) {
            is com.bubi.nodanotes.ui.screens.maintenance.MaintenanceUiState.Loading -> {
                Box(modifier = Modifier.fillMaxWidth().padding(16.dp), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator(modifier = Modifier.size(24.dp))
                }
            }
            is com.bubi.nodanotes.ui.screens.maintenance.MaintenanceUiState.Success -> {
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.primaryContainer),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(modifier = Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                        Icon(Icons.Default.CheckCircle, contentDescription = null, tint = MaterialTheme.colorScheme.onPrimaryContainer)
                        Spacer(modifier = Modifier.width(12.dp))
                        Text(state.message, color = MaterialTheme.colorScheme.onPrimaryContainer, style = MaterialTheme.typography.bodyMedium)
                    }
                }
            }
            is com.bubi.nodanotes.ui.screens.maintenance.MaintenanceUiState.Error -> {
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(modifier = Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                        Icon(Icons.Default.Error, contentDescription = null, tint = MaterialTheme.colorScheme.onErrorContainer)
                        Spacer(modifier = Modifier.width(12.dp))
                        Text(state.message, color = MaterialTheme.colorScheme.onErrorContainer, style = MaterialTheme.typography.bodyMedium)
                    }
                }
            }
            else -> {}
        }

        // Section 1: Optimizations
        Card(
            shape = RoundedCornerShape(12.dp),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f))
        ) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text("Database Administration", fontWeight = FontWeight.Bold, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)
                
                Text("Rebuild Cache", fontWeight = FontWeight.SemiBold, style = MaterialTheme.typography.bodyMedium)
                Text("Scans all markdown files, regenerates SQLite indices, and updates metadata.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Button(onClick = { viewModel.rebuildCache() }, modifier = Modifier.fillMaxWidth()) {
                    Text("Rebuild Index Cache")
                }

                HorizontalDivider(modifier = Modifier.padding(vertical = 4.dp))

                Text("Optimize FTS5 Search", fontWeight = FontWeight.SemiBold, style = MaterialTheme.typography.bodyMedium)
                Text("Runs SQLite FTS5 database optimization routine to pack search indices.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Button(onClick = { viewModel.optimizeFts() }, modifier = Modifier.fillMaxWidth()) {
                    Text("Optimize Search Index")
                }

                HorizontalDivider(modifier = Modifier.padding(vertical = 4.dp))

                Text("Synchronization Self-Healing", fontWeight = FontWeight.SemiBold, style = MaterialTheme.typography.bodyMedium)
                Text("Resets sync tracking cache or clears local pending operations queue.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedButton(onClick = { viewModel.clearRemoteCache() }, modifier = Modifier.weight(1f)) {
                        Text("Reset Sync State")
                    }
                    OutlinedButton(onClick = { viewModel.resetSyncQueue() }, modifier = Modifier.weight(1f)) {
                        Text("Clear Queue")
                    }
                }
            }
        }

        // Section 2: Duplicates
        Card(
            shape = RoundedCornerShape(12.dp),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f))
        ) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text("Duplicate Note Diagnostics", fontWeight = FontWeight.Bold, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)
                Text("Checks for multiple files containing the exact same Note ID (ULID).", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                
                Button(onClick = { viewModel.scanDuplicates() }, modifier = Modifier.fillMaxWidth()) {
                    Text("Scan for Duplicate Notes")
                }

                if (duplicates.isNotEmpty()) {
                    Spacer(modifier = Modifier.height(4.dp))
                    duplicates.forEach { group ->
                        Card(
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.2f)),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Column(modifier = Modifier.padding(12.dp)) {
                                Text("Title: ${group.title}", fontWeight = FontWeight.Bold, style = MaterialTheme.typography.bodyMedium)
                                Text("ID: ${group.note_id}", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Spacer(modifier = Modifier.height(6.dp))
                                group.files.forEach { file ->
                                    Row(
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .clickable {
                                                try {
                                                    val f = if (vaultPath != null) java.io.File(vaultPath, file.path) else java.io.File(file.path)
                                                    val text = if (f.exists()) f.readText() else "File not found"
                                                    previewFileTitle = group.title
                                                    previewFilePath = f.absolutePath
                                                    previewFileContent = text
                                                    previewFileMime = null
                                                } catch (e: Exception) {
                                                    previewFileTitle = group.title
                                                    previewFilePath = file.path
                                                    previewFileContent = "Failed to read content: ${e.message}"
                                                    previewFileMime = null
                                                }
                                            }
                                            .padding(vertical = 4.dp),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically
                                    ) {
                                        Column(modifier = Modifier.weight(1f)) {
                                            Text(file.path, style = MaterialTheme.typography.bodySmall, fontWeight = FontWeight.SemiBold)
                                            Text("Size: ${file.size_bytes} B | Mod: ${file.modified_at}", style = MaterialTheme.typography.labelSmall)
                                        }
                                        IconButton(onClick = { viewModel.deleteDuplicateFile(file.path) }) {
                                            Icon(Icons.Default.Delete, contentDescription = "Delete duplicate", tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(20.dp))
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Section 3: Remnants & Orphaned files
        Card(
            shape = RoundedCornerShape(12.dp),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f))
        ) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text("Orphaned Remnants & Attachments", fontWeight = FontWeight.Bold, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)
                Text("Clean up old history snapshots, conflict files, or unused attachment media.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                    Button(
                        onClick = {
                            viewModel.scanOrphanedRemnants()
                            viewModel.scanOrphanedAttachments()
                        },
                        modifier = Modifier.weight(1f)
                    ) {
                        Text("Scan Remnants")
                    }
                    if (remnants.isNotEmpty()) {
                        Button(
                            onClick = { viewModel.deleteAllOrphanedRemnants() },
                            colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error),
                            modifier = Modifier.weight(1f)
                        ) {
                            Text("Delete All")
                        }
                    }
                }

                if (remnants.isNotEmpty()) {
                    Text("Orphaned History & Conflicts", fontWeight = FontWeight.Bold, style = MaterialTheme.typography.bodyMedium)
                    remnants.forEach { file ->
                        Card(
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f)),
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable {
                                    try {
                                        val f = if (vaultPath != null) java.io.File(vaultPath, file.relative_path) else java.io.File(file.relative_path)
                                        val text = if (f.exists()) f.readText() else "File not found"
                                        previewFileTitle = file.title
                                        previewFilePath = f.absolutePath
                                        previewFileContent = text
                                        previewFileMime = null
                                    } catch (e: Exception) {
                                        previewFileTitle = file.title
                                        previewFilePath = file.relative_path
                                        previewFileContent = "Failed to read content: ${e.message}"
                                        previewFileMime = null
                                    }
                                }
                        ) {
                            Row(modifier = Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(file.title, fontWeight = FontWeight.SemiBold, style = MaterialTheme.typography.bodySmall)
                                    Text("Path: ${file.relative_path}", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                    Text("Type: ${file.file_type} | Size: ${file.size_bytes} B", style = MaterialTheme.typography.labelSmall)
                                }
                                IconButton(onClick = { viewModel.deleteOrphanedFile(file.relative_path) }) {
                                    Icon(Icons.Default.Delete, contentDescription = "Delete remnant", tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(20.dp))
                                }
                            }
                        }
                    }
                }

                if (attachments.isNotEmpty()) {
                    Text("Orphaned Attachments", fontWeight = FontWeight.Bold, style = MaterialTheme.typography.bodyMedium)
                    attachments.forEach { attachment ->
                        Card(
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f)),
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable {
                                    val filePath = "$vaultPath/.noda/attachments/${attachment.name}"
                                    val isImage = attachment.name.endsWith(".jpg", ignoreCase = true) ||
                                            attachment.name.endsWith(".png", ignoreCase = true) ||
                                            attachment.name.endsWith(".jpeg", ignoreCase = true)
                                    previewFileTitle = attachment.name
                                    previewFilePath = filePath
                                    previewFileContent = ""
                                    previewFileMime = if (isImage) "image/jpeg" else "text/plain"
                                }
                        ) {
                            Row(modifier = Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(attachment.name, fontWeight = FontWeight.SemiBold, style = MaterialTheme.typography.bodySmall)
                                    Text("Size: ${attachment.size_bytes} bytes", style = MaterialTheme.typography.labelSmall)
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Preview Dialog Overlay
    previewFileTitle?.let { title ->
        com.bubi.nodanotes.ui.components.FilePreviewDialog(
            title = title,
            content = previewFileContent ?: "",
            filePath = previewFilePath,
            mimeType = previewFileMime,
            onDismiss = {
                previewFileTitle = null
                previewFilePath = null
                previewFileContent = null
                previewFileMime = null
            }
        )
    }
}
