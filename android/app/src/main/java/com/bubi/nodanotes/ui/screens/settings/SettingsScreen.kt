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
import kotlinx.coroutines.launch
import androidx.lifecycle.viewmodel.compose.viewModel
import com.bubi.nodanotes.data.model.SettingsDto
import com.bubi.nodanotes.data.repository.UpdateManager
import com.bubi.nodanotes.data.model.UpdateState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    onBackClick: () -> Unit,
    onNavigateToVaultSelector: () -> Unit,
    viewModel: SettingsViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsState()
    var selectedTab by remember { mutableStateOf(0) }
    val tabTitles = listOf("Appearance", "Editor", "Sync", "History", "Templates", "Vaults", "Maintenance", "Security", "Updates")

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
                            4 -> TemplatesTab(state.settings, viewModel)
                            5 -> VaultsTab(state.recentVaults, state.currentVault, onNavigateToVaultSelector)
                            6 -> MaintenanceTab()
                            7 -> SecurityTab(viewModel)
                            8 -> UpdatesTab(viewModel)
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
    var deviceName by remember { mutableStateOf(settings.sync.device_name) }
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
            value = deviceName,
            onValueChange = { deviceName = it },
            label = { Text("Device Name") },
            placeholder = { Text("e.g. My-Android-Phone") },
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
                    viewModel.updateSync(webdavUrl, username, password.ifEmpty { null }, interval.toLongOrNull() ?: 0L, deviceName)
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
    var localSnapshotInterval by remember(settings.history.snapshot_interval_mins) { mutableStateOf(settings.history.snapshot_interval_mins.toFloat()) }

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
                    viewModel.updateHistory(localRetentionDays.toInt(), localMaxSnapshots.toInt(), localEmptyTrashDays.toInt(), localSnapshotInterval.toInt())
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
                    viewModel.updateHistory(localRetentionDays.toInt(), localMaxSnapshots.toInt(), localEmptyTrashDays.toInt(), localSnapshotInterval.toInt())
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
                    viewModel.updateHistory(localRetentionDays.toInt(), localMaxSnapshots.toInt(), localEmptyTrashDays.toInt(), localSnapshotInterval.toInt())
                },
                valueRange = 1f..90f,
                steps = 89
            )
        }

        HorizontalDivider()

        Column {
            Text("Snapshot Interval (${localSnapshotInterval.toInt()} minutes)", style = MaterialTheme.typography.bodyLarge)
            Text("Time interval between auto-save snapshots when typing continuously", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Slider(
                value = localSnapshotInterval,
                onValueChange = { localSnapshotInterval = it },
                onValueChangeFinished = {
                    viewModel.updateHistory(localRetentionDays.toInt(), localMaxSnapshots.toInt(), localEmptyTrashDays.toInt(), localSnapshotInterval.toInt())
                },
                valueRange = 1f..60f,
                steps = 59
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TemplatesTab(settings: SettingsDto, viewModel: SettingsViewModel) {
    val templates by viewModel.templates.collectAsState()
    var expanded by remember { mutableStateOf(false) }

    val currentTemplate = templates.find { it.id == settings.editor.default_daily_template }
    val currentLabel = currentTemplate?.title ?: "No Template (Blank daily note)"

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp)
    ) {
        Text("Daily Note Template Selection", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
        Text("Choose a markdown template file from your '.templates' folder to populate new daily notes.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)

        Spacer(modifier = Modifier.height(8.dp))

        ExposedDropdownMenuBox(
            expanded = expanded,
            onExpandedChange = { expanded = !expanded }
        ) {
            OutlinedTextField(
                readOnly = true,
                value = currentLabel,
                onValueChange = {},
                label = { Text("Selected Template") },
                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
                colors = ExposedDropdownMenuDefaults.outlinedTextFieldColors(),
                modifier = Modifier
                    .fillMaxWidth()
                    .menuAnchor()
            )
            ExposedDropdownMenu(
                expanded = expanded,
                onDismissRequest = { expanded = false }
            ) {
                DropdownMenuItem(
                    text = { Text("No Template (Blank daily note)") },
                    onClick = {
                        viewModel.updateDefaultDailyTemplate(null)
                        expanded = false
                    }
                )
                templates.forEach { template ->
                    DropdownMenuItem(
                        text = {
                            Column {
                                Text(template.title, fontWeight = FontWeight.SemiBold)
                                Text(template.file_path, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                        },
                        onClick = {
                            viewModel.updateDefaultDailyTemplate(template.id)
                            expanded = false
                        }
                    )
                }
            }
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

@Composable
fun SecurityTab(viewModel: SettingsViewModel) {
    val scrollState = rememberScrollState()
    val isPasswordConfigured by viewModel.isPasswordConfigured.collectAsState()
    val vaultTimeout by viewModel.vaultTimeout.collectAsState()

    var passwordInput by remember { mutableStateOf("") }
    var confirmPasswordInput by remember { mutableStateOf("") }
    var oldPasswordInput by remember { mutableStateOf("") }
    var newPasswordInput by remember { mutableStateOf("") }
    var confirmNewPasswordInput by remember { mutableStateOf("") }
    
    var showPassword by remember { mutableStateOf(false) }
    var showConfirmPassword by remember { mutableStateOf(false) }
    var showOldPassword by remember { mutableStateOf(false) }
    var showNewPassword by remember { mutableStateOf(false) }
    var showConfirmNewPassword by remember { mutableStateOf(false) }

    var actionMessage by remember { mutableStateOf<String?>(null) }
    var isError by remember { mutableStateOf(false) }

    var timeoutExpanded by remember { mutableStateOf(false) }
    val timeoutOptions = listOf(
        "1m" to "1 Minute",
        "5m" to "5 Minutes",
        "15m" to "15 Minutes",
        "1h" to "1 Hour",
        "app_close" to "Until App Closes",
        "always" to "Every Time"
    )
    val currentTimeoutLabel = timeoutOptions.find { it.first == vaultTimeout }?.second ?: vaultTimeout

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(scrollState)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp)
    ) {
        Text("Vault Security & Encryption", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
        Text("Manage master password settings and lock configuration for zero-knowledge notes.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)

        actionMessage?.let { msg ->
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = if (isError) MaterialTheme.colorScheme.errorContainer else MaterialTheme.colorScheme.primaryContainer
                ),
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    modifier = Modifier.padding(12.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(
                        imageVector = if (isError) Icons.Default.Error else Icons.Default.CheckCircle,
                        contentDescription = null,
                        tint = if (isError) MaterialTheme.colorScheme.onErrorContainer else MaterialTheme.colorScheme.onPrimaryContainer
                    )
                    Spacer(modifier = Modifier.width(12.dp))
                    Text(
                        text = msg,
                        color = if (isError) MaterialTheme.colorScheme.onErrorContainer else MaterialTheme.colorScheme.onPrimaryContainer,
                        style = MaterialTheme.typography.bodyMedium
                    )
                }
            }
        }

        if (!isPasswordConfigured) {
            // Setup flow
            Card(
                shape = RoundedCornerShape(12.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f))
            ) {
                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text("Setup Master Password", fontWeight = FontWeight.Bold, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)
                    Text("Choose a master password to encrypt your sensitive notes. Make sure to keep it safe; it cannot be recovered if lost.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)

                    OutlinedTextField(
                        value = passwordInput,
                        onValueChange = { passwordInput = it },
                        label = { Text("Master Password") },
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
                        value = confirmPasswordInput,
                        onValueChange = { confirmPasswordInput = it },
                        label = { Text("Confirm Master Password") },
                        visualTransformation = if (showConfirmPassword) VisualTransformation.None else PasswordVisualTransformation(),
                        trailingIcon = {
                            IconButton(onClick = { showConfirmPassword = !showConfirmPassword }) {
                                Icon(imageVector = if (showConfirmPassword) Icons.Default.VisibilityOff else Icons.Default.Visibility, contentDescription = "Toggle password")
                            }
                        },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true
                    )

                    Button(
                        onClick = {
                            if (passwordInput.isEmpty()) {
                                isError = true
                                actionMessage = "Password cannot be empty."
                                return@Button
                            }
                            if (passwordInput != confirmPasswordInput) {
                                isError = true
                                actionMessage = "Passwords do not match."
                                return@Button
                            }
                            viewModel.setMasterPassword(
                                passwordInput,
                                onSuccess = {
                                    isError = false
                                    actionMessage = "Master password setup completed successfully."
                                    passwordInput = ""
                                    confirmPasswordInput = ""
                                },
                                onFailure = { err ->
                                    isError = true
                                    actionMessage = err
                                }
                            )
                        },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Configure Password")
                    }
                }
            }
        } else {
            // Change flow
            Card(
                shape = RoundedCornerShape(12.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f))
            ) {
                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text("Change Master Password", fontWeight = FontWeight.Bold, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)

                    OutlinedTextField(
                        value = oldPasswordInput,
                        onValueChange = { oldPasswordInput = it },
                        label = { Text("Current Master Password") },
                        visualTransformation = if (showOldPassword) VisualTransformation.None else PasswordVisualTransformation(),
                        trailingIcon = {
                            IconButton(onClick = { showOldPassword = !showOldPassword }) {
                                Icon(imageVector = if (showOldPassword) Icons.Default.VisibilityOff else Icons.Default.Visibility, contentDescription = "Toggle password")
                            }
                        },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true
                    )

                    OutlinedTextField(
                        value = newPasswordInput,
                        onValueChange = { newPasswordInput = it },
                        label = { Text("New Master Password") },
                        visualTransformation = if (showNewPassword) VisualTransformation.None else PasswordVisualTransformation(),
                        trailingIcon = {
                            IconButton(onClick = { showNewPassword = !showNewPassword }) {
                                Icon(imageVector = if (showNewPassword) Icons.Default.VisibilityOff else Icons.Default.Visibility, contentDescription = "Toggle password")
                            }
                        },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true
                    )

                    OutlinedTextField(
                        value = confirmNewPasswordInput,
                        onValueChange = { confirmNewPasswordInput = it },
                        label = { Text("Confirm New Master Password") },
                        visualTransformation = if (showConfirmNewPassword) VisualTransformation.None else PasswordVisualTransformation(),
                        trailingIcon = {
                            IconButton(onClick = { showConfirmNewPassword = !showConfirmNewPassword }) {
                                Icon(imageVector = if (showConfirmNewPassword) Icons.Default.VisibilityOff else Icons.Default.Visibility, contentDescription = "Toggle password")
                            }
                        },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true
                    )

                    Button(
                        onClick = {
                            if (oldPasswordInput.isEmpty() || newPasswordInput.isEmpty() || confirmNewPasswordInput.isEmpty()) {
                                isError = true
                                actionMessage = "Fields cannot be empty."
                                return@Button
                            }
                            if (newPasswordInput != confirmNewPasswordInput) {
                                isError = true
                                actionMessage = "New passwords do not match."
                                return@Button
                            }
                            viewModel.changeMasterPassword(
                                oldPasswordInput,
                                newPasswordInput,
                                onSuccess = {
                                    isError = false
                                    actionMessage = "Master password changed successfully."
                                    oldPasswordInput = ""
                                    newPasswordInput = ""
                                    confirmNewPasswordInput = ""
                                },
                                onFailure = { err ->
                                    isError = true
                                    actionMessage = err
                                }
                            )
                        },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Change Password")
                    }
                }
            }
        }

        HorizontalDivider()

        // Section 3: Timeout setting
        Column(
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Text("Session Timeout", style = MaterialTheme.typography.titleSmall, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
            Text("Select how long a vault session remains unlocked before requiring a password entry again.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            
            Box(modifier = Modifier.fillMaxWidth()) {
                OutlinedCard(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable { timeoutExpanded = true }
                ) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(16.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(currentTimeoutLabel, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.SemiBold)
                        Icon(Icons.Default.ArrowDropDown, contentDescription = null)
                    }
                }

                DropdownMenu(
                    expanded = timeoutExpanded,
                    onDismissRequest = { timeoutExpanded = false }
                ) {
                    timeoutOptions.forEach { (option, label) ->
                        DropdownMenuItem(
                            text = { Text(label) },
                            onClick = {
                                viewModel.updateVaultTimeout(option)
                                timeoutExpanded = false
                            }
                        )
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun UpdatesTab(viewModel: SettingsViewModel) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val scope = rememberCoroutineScope()
    val scrollState = rememberScrollState()

    val updateInterval by viewModel.updateInterval.collectAsState()
    val isChecking by viewModel.isCheckingUpdate.collectAsState()
    val globalUpdateState by UpdateManager.updateState.collectAsState()

    var checkResultMessage by remember { mutableStateOf<String?>(null) }
    var isDownloading by remember { mutableStateOf(false) }

    val intervalOptions = listOf("Every Entry", "Hourly", "Daily", "Weekly")
    var intervalExpanded by remember { mutableStateOf(false) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(scrollState)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text(
            text = "App Update Settings",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.primary
        )

        // Interval Card
        Card(
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f)),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text(
                    text = "Update Validation Interval",
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = "Configure how often the application checks for updates automatically.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Spacer(modifier = Modifier.height(16.dp))

                Box {
                    OutlinedCard(
                        onClick = { intervalExpanded = true },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(16.dp),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(updateInterval, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.SemiBold)
                            Icon(Icons.Default.ArrowDropDown, contentDescription = null)
                        }
                    }

                    DropdownMenu(
                        expanded = intervalExpanded,
                        onDismissRequest = { intervalExpanded = false }
                    ) {
                        intervalOptions.forEach { option ->
                            DropdownMenuItem(
                                text = { Text(option) },
                                onClick = {
                                    viewModel.updateUpdateInterval(option)
                                    intervalExpanded = false
                                }
                            )
                        }
                    }
                }
            }
        }

        // Manual Check Card
        Card(
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f)),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text(
                    text = "Check for Updates",
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = "Manually query the server proxy to verify if a newer application package is available.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Spacer(modifier = Modifier.height(16.dp))

                Button(
                    onClick = {
                        checkResultMessage = null
                        viewModel.triggerManualUpdateCheck { result ->
                            checkResultMessage = when (result) {
                                is UpdateState.NoUpdate -> "NodaNotes is up to date."
                                is UpdateState.FlexibleUpdate -> "Optional update available: Version ${result.tagName}"
                                is UpdateState.MandatoryUpdate -> "Mandatory update required: Version ${result.tagName}"
                            }
                        }
                    },
                    enabled = !isChecking && !isDownloading,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    if (isChecking) {
                        CircularProgressIndicator(
                            modifier = Modifier.size(18.dp),
                            color = MaterialTheme.colorScheme.onPrimary,
                            strokeWidth = 2.dp
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("Checking...")
                    } else {
                        Text("Check for Updates")
                    }
                }

                checkResultMessage?.let { msg ->
                    Spacer(modifier = Modifier.height(12.dp))
                    Text(
                        text = msg,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.primary,
                        fontWeight = FontWeight.Medium
                    )
                }

                // If flexible update is available, offer direct download in Settings grid
                if (globalUpdateState is UpdateState.FlexibleUpdate) {
                    val flexState = globalUpdateState as UpdateState.FlexibleUpdate
                    Spacer(modifier = Modifier.height(16.dp))
                    HorizontalDivider()
                    Spacer(modifier = Modifier.height(16.dp))

                    Text(
                        text = "New Optional Version: ${flexState.tagName}",
                        style = MaterialTheme.typography.bodyLarge,
                        fontWeight = FontWeight.Bold
                    )
                    Spacer(modifier = Modifier.height(8.dp))

                    Button(
                        onClick = {
                            isDownloading = true
                            scope.launch {
                                val file = UpdateManager.downloadApk(context, flexState.apkUrl)
                                isDownloading = false
                                if (file != null) {
                                    UpdateManager.installApk(context, file)
                                } else {
                                    checkResultMessage = "Failed to download update APK."
                                }
                            }
                        },
                        enabled = !isDownloading,
                        colors = ButtonDefaults.buttonColors(
                            containerColor = MaterialTheme.colorScheme.secondaryContainer,
                            contentColor = MaterialTheme.colorScheme.onSecondaryContainer
                        ),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        if (isDownloading) {
                            CircularProgressIndicator(modifier = Modifier.size(18.dp))
                            Spacer(modifier = Modifier.width(8.dp))
                            Text("Downloading...")
                        } else {
                            Text("Download and Install Now")
                        }
                    }
                }
            }
        }
    }
}
