package com.bubi.nodanotes.ui.screens.notelist

import androidx.compose.animation.*
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.bubi.nodanotes.data.model.NoteListItemDto
import com.bubi.nodanotes.ui.components.NoteCard
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
@Composable
fun NoteListScreen(
    onMenuClick: () -> Unit,
    onSearchClick: () -> Unit,
    onNavigateToEditor: (String) -> Unit,
    viewModel: NoteListViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsState()
    val currentFolder by FolderContext.currentFolderState.collectAsState()
    val vaultName by viewModel.vaultName.collectAsState()

    val scope = rememberCoroutineScope()
    val snackbarHostState = remember { SnackbarHostState() }

    var selectedSortOrder by remember { mutableStateOf(SortOrder.UPDATED) }
    var showSortMenu by remember { mutableStateOf(false) }

    LaunchedEffect(currentFolder) {
        viewModel.loadNotes(currentFolder)
    }

    // Lifecycle observer for ON_RESUME
    val lifecycleOwner = androidx.lifecycle.compose.LocalLifecycleOwner.current
    DisposableEffect(lifecycleOwner) {
        val observer = androidx.lifecycle.LifecycleEventObserver { _, event ->
            if (event == androidx.lifecycle.Lifecycle.Event.ON_RESUME) {
                viewModel.refreshOnResume()
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
        }
    }

    // 30-second periodic polling while screen is active
    LaunchedEffect(Unit) {
        while (true) {
            kotlinx.coroutines.delay(30000)
            viewModel.refreshOnResume()
        }
    }

    val isRefreshing by viewModel.isRefreshing.collectAsState()
    val syncStatus by viewModel.syncStatus.collectAsState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(
                            text = currentFolder ?: "All Notes",
                            fontWeight = FontWeight.Bold,
                            fontSize = 18.sp
                        )
                        Text(
                            text = vaultName,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f)
                        )
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onMenuClick) {
                        Icon(Icons.Default.Menu, contentDescription = "Menu")
                    }
                },
                actions = {
                    IconButton(onClick = onSearchClick) {
                        Icon(Icons.Default.Search, contentDescription = "Search")
                    }
                    IconButton(onClick = { showSortMenu = true }) {
                        Icon(Icons.Default.Sort, contentDescription = "Sort Options")
                    }
                    DropdownMenu(
                        expanded = showSortMenu,
                        onDismissRequest = { showSortMenu = false }
                    ) {
                        Text(
                            text = "  Sort By",
                            fontSize = 12.sp,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.padding(vertical = 4.dp)
                        )
                        DropdownMenuItem(
                            text = { Text("Last Updated") },
                            onClick = {
                                selectedSortOrder = SortOrder.UPDATED
                                showSortMenu = false
                            },
                            leadingIcon = {
                                if (selectedSortOrder == SortOrder.UPDATED) {
                                    Icon(Icons.Default.Check, contentDescription = null)
                                }
                            }
                        )
                        DropdownMenuItem(
                            text = { Text("Note Title") },
                            onClick = {
                                selectedSortOrder = SortOrder.TITLE
                                showSortMenu = false
                            },
                            leadingIcon = {
                                if (selectedSortOrder == SortOrder.TITLE) {
                                    Icon(Icons.Default.Check, contentDescription = null)
                                }
                            }
                        )
                    }
                }
            )
        },
        floatingActionButton = {
            ExtendedFloatingActionButton(
                onClick = {
                    viewModel.createNote(currentFolder) { noteId ->
                        onNavigateToEditor(noteId)
                    }
                },
                shape = RoundedCornerShape(16.dp),
                containerColor = MaterialTheme.colorScheme.primary,
                contentColor = MaterialTheme.colorScheme.onPrimary
            ) {
                Icon(Icons.Default.Add, contentDescription = "New Note")
                Spacer(modifier = Modifier.width(8.dp))
                Text("New Note")
            }
        },
        snackbarHost = { SnackbarHost(snackbarHostState) }
    ) { paddingValues ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            Surface(
                color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 6.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(
                        imageVector = Icons.Default.Sync,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.size(16.dp)
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = syncStatus,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            PullToRefreshBox(
                isRefreshing = isRefreshing,
                onRefresh = { viewModel.triggerSync() },
                modifier = Modifier.weight(1f)
            ) {
                when (val state = uiState) {
                    is NoteListUiState.Loading -> {
                        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                            CircularProgressIndicator()
                        }
                    }
                    is NoteListUiState.Error -> {
                        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                            Column(
                                horizontalAlignment = Alignment.CenterHorizontally,
                                modifier = Modifier.padding(24.dp)
                            ) {
                                Icon(Icons.Default.Error, contentDescription = null, tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(48.dp))
                                Spacer(modifier = Modifier.height(16.dp))
                                Text(state.message, textAlign = TextAlign.Center, color = MaterialTheme.colorScheme.error)
                                Spacer(modifier = Modifier.height(16.dp))
                                Button(onClick = { viewModel.loadNotes(currentFolder) }) {
                                    Text("Retry")
                                }
                            }
                        }
                    }
                    is NoteListUiState.Success -> {
                        val notes = state.notes
                        if (notes.isEmpty()) {
                            EmptyStateView(
                                folderName = currentFolder ?: "All Notes",
                                onCreateNote = {
                                    viewModel.createNote(currentFolder) { noteId ->
                                        onNavigateToEditor(noteId)
                                    }
                                }
                            )
                        } else {
                            val sortedNotes = when (selectedSortOrder) {
                                SortOrder.UPDATED -> notes.sortedByDescending { it.updated_at }
                                SortOrder.TITLE -> notes.sortedBy { it.title.lowercase() }
                            }

                            val pinnedNotes = sortedNotes.filter { it.pinned }
                            val remainingNotes = sortedNotes.filter { !it.pinned }

                            LazyColumn(
                                contentPadding = PaddingValues(16.dp),
                                modifier = Modifier.fillMaxSize()
                            ) {
                                if (pinnedNotes.isNotEmpty()) {
                                    item {
                                        Text(
                                            text = "PINNED",
                                            style = MaterialTheme.typography.labelMedium,
                                            fontWeight = FontWeight.Bold,
                                            color = MaterialTheme.colorScheme.primary,
                                            modifier = Modifier.padding(start = 8.dp, bottom = 8.dp)
                                        )
                                    }
                                    items(pinnedNotes, key = { it.id }) { note ->
                                        NoteCard(
                                            note = note,
                                            onNoteClick = onNavigateToEditor,
                                            onPinToggle = { viewModel.togglePinNote(it) },
                                            onDelete = { itemToDelete ->
                                                viewModel.deleteNoteWithUndo(itemToDelete) { undoCallback ->
                                                    scope.launch {
                                                        val snackbarResult = snackbarHostState.showSnackbar(
                                                            message = "Note deleted",
                                                            actionLabel = "Undo",
                                                            duration = SnackbarDuration.Short
                                                        )
                                                        if (snackbarResult == SnackbarResult.ActionPerformed) {
                                                            undoCallback()
                                                        }
                                                    }
                                                }
                                            }
                                        )
                                    }
                                    item { Spacer(modifier = Modifier.height(16.dp)) }
                                }

                                if (remainingNotes.isNotEmpty()) {
                                    if (pinnedNotes.isNotEmpty()) {
                                        item {
                                            Text(
                                                text = "NOTES",
                                                style = MaterialTheme.typography.labelMedium,
                                                fontWeight = FontWeight.Bold,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
                                                modifier = Modifier.padding(start = 8.dp, bottom = 8.dp)
                                            )
                                        }
                                    }
                                    items(remainingNotes, key = { it.id }) { note ->
                                        NoteCard(
                                            note = note,
                                            onNoteClick = onNavigateToEditor,
                                            onPinToggle = { viewModel.togglePinNote(it) },
                                            onDelete = { itemToDelete ->
                                                viewModel.deleteNoteWithUndo(itemToDelete) { undoCallback ->
                                                    scope.launch {
                                                        val snackbarResult = snackbarHostState.showSnackbar(
                                                            message = "Note deleted",
                                                            actionLabel = "Undo",
                                                            duration = SnackbarDuration.Short
                                                        )
                                                        if (snackbarResult == SnackbarResult.ActionPerformed) {
                                                            undoCallback()
                                                        }
                                                    }
                                                }
                                            }
                                        )
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
fun EmptyStateView(folderName: String, onCreateNote: () -> Unit) {
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
                imageVector = Icons.Default.Description,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.3f),
                modifier = Modifier.size(80.dp)
            )
            Spacer(modifier = Modifier.height(24.dp))
            Text(
                text = "No notes found in '$folderName'",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = "Tap '+ New Note' to create your first markdown document.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f),
                textAlign = TextAlign.Center
            )
            Spacer(modifier = Modifier.height(24.dp))
            Button(onClick = onCreateNote) {
                Text("Create Note")
            }
        }
    }
}

enum class SortOrder {
    UPDATED, TITLE
}
