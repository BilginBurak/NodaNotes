package com.bubi.nodanotes.ui.screens.maintenance

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.bubi.nodanotes.data.model.DuplicateNoteGroupDto
import com.bubi.nodanotes.data.model.OrphanedFileDto

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MaintenanceScreen(
    onBackClick: () -> Unit,
    viewModel: MaintenanceViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsState()
    val duplicates by viewModel.duplicates.collectAsState()
    val remnants by viewModel.orphanedRemnants.collectAsState()
    val attachments by viewModel.orphanedAttachments.collectAsState()

    var activeTab by remember { mutableStateOf(0) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Vault Maintenance", fontWeight = FontWeight.Bold, fontSize = 20.sp) },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { paddingValues ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            TabRow(selectedTabIndex = activeTab) {
                Tab(selected = activeTab == 0, onClick = { activeTab = 0 }, text = { Text("Optimizations") })
                Tab(selected = activeTab == 1, onClick = { activeTab = 1 }, text = { Text("Duplicates") })
                Tab(selected = activeTab == 2, onClick = { activeTab = 2 }, text = { Text("Remnants") })
            }

            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .weight(1f)
            ) {
                when (val state = uiState) {
                    is MaintenanceUiState.Loading -> {
                        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                            CircularProgressIndicator()
                        }
                    }
                    is MaintenanceUiState.Success -> {
                        Box(modifier = Modifier.fillMaxSize().padding(24.dp), contentAlignment = Alignment.Center) {
                            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                                Icon(Icons.Default.CheckCircle, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(56.dp))
                                Spacer(modifier = Modifier.height(16.dp))
                                Text(state.message, textAlign = TextAlign.Center)
                            }
                        }
                    }
                    is MaintenanceUiState.Error -> {
                        Box(modifier = Modifier.fillMaxSize().padding(24.dp), contentAlignment = Alignment.Center) {
                            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                                Icon(Icons.Default.Error, contentDescription = null, tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(56.dp))
                                Spacer(modifier = Modifier.height(16.dp))
                                Text(state.message, textAlign = TextAlign.Center, color = MaterialTheme.colorScheme.error)
                            }
                        }
                    }
                    is MaintenanceUiState.Idle -> {
                        when (activeTab) {
                            0 -> OptimizationsTab(viewModel = viewModel)
                            1 -> DuplicatesTab(duplicates = duplicates, onDelete = { viewModel.deleteDuplicateFile(it) }, onScan = { viewModel.scanDuplicates() })
                            2 -> RemnantsTab(
                                remnants = remnants,
                                attachments = attachments,
                                onDeleteRemnant = { viewModel.deleteOrphanedFile(it) },
                                onDeleteAllRemnants = { viewModel.deleteAllOrphanedRemnants() },
                                onScan = {
                                    viewModel.scanOrphanedRemnants()
                                    viewModel.scanOrphanedAttachments()
                                }
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun OptimizationsTab(viewModel: MaintenanceViewModel) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp)
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Card(
            shape = RoundedCornerShape(12.dp),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f))
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text("Rebuild Cache", fontWeight = FontWeight.Bold, style = MaterialTheme.typography.titleMedium)
                Spacer(modifier = Modifier.height(4.dp))
                Text("Scans all markdown files in the active vault, regenerates SQLite indices, and updates metadata. Use this if your note list feels out of sync.", style = MaterialTheme.typography.bodyMedium)
                Spacer(modifier = Modifier.height(12.dp))
                Button(onClick = { viewModel.rebuildCache() }, modifier = Modifier.fillMaxWidth()) {
                    Text("Rebuild Index Cache")
                }
            }
        }

        Card(
            shape = RoundedCornerShape(12.dp),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f))
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text("Optimize FTS5 Search", fontWeight = FontWeight.Bold, style = MaterialTheme.typography.titleMedium)
                Spacer(modifier = Modifier.height(4.dp))
                Text("Runs SQLite FTS5 database optimization routine to pack search indices for quicker, instantaneous response time.", style = MaterialTheme.typography.bodyMedium)
                Spacer(modifier = Modifier.height(12.dp))
                Button(onClick = { viewModel.optimizeFts() }, modifier = Modifier.fillMaxWidth()) {
                    Text("Optimize Search Index")
                }
            }
        }

        Card(
            shape = RoundedCornerShape(12.dp),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f))
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text("Reset Remote Sync Tracking", fontWeight = FontWeight.Bold, style = MaterialTheme.typography.titleMedium)
                Spacer(modifier = Modifier.height(4.dp))
                Text("Resets sync logs and state queue. Use this if your sync process is hanging or needs a complete clean reset.", style = MaterialTheme.typography.bodyMedium)
                Spacer(modifier = Modifier.height(12.dp))
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
    }
}

@Composable
fun DuplicatesTab(
    duplicates: List<DuplicateNoteGroupDto>,
    onDelete: (String) -> Unit,
    onScan: () -> Unit
) {
    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        Button(onClick = onScan, modifier = Modifier.fillMaxWidth()) {
            Text("Scan for Duplicate Notes")
        }
        Spacer(modifier = Modifier.height(16.dp))
        if (duplicates.isEmpty()) {
            Box(modifier = Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                Text("No duplicates found. Run scan to check.", color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f))
            }
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                items(duplicates) { group ->
                    Card(
                        modifier = Modifier.fillMaxWidth(),
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.2f))
                    ) {
                        Column(modifier = Modifier.padding(16.dp)) {
                            Text("Note ID: ${group.note_id}", fontWeight = FontWeight.Bold)
                            Text("Title: ${group.title}", style = MaterialTheme.typography.bodyMedium)
                            Spacer(modifier = Modifier.height(8.dp))
                            group.files.forEach { file ->
                                Row(
                                    modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Column(modifier = Modifier.weight(1f)) {
                                        Text(file.path, style = MaterialTheme.typography.bodySmall, fontWeight = FontWeight.SemiBold)
                                        Text("Size: ${file.size_bytes} bytes | Mod: ${file.modified_at}", style = MaterialTheme.typography.bodySmall)
                                    }
                                    IconButton(onClick = { onDelete(file.path) }) {
                                        Icon(Icons.Default.Delete, contentDescription = "Delete File", tint = MaterialTheme.colorScheme.error)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun RemnantsTab(
    remnants: List<OrphanedFileDto>,
    attachments: List<com.bubi.nodanotes.data.repository.DiagnosticsRepository.OrphanedAttachmentDto>,
    onDeleteRemnant: (String) -> Unit,
    onDeleteAllRemnants: () -> Unit,
    onScan: () -> Unit
) {
    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
            Button(onClick = onScan, modifier = Modifier.weight(1f)) {
                Text("Scan Remnants")
            }
            if (remnants.isNotEmpty()) {
                Button(
                    onClick = onDeleteAllRemnants,
                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error),
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Delete All Remnants")
                }
            }
        }
        Spacer(modifier = Modifier.height(16.dp))
        if (remnants.isEmpty() && attachments.isEmpty()) {
            Box(modifier = Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                Text("No remnants or orphaned files found.", color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f))
            }
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                if (remnants.isNotEmpty()) {
                    item {
                        Text("Orphaned History & Conflicts", fontWeight = FontWeight.Bold, style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(vertical = 4.dp))
                    }
                    items(remnants) { file ->
                        Card(modifier = Modifier.fillMaxWidth()) {
                            Row(modifier = Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(file.title, fontWeight = FontWeight.Bold)
                                    Text("Path: ${file.relative_path}", style = MaterialTheme.typography.bodySmall)
                                    Text("Type: ${file.file_type} | Size: ${file.size_bytes} bytes", style = MaterialTheme.typography.bodySmall)
                                }
                                IconButton(onClick = { onDeleteRemnant(file.relative_path) }) {
                                    Icon(Icons.Default.Delete, contentDescription = "Delete", tint = MaterialTheme.colorScheme.error)
                                }
                            }
                        }
                    }
                }

                if (attachments.isNotEmpty()) {
                    item {
                        Spacer(modifier = Modifier.height(12.dp))
                        Text("Orphaned Attachments", fontWeight = FontWeight.Bold, style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(vertical = 4.dp))
                    }
                    items(attachments) { attachment ->
                        Card(modifier = Modifier.fillMaxWidth()) {
                            Row(modifier = Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(attachment.name, fontWeight = FontWeight.Bold)
                                    Text("Size: ${attachment.size_bytes} bytes", style = MaterialTheme.typography.bodySmall)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
