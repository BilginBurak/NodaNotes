package com.bubi.nodanotes.ui.screens.notelist

import androidx.compose.animation.*
import androidx.activity.compose.BackHandler
import androidx.compose.material.icons.automirrored.filled.ExitToApp
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.pulltorefresh.rememberPullToRefreshState
import androidx.compose.ui.graphics.graphicsLayer
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
import java.time.LocalDate
import java.time.YearMonth
import java.time.DayOfWeek
import java.time.format.TextStyle
import java.util.Locale
import androidx.compose.ui.window.Dialog
import androidx.compose.foundation.clickable
import androidx.compose.foundation.background
import androidx.compose.ui.draw.clip

@OptIn(ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
@Composable
fun NoteListScreen(
    onMenuClick: () -> Unit,
    onSearchClick: () -> Unit,
    onNavigateToEditor: (String) -> Unit,
    onNavigateToSyncReport: () -> Unit,
    viewModel: NoteListViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsState()
    val currentFolder by FolderContext.currentFolderState.collectAsState()
    val selectedTag by FolderContext.selectedTagState.collectAsState()
    val vaultName by viewModel.vaultName.collectAsState()

    val lazyListState = rememberLazyListState()
    var isFabVisible by remember { mutableStateOf(true) }
    var previousIndex by remember { mutableStateOf(0) }
    var previousScrollOffset by remember { mutableStateOf(0) }

    LaunchedEffect(lazyListState) {
        snapshotFlow { Pair(lazyListState.firstVisibleItemIndex, lazyListState.firstVisibleItemScrollOffset) }
            .collect { (index, offset) ->
                if (index > previousIndex) {
                    isFabVisible = false
                } else if (index < previousIndex) {
                    isFabVisible = true
                } else {
                    if (offset > previousScrollOffset + 10) {
                        isFabVisible = false
                    } else if (offset < previousScrollOffset - 10) {
                        isFabVisible = true
                    }
                }
                previousIndex = index
                previousScrollOffset = offset
            }
    }

    val scope = rememberCoroutineScope()
    val snackbarHostState = remember { SnackbarHostState() }

    var selectedSortOrder by remember { mutableStateOf(SortOrder.UPDATED) }
    var showSortMenu by remember { mutableStateOf(false) }

    val isRefreshing by viewModel.isRefreshing.collectAsState()
    val syncStatus by viewModel.syncStatus.collectAsState()

    // Selection mode state variables
    var isSelectionMode by remember { mutableStateOf(false) }
    val selectedNotes = remember { mutableStateListOf<NoteListItemDto>() }

    var showCalendarDialog by remember { mutableStateOf(false) }
    var dailyNoteDates by remember { mutableStateOf(setOf<String>()) }

    var showExitDialog by remember { mutableStateOf(false) }

    BackHandler(enabled = true) {
        showExitDialog = true
    }

    if (showExitDialog) {
        val context = androidx.compose.ui.platform.LocalContext.current
        AlertDialog(
            onDismissRequest = { showExitDialog = false },
            icon = {
                Icon(
                    imageVector = Icons.AutoMirrored.Filled.ExitToApp,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary
                )
            },
            title = {
                Text(
                    text = "Exit App",
                    fontWeight = FontWeight.Bold,
                    fontSize = 18.sp
                )
            },
            text = {
                Text(
                    text = "Are you sure you want to exit NodaNotes?",
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            },
            confirmButton = {
                Button(
                    onClick = {
                        showExitDialog = false
                        (context as? android.app.Activity)?.finish()
                    },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.primary
                    )
                ) {
                    Text("Exit")
                }
            },
            dismissButton = {
                TextButton(onClick = { showExitDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }

    if (showCalendarDialog) {
        Dialog(
            onDismissRequest = { showCalendarDialog = false }
        ) {
            var currentMonth by remember { mutableStateOf(YearMonth.now()) }
            val firstOfMonth = currentMonth.atDay(1)
            val firstDayOfWeek = firstOfMonth.dayOfWeek
            val emptyCellsBefore = firstDayOfWeek.value - 1
            val daysInMonth = currentMonth.lengthOfMonth()
            
            val cells = remember(currentMonth) {
                val list = mutableListOf<LocalDate?>()
                repeat(emptyCellsBefore) { list.add(null) }
                for (day in 1..daysInMonth) {
                    list.add(currentMonth.atDay(day))
                }
                while (list.size % 7 != 0) {
                    list.add(null)
                }
                list.chunked(7)
            }

            Surface(
                modifier = Modifier
                    .fillMaxWidth()
                    .wrapContentHeight()
                    .padding(horizontal = 8.dp),
                shape = RoundedCornerShape(24.dp),
                color = MaterialTheme.colorScheme.surface,
                tonalElevation = 6.dp
            ) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(20.dp),
                    horizontalAlignment = Alignment.CenterHorizontally
                ) {
                    // Header: Month Year + navigation arrows
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        IconButton(onClick = { currentMonth = currentMonth.minusMonths(1) }) {
                            Icon(Icons.Default.ChevronLeft, contentDescription = "Previous Month")
                        }
                        
                        Text(
                            text = "${currentMonth.month.getDisplayName(TextStyle.FULL, Locale.getDefault())} ${currentMonth.year}",
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.onSurface
                        )
                        
                        IconButton(onClick = { currentMonth = currentMonth.plusMonths(1) }) {
                            Icon(Icons.Default.ChevronRight, contentDescription = "Next Month")
                        }
                    }
                    
                    Spacer(modifier = Modifier.height(16.dp))
                    
                    // Weekday headers
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceEvenly
                    ) {
                        val days = listOf("M", "T", "W", "T", "F", "S", "S")
                        days.forEach { day ->
                            Text(
                                text = day,
                                style = MaterialTheme.typography.bodySmall,
                                fontWeight = FontWeight.Bold,
                                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
                                modifier = Modifier.weight(1f),
                                textAlign = TextAlign.Center
                            )
                        }
                    }
                    
                    Spacer(modifier = Modifier.height(8.dp))
                    
                    // Days grid
                    cells.forEach { row ->
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceEvenly
                        ) {
                            row.forEach { date ->
                                if (date == null) {
                                    Spacer(modifier = Modifier.weight(1f).aspectRatio(1f))
                                } else {
                                    val isToday = date == LocalDate.now()
                                    val hasDailyNote = dailyNoteDates.contains(date.toString())
                                    
                                    Box(
                                        modifier = Modifier
                                            .weight(1f)
                                            .aspectRatio(1f)
                                            .padding(2.dp)
                                            .clip(RoundedCornerShape(12.dp))
                                            .background(
                                                if (isToday) MaterialTheme.colorScheme.primaryContainer
                                                else androidx.compose.ui.graphics.Color.Transparent
                                            )
                                            .clickable {
                                                scope.launch {
                                                    val repo = com.bubi.nodanotes.data.repository.NoteRepository()
                                                    repo.triggerDailyNote(date.toString()).fold(
                                                        onSuccess = { noteDto ->
                                                            showCalendarDialog = false
                                                            onNavigateToEditor(noteDto.id)
                                                        },
                                                        onFailure = {}
                                                    )
                                                }
                                            },
                                        contentAlignment = Alignment.Center
                                    ) {
                                        Column(
                                            horizontalAlignment = Alignment.CenterHorizontally,
                                            verticalArrangement = Arrangement.Center,
                                            modifier = Modifier.fillMaxSize()
                                        ) {
                                            Text(
                                                text = date.dayOfMonth.toString(),
                                                style = MaterialTheme.typography.bodyMedium,
                                                fontWeight = if (isToday) FontWeight.Bold else FontWeight.Normal,
                                                color = if (isToday) MaterialTheme.colorScheme.onPrimaryContainer
                                                        else MaterialTheme.colorScheme.onSurface
                                            )
                                            if (hasDailyNote) {
                                                Spacer(modifier = Modifier.height(2.dp))
                                                Box(
                                                    modifier = Modifier
                                                        .size(5.dp)
                                                        .clip(RoundedCornerShape(50))
                                                        .background(MaterialTheme.colorScheme.primary)
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    Spacer(modifier = Modifier.height(20.dp))
                    
                    // Bottom actions: Today shortcut + close
                    Button(
                        onClick = {
                            scope.launch {
                                val repo = com.bubi.nodanotes.data.repository.NoteRepository()
                                repo.triggerDailyNote(null).fold(
                                    onSuccess = { noteDto ->
                                        showCalendarDialog = false
                                        onNavigateToEditor(noteDto.id)
                                    },
                                    onFailure = {}
                                )
                            }
                        },
                        modifier = Modifier.fillMaxWidth(),
                        shape = RoundedCornerShape(12.dp),
                        colors = ButtonDefaults.buttonColors(
                            containerColor = MaterialTheme.colorScheme.primary
                        )
                    ) {
                        Icon(Icons.Default.Today, contentDescription = null, modifier = Modifier.size(18.dp))
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("Trigger Today's Daily Note", fontWeight = FontWeight.Bold)
                    }
                    
                    Spacer(modifier = Modifier.height(8.dp))
                    
                    TextButton(
                        onClick = { showCalendarDialog = false },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Close", color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
        }
    }

    // Observe syncStatus but do not show toast anymore as it is removed by the user requirement.
    // LaunchedEffect(syncStatus) logic removed completely.

    LaunchedEffect(currentFolder, selectedTag) {
        viewModel.loadNotes(currentFolder)
        isSelectionMode = false
        selectedNotes.clear()
    }

    LaunchedEffect(showCalendarDialog) {
        if (showCalendarDialog) {
            val repo = com.bubi.nodanotes.data.repository.NoteRepository()
            repo.getAllNotes().fold(
                onSuccess = { notes ->
                    dailyNoteDates = notes
                        .filter { it.file_path.startsWith("Daily Notes/") }
                        .map { it.title }
                        .toSet()
                },
                onFailure = {}
            )
        }
    }

    // Lifecycle observer for ON_RESUME
    val lifecycleOwner = androidx.lifecycle.compose.LocalLifecycleOwner.current
    DisposableEffect(lifecycleOwner) {
        val observer = androidx.lifecycle.LifecycleEventObserver { _, event ->
            if (event == androidx.lifecycle.Lifecycle.Event.ON_RESUME) {
                viewModel.refreshOnResume()
                viewModel.updateRelativeSyncStatus()
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
            viewModel.updateRelativeSyncStatus()
        }
    }

    Scaffold(
        topBar = {
            if (isSelectionMode) {
                TopAppBar(
                    title = {
                        Text(
                            text = "${selectedNotes.size} selected",
                            fontWeight = FontWeight.Bold,
                            fontSize = 17.sp
                        )
                    },
                    navigationIcon = {
                        IconButton(onClick = {
                            isSelectionMode = false
                            selectedNotes.clear()
                        }) {
                            Icon(Icons.Default.Close, contentDescription = "Cancel Selection")
                        }
                    },
                    actions = {
                        // Pin/Unpin action
                        IconButton(onClick = {
                            selectedNotes.forEach { note ->
                                viewModel.togglePinNote(note)
                            }
                            isSelectionMode = false
                            selectedNotes.clear()
                        }) {
                            Icon(Icons.Default.PushPin, contentDescription = "Pin/Unpin Selected")
                        }
                        // Delete action
                        IconButton(onClick = {
                            selectedNotes.forEach { note ->
                                viewModel.deleteNoteWithUndo(note) { undoCallback ->
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
                            isSelectionMode = false
                            selectedNotes.clear()
                        }) {
                            Icon(Icons.Default.Delete, contentDescription = "Delete Selected")
                        }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.primaryContainer,
                        titleContentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                        navigationIconContentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                        actionIconContentColor = MaterialTheme.colorScheme.onPrimaryContainer
                    ),
                    windowInsets = WindowInsets(0, 0, 0, 0)
                )
            } else {
                TopAppBar(
                    title = {
                        Column {
                            Text(
                                text = (currentFolder ?: "All Notes") + (if (selectedTag != null) " • #$selectedTag" else ""),
                                fontWeight = FontWeight.Bold,
                                fontSize = 17.sp
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
                        IconButton(onClick = {
                            showCalendarDialog = true
                        }) {
                            Icon(Icons.Default.Today, contentDescription = "Open Calendar")
                        }
                        IconButton(onClick = onSearchClick) {
                            Icon(Icons.Default.Search, contentDescription = "Search")
                        }
                        IconButton(onClick = { viewModel.triggerSync() }) {
                            Icon(Icons.Default.Sync, contentDescription = "Sync Now")
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
                            DropdownMenuItem(
                                text = { Text("Created Date") },
                                onClick = {
                                    selectedSortOrder = SortOrder.CREATED
                                    showSortMenu = false
                                },
                                leadingIcon = {
                                    if (selectedSortOrder == SortOrder.CREATED) {
                                        Icon(Icons.Default.Check, contentDescription = null)
                                    }
                                }
                            )
                        }
                    },
                    windowInsets = WindowInsets(0, 0, 0, 0)
                )
            }
        },
        floatingActionButton = {
            AnimatedVisibility(
                visible = isFabVisible,
                enter = scaleIn() + fadeIn(),
                exit = scaleOut() + fadeOut()
            ) {
                FloatingActionButton(
                    onClick = {
                        if (currentFolder == "Daily Notes") {
                            scope.launch {
                                val repo = com.bubi.nodanotes.data.repository.NoteRepository()
                                repo.triggerDailyNote().fold(
                                    onSuccess = { noteDto ->
                                        onNavigateToEditor(noteDto.id)
                                    },
                                    onFailure = {
                                        // Fallback to normal note creation
                                        viewModel.createNote(currentFolder) { noteId ->
                                            onNavigateToEditor(noteId)
                                        }
                                    }
                                )
                            }
                        } else {
                            viewModel.createNote(currentFolder) { noteId ->
                                onNavigateToEditor(noteId)
                            }
                        }
                    },
                    shape = RoundedCornerShape(16.dp),
                    containerColor = MaterialTheme.colorScheme.primary,
                    contentColor = MaterialTheme.colorScheme.onPrimary
                ) {
                    Icon(Icons.Default.Add, contentDescription = "New Note")
                }
            }
        },
        snackbarHost = { SnackbarHost(snackbarHostState) }
    ) { paddingValues ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            Column(
                modifier = Modifier.fillMaxSize()
            ) {
            Surface(
                color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                onClick = onNavigateToSyncReport,
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

            val pullToRefreshState = rememberPullToRefreshState()
            var isDeepPull by remember { mutableStateOf(false) }

            LaunchedEffect(pullToRefreshState.distanceFraction, isRefreshing) {
                if (!isRefreshing) {
                    if (pullToRefreshState.distanceFraction >= 1.5f) {
                        isDeepPull = true
                    } else if (pullToRefreshState.distanceFraction < 1.0f) {
                        isDeepPull = false
                    }
                }
            }

            PullToRefreshBox(
                state = pullToRefreshState,
                isRefreshing = isRefreshing,
                onRefresh = {
                    if (isDeepPull) {
                        viewModel.triggerSync()
                    } else {
                        viewModel.triggerFilesystemScan()
                    }
                },
                indicator = {
                    val fraction = pullToRefreshState.distanceFraction
                    if (fraction > 0f || isRefreshing) {
                        Card(
                            colors = CardDefaults.cardColors(
                                containerColor = MaterialTheme.colorScheme.surfaceVariant
                            ),
                            elevation = CardDefaults.cardElevation(defaultElevation = 4.dp),
                            shape = RoundedCornerShape(24.dp),
                            modifier = Modifier
                                .align(Alignment.TopCenter)
                                .padding(top = 16.dp)
                        ) {
                            Row(
                                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.Center
                            ) {
                                if (isRefreshing) {
                                    CircularProgressIndicator(
                                        modifier = Modifier.size(18.dp),
                                        strokeWidth = 2.dp,
                                        color = if (isDeepPull) MaterialTheme.colorScheme.tertiary else MaterialTheme.colorScheme.primary
                                    )
                                } else {
                                    val rotation = (fraction * 360f) % 360f
                                    Icon(
                                        imageVector = if (fraction >= 1.5f) Icons.Default.Sync else Icons.Default.Refresh,
                                        contentDescription = null,
                                        tint = if (fraction >= 1.5f) MaterialTheme.colorScheme.tertiary else MaterialTheme.colorScheme.primary,
                                        modifier = Modifier
                                            .size(18.dp)
                                            .graphicsLayer(rotationZ = rotation)
                                    )
                                }
                                Spacer(modifier = Modifier.width(8.dp))
                                val text = when {
                                    isRefreshing -> if (isDeepPull) "Bulutla eşitleniyor..." else "Dosyalar taranıyor..."
                                    fraction >= 1.5f -> "Bulut eşitlemesi için bırakın..."
                                    else -> "Yerel tarama için bırakın..."
                                }
                                Text(
                                    text = text,
                                    style = MaterialTheme.typography.bodySmall,
                                    fontWeight = FontWeight.Bold,
                                    color = MaterialTheme.colorScheme.onSurface
                                )
                            }
                        }
                    }
                },
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
                            val sortedNotes = if (currentFolder == "Daily Notes") {
                                notes.sortedByDescending { it.title }
                            } else {
                                when (selectedSortOrder) {
                                    SortOrder.UPDATED -> notes.sortedByDescending { it.updated_at }
                                    SortOrder.TITLE -> notes.sortedBy { it.title.lowercase() }
                                    SortOrder.CREATED -> notes.sortedByDescending { it.id }
                                }
                            }

                            val pinnedNotes = sortedNotes.filter { it.pinned }
                            val remainingNotes = sortedNotes.filter { !it.pinned }

                            LazyColumn(
                                state = lazyListState,
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
                                            },
                                            isSelectionMode = isSelectionMode,
                                            isSelected = selectedNotes.contains(note),
                                            onToggleSelection = { toggledNote ->
                                                if (selectedNotes.contains(toggledNote)) {
                                                    selectedNotes.remove(toggledNote)
                                                    if (selectedNotes.isEmpty()) {
                                                        isSelectionMode = false
                                                    }
                                                } else {
                                                    selectedNotes.add(toggledNote)
                                                    isSelectionMode = true
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
                                            },
                                            isSelectionMode = isSelectionMode,
                                            isSelected = selectedNotes.contains(note),
                                            onToggleSelection = { toggledNote ->
                                                if (selectedNotes.contains(toggledNote)) {
                                                    selectedNotes.remove(toggledNote)
                                                    if (selectedNotes.isEmpty()) {
                                                        isSelectionMode = false
                                                    }
                                                } else {
                                                    selectedNotes.add(toggledNote)
                                                    isSelectionMode = true
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
    UPDATED, TITLE, CREATED
}
