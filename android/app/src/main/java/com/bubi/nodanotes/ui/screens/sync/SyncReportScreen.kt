package com.bubi.nodanotes.ui.screens.sync

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.bubi.nodanotes.data.model.SyncReportDto
import com.bubi.nodanotes.data.preferences.VaultPreferences
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SyncReportScreen(
    onBackClick: () -> Unit,
    onNavigateToConflicts: () -> Unit,
    vaultPreferences: VaultPreferences
) {
    val syncRepository = remember { com.bubi.nodanotes.data.repository.SyncRepository() }
    var isSyncing by remember { mutableStateOf(false) }
    var currentProgress by remember { mutableStateOf<com.bubi.nodanotes.data.repository.SyncProgressDto?>(null) }
    var localReportJson by remember { mutableStateOf(vaultPreferences.getLastSyncReport()) }

    LaunchedEffect(Unit) {
        isSyncing = syncRepository.getSyncStatus().map { it.is_syncing }.getOrDefault(false)
        syncRepository.syncProgressFlow.collect { progress ->
            isSyncing = true
            currentProgress = progress
        }
    }

    LaunchedEffect(isSyncing) {
        if (isSyncing) {
            while (isSyncing) {
                kotlinx.coroutines.delay(1000)
                val status = syncRepository.getSyncStatus().getOrNull()
                if (status != null && !status.is_syncing) {
                    isSyncing = false
                    localReportJson = vaultPreferences.getLastSyncReport()
                }
            }
        }
    }

    val currentReportJson = localReportJson
    val report = remember(currentReportJson) {
        if (currentReportJson != null) {
            try {
                Json.decodeFromString<SyncReportDto>(currentReportJson)
            } catch (e: Exception) {
                null
            }
        } else {
            null
        }
    }
    val syncTime = remember(currentReportJson) {
        if (currentReportJson != null) {
            try {
                val element = Json.parseToJsonElement(currentReportJson)
                val timeStr = element.jsonObject["sync_time"]?.jsonPrimitive?.content
                if (timeStr != null) {
                    val dt = java.time.ZonedDateTime.parse(timeStr)
                    val formatter = java.time.format.DateTimeFormatter.ofPattern("MMM dd, yyyy · HH:mm", java.util.Locale.getDefault())
                    dt.withZoneSameInstant(java.time.ZoneId.systemDefault()).format(formatter)
                } else {
                    null
                }
            } catch (e: Exception) {
                null
            }
        } else {
            null
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Sync Report", fontWeight = FontWeight.Bold) },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { innerPadding ->
        if (isSyncing) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(innerPadding)
                    .padding(24.dp),
                contentAlignment = Alignment.Center
            ) {
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.Center,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(64.dp),
                        strokeWidth = 6.dp,
                        color = MaterialTheme.colorScheme.primary
                    )
                    Spacer(modifier = Modifier.height(24.dp))
                    Text(
                        text = "Syncing in progress...",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.primary
                    )
                    Spacer(modifier = Modifier.height(16.dp))
                    val progress = currentProgress
                    if (progress != null) {
                        val percentage = if (progress.total_count > 0) {
                            progress.current_index.toFloat() / progress.total_count.toFloat()
                        } else {
                            0f
                        }
                        LinearProgressIndicator(
                            progress = { percentage },
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(8.dp),
                            color = MaterialTheme.colorScheme.secondary,
                            trackColor = MaterialTheme.colorScheme.secondaryContainer
                        )
                        Spacer(modifier = Modifier.height(12.dp))
                        Text(
                            text = "Processing ${progress.current_index} of ${progress.total_count}",
                            style = MaterialTheme.typography.bodyMedium,
                            fontWeight = FontWeight.SemiBold
                        )
                        Spacer(modifier = Modifier.height(4.dp))
                        Text(
                            text = "Action: ${progress.action.uppercase()}",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Spacer(modifier = Modifier.height(4.dp))
                        Text(
                            text = progress.file_path,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.8f),
                            maxLines = 2,
                            overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis
                        )
                    } else {
                        Text(
                            text = "Initializing connection...",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
            }
        } else if (report == null) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(innerPadding),
                contentAlignment = Alignment.Center
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Icon(
                        imageVector = Icons.Default.Info,
                        contentDescription = null,
                        modifier = Modifier.size(64.dp),
                        tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f)
                    )
                    Spacer(modifier = Modifier.height(16.dp))
                    Text("No sync reports found.", color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        } else {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(innerPadding)
                    .verticalScroll(rememberScrollState())
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                // Summary Card
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f))
                ) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text("Summary", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
                            if (syncTime != null) {
                                  Text(
                                      text = syncTime,
                                      style = MaterialTheme.typography.bodySmall,
                                      color = MaterialTheme.colorScheme.onSurfaceVariant
                                  )
                              }
                          }
                          Spacer(modifier = Modifier.height(12.dp))
                          Row(
                              modifier = Modifier.fillMaxWidth(),
                              horizontalArrangement = Arrangement.SpaceBetween
                          ) {
                              SummaryItem(label = "Uploads", count = report.uploads, icon = Icons.Default.Upload)
                              SummaryItem(label = "Downloads", count = report.downloads, icon = Icons.Default.Download)
                              SummaryItem(label = "Local Del", count = report.deletes_local, icon = Icons.Default.Delete)
                              SummaryItem(label = "Remote Del", count = report.deletes_remote, icon = Icons.Default.DeleteForever)
                              SummaryItem(label = "Conflicts", count = report.conflicts, icon = Icons.Default.Warning, isWarning = report.conflicts > 0)
                          }
                      }
                  }
  
                  if (report.conflicts > 0) {
                      Button(
                          onClick = onNavigateToConflicts,
                          modifier = Modifier.fillMaxWidth(),
                          colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error)
                      ) {
                          Icon(Icons.Default.Warning, contentDescription = null)
                          Spacer(modifier = Modifier.width(8.dp))
                          Text("Resolve Conflicts (${report.conflicts})")
                      }
                  }
  
                  // Detailed Files Lists
                  ReportListSection(title = "Uploaded Notes", files = report.uploaded_files, icon = Icons.Default.Upload)
                  ReportListSection(title = "Downloaded Notes", files = report.downloaded_files, icon = Icons.Default.Download)
                  ReportListSection(title = "Deleted Local", files = report.deleted_local_files, icon = Icons.Default.Delete)
                ReportListSection(title = "Deleted Remote", files = report.deleted_remote_files, icon = Icons.Default.DeleteForever)
                ReportListSection(title = "Conflicts", files = report.conflict_files, icon = Icons.Default.Warning, isWarning = true)
            }
        }
    }
}

@Composable
fun SummaryItem(label: String, count: Int, icon: ImageVector, isWarning: Boolean = false) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier.padding(8.dp)
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            tint = if (isWarning) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant
        )
        Spacer(modifier = Modifier.height(4.dp))
        Text(text = count.toString(), style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
        Text(text = label, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
fun ReportListSection(title: String, files: List<String>, icon: ImageVector, isWarning: Boolean = false) {
    if (files.isEmpty()) return

    Column(modifier = Modifier.fillMaxWidth()) {
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold,
            color = if (isWarning) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(vertical = 8.dp)
        )
        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
        ) {
            Column(modifier = Modifier.padding(12.dp)) {
                files.forEach { file ->
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Icon(
                            imageVector = icon,
                            contentDescription = null,
                            tint = if (isWarning) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(18.dp)
                        )
                        Spacer(modifier = Modifier.width(12.dp))
                        Text(text = file, style = MaterialTheme.typography.bodyMedium)
                    }
                }
            }
        }
    }
}
