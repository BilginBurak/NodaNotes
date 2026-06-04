package com.bubi.nodanotes.ui.screens.history

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.History
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.bubi.nodanotes.data.model.SnapshotDto
import com.bubi.nodanotes.ui.components.DiffViewer

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HistoryScreen(
    noteId: String,
    onBackClick: () -> Unit,
    viewModel: HistoryViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsState()
    val diffState by viewModel.diffState.collectAsState()

    var showDiffSheet by remember { mutableStateOf(false) }
    var selectedTimestamp by remember { mutableStateOf("") }
    var snapshotToDelete by remember { mutableStateOf<SnapshotDto?>(null) }
    var snapshotToRestore by remember { mutableStateOf<SnapshotDto?>(null) }

    LaunchedEffect(noteId) {
        viewModel.loadSnapshots(noteId)
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Version History", fontWeight = FontWeight.Bold) },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { paddingValues ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            when (val state = uiState) {
                is HistoryUiState.Loading -> {
                    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        CircularProgressIndicator()
                    }
                }
                is HistoryUiState.Error -> {
                    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        Column(
                            horizontalAlignment = Alignment.CenterHorizontally,
                            modifier = Modifier.padding(24.dp)
                        ) {
                            Text(state.message, color = MaterialTheme.colorScheme.error)
                            Spacer(modifier = Modifier.height(16.dp))
                            Button(onClick = { viewModel.loadSnapshots(noteId) }) {
                                Text("Retry")
                            }
                        }
                    }
                }
                is HistoryUiState.Success -> {
                    if (state.snapshots.isEmpty()) {
                        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                                Icon(
                                    imageVector = Icons.Default.History,
                                    contentDescription = null,
                                    modifier = Modifier.size(64.dp),
                                    tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f)
                                )
                                Spacer(modifier = Modifier.height(16.dp))
                                Text(
                                    text = "No history snapshots yet",
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                        }
                    } else {
                        LazyColumn(
                            modifier = Modifier.fillMaxSize(),
                            contentPadding = PaddingValues(16.dp),
                            verticalArrangement = Arrangement.spacedBy(12.dp)
                        ) {
                            items(state.snapshots) { snapshot ->
                                SnapshotCard(
                                    snapshot = snapshot,
                                    onViewDiff = {
                                        selectedTimestamp = snapshot.timestamp
                                        viewModel.loadDiff(snapshot.timestamp)
                                        showDiffSheet = true
                                    },
                                    onDelete = {
                                        snapshotToDelete = snapshot
                                    }
                                )
                            }
                        }
                    }
                }
            }
        }
    }

    // Diff Comparison Bottom Sheet
    if (showDiffSheet) {
        ModalBottomSheet(
            onDismissRequest = {
                showDiffSheet = false
                viewModel.clearDiff()
            },
            sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .fillMaxHeight(0.85f)
                    .navigationBarsPadding()
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 24.dp, vertical = 16.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = "Version Changes",
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.Bold
                    )
                    Button(
                        onClick = {
                            val targetSnapshot = (uiState as? HistoryUiState.Success)?.snapshots?.find { it.timestamp == selectedTimestamp }
                            snapshotToRestore = targetSnapshot
                        },
                        colors = ButtonDefaults.buttonColors(
                            containerColor = MaterialTheme.colorScheme.primary
                        )
                    ) {
                        Text("Restore Version")
                    }
                }

                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)

                Box(
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxWidth()
                ) {
                    when (val dState = diffState) {
                        is DiffUiState.Loading -> {
                            Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                                CircularProgressIndicator()
                            }
                        }
                        is DiffUiState.Error -> {
                            Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                                Text(dState.message, color = MaterialTheme.colorScheme.error)
                            }
                        }
                        is DiffUiState.Success -> {
                            DiffViewer(
                                chunks = dState.chunks,
                                modifier = Modifier.fillMaxSize()
                            )
                        }
                        else -> {}
                    }
                }
            }
        }
    }

    // Delete Confirmation Dialog
    if (snapshotToDelete != null) {
        AlertDialog(
            onDismissRequest = { snapshotToDelete = null },
            title = { Text("Delete Snapshot?") },
            text = { Text("This will permanently remove this version snapshot. This action cannot be undone.") },
            confirmButton = {
                TextButton(
                    onClick = {
                        snapshotToDelete?.let { viewModel.deleteSnapshot(it.timestamp) }
                        snapshotToDelete = null
                    }
                ) {
                    Text("Delete", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { snapshotToDelete = null }) {
                    Text("Cancel")
                }
            }
        )
    }

    // Restore Confirmation Dialog
    if (snapshotToRestore != null) {
        AlertDialog(
            onDismissRequest = { snapshotToRestore = null },
            title = { Text("Restore Version?") },
            text = { Text("Are you sure you want to restore the note to this version? Your current version will be saved as a new snapshot first.") },
            confirmButton = {
                TextButton(
                    onClick = {
                        snapshotToRestore?.let { snapshot ->
                            viewModel.restoreSnapshot(snapshot.timestamp) {
                                showDiffSheet = false
                                snapshotToRestore = null
                                onBackClick()
                            }
                        }
                    }
                ) {
                    Text("Restore", fontWeight = FontWeight.Bold)
                }
            },
            dismissButton = {
                TextButton(onClick = { snapshotToRestore = null }) {
                    Text("Cancel")
                }
            }
        )
    }
}

@Composable
private fun SnapshotCard(
    snapshot: SnapshotDto,
    onViewDiff: () -> Unit,
    onDelete: () -> Unit,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f)
        ),
        border = CardDefaults.outlinedCardBorder()
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = formatTimestamp(snapshot.timestamp) + " [${snapshot.reason}]",
                    fontWeight = FontWeight.Bold,
                    fontSize = 15.sp,
                    color = MaterialTheme.colorScheme.onSurface
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = formatBytes(snapshot.size_bytes),
                    fontSize = 12.sp,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                Button(
                    onClick = onViewDiff,
                    colors = ButtonDefaults.filledTonalButtonColors()
                ) {
                    Text("View Diff")
                }
                Spacer(modifier = Modifier.width(8.dp))
                IconButton(onClick = onDelete) {
                    Icon(
                        imageVector = Icons.Default.Delete,
                        contentDescription = "Delete version snapshot",
                        tint = MaterialTheme.colorScheme.error
                    )
                }
            }
        }
    }
}

private fun formatTimestamp(timestamp: String): String {
    return try {
        val instant = java.time.Instant.parse(timestamp)
        val zoneId = java.time.ZoneId.systemDefault()
        val localDateTime = java.time.ZonedDateTime.ofInstant(instant, zoneId)
        val formatter = java.time.format.DateTimeFormatter.ofPattern("d MMMM yyyy - HH:mm", java.util.Locale("tr", "TR"))
        localDateTime.format(formatter)
    } catch (e: Exception) {
        try {
            if (timestamp.length >= 15 && timestamp.contains("_")) {
                val parts = timestamp.split("_")
                val datePart = parts[0]
                val timePart = parts[1]

                val year = datePart.substring(0, 4)
                val monthNum = datePart.substring(4, 6).toInt()
                val day = datePart.substring(6, 8).toInt()

                val hour = timePart.substring(0, 2)
                val minute = timePart.substring(2, 4)

                val months = listOf(
                    "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran",
                    "Temmuz", "Ağustos", "Eylül", "Ekim", "Kasım", "Aralık"
                )
                val monthStr = months.getOrNull(monthNum - 1) ?: "$monthNum"

                "$day $monthStr $year - $hour:$minute"
            } else {
                timestamp
            }
        } catch (ex: Exception) {
            timestamp
        }
    }
}

private fun formatBytes(bytes: Long): String {
    if (bytes < 1024) return "$bytes B"
    val exp = (Math.log(bytes.toDouble()) / Math.log(1024.0)).toInt()
    val pre = "KMGTPE"[exp - 1] + "i"
    return String.format("%.1f %sB", bytes / Math.pow(1024.0, exp.toDouble()), pre)
}
