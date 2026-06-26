package com.bubi.nodanotes.ui.components

import androidx.compose.animation.*
import androidx.compose.animation.core.*
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.ui.layout.onGloballyPositioned
import android.content.Context
import androidx.navigation.NavHostController
import androidx.navigation.compose.currentBackStackEntryAsState
import com.bubi.nodanotes.data.model.FolderDto
import com.bubi.nodanotes.data.preferences.VaultPreferences
import com.bubi.nodanotes.ui.navigation.NodaNavGraph
import com.bubi.nodanotes.ui.navigation.Screen
import kotlinx.coroutines.launch
import com.bubi.nodanotes.data.repository.UpdateManager
import com.bubi.nodanotes.data.model.UpdateState
import androidx.compose.foundation.Image
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.foundation.background


@OptIn(ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
@Composable
fun NodaAppShell(
    navController: NavHostController,
    startDestination: String,
    vaultPreferences: VaultPreferences,
    drawerViewModel: DrawerViewModel = viewModel()
) {
    val scope = rememberCoroutineScope()
    val drawerState = rememberDrawerState(initialValue = DrawerValue.Closed)
    val navBackStackEntry by navController.currentBackStackEntryAsState()
    val currentRoute = navBackStackEntry?.destination?.route

    // Disable drawer entirely on Vault Selector and Note Editor/Reader screens
    val isDrawerEnabled = currentRoute != Screen.VaultSelector.route && currentRoute?.startsWith("note_editor") != true

    val vaultPath = vaultPreferences.getVaultPath()
    val vaultName = vaultPath?.substringAfterLast('/') ?: "No Vault"

    // Check for deep link intents from dynamic notifications
    val context = androidx.compose.ui.platform.LocalContext.current
    LaunchedEffect(Unit) {
        val activity = context as? android.app.Activity
        val navigateTo = activity?.intent?.getStringExtra("navigate_to")
        if (navigateTo == "sync_report") {
            activity.intent.removeExtra("navigate_to") // Consume extra
            navController.navigate(Screen.SyncReport.route)
        }
    }

    // Folder states
    val folders by drawerViewModel.folders.collectAsState()
    val expandedPaths by drawerViewModel.expandedPaths.collectAsState()
    val isFoldersLoading by drawerViewModel.isLoading.collectAsState()

    // Dialog trigger states
    var activeFolderForAction by remember { mutableStateOf<FolderDto?>(null) }
    var showCreateSubfolderDialog by remember { mutableStateOf(false) }
    var showRenameDialog by remember { mutableStateOf(false) }
    var showDeleteConfirmDialog by remember { mutableStateOf(false) }
    var showCreateTopLevelFolderDialog by remember { mutableStateOf(false) }

    // Text inputs for dialogs
    var folderInputName by remember { mutableStateOf("") }

    // We don't trigger full reload on open because CRUD operations already trigger reload,
    // and periodic sync / other watchers can sync state correctly. This keeps the drawer perfectly smooth.
    LaunchedEffect(drawerState.isOpen) {
        if (drawerState.isOpen) {
            // Only fetch light count updates and avoid full folder tree rebuild if unnecessary
            drawerViewModel.loadFolders()
        }
    }

    val drawerContent: @Composable () -> Unit = {
        val context = androidx.compose.ui.platform.LocalContext.current
        val sharedPrefs = remember { context.getSharedPreferences("noda_prefs", Context.MODE_PRIVATE) }
        var workspaceExpanded by remember { mutableStateOf(sharedPrefs.getBoolean("workspace_expanded", true)) }
        var foldersExpanded by remember { mutableStateOf(sharedPrefs.getBoolean("folders_expanded", true)) }
        var tagsExpanded by remember { mutableStateOf(sharedPrefs.getBoolean("tags_expanded", true)) }
        var managementExpanded by remember { mutableStateOf(sharedPrefs.getBoolean("management_expanded", true)) }
        var foldersRatio by remember { mutableStateOf(sharedPrefs.getFloat("folders_ratio", 0.5f)) }

        var totalHeightPx by remember { mutableStateOf(1f) }

        val currentFolder by com.bubi.nodanotes.ui.screens.notelist.FolderContext.currentFolderState.collectAsState()
        val selectedTag by com.bubi.nodanotes.ui.screens.notelist.FolderContext.selectedTagState.collectAsState()
        val showOnlyEncrypted by com.bubi.nodanotes.ui.screens.notelist.FolderContext.showOnlyEncryptedState.collectAsState()

        ModalDrawerSheet(
            modifier = Modifier.width(280.dp),
            drawerShape = RoundedCornerShape(topEnd = 16.dp, bottomEnd = 16.dp),
            drawerContainerColor = MaterialTheme.colorScheme.surface
        ) {
            Column(modifier = Modifier.fillMaxSize()) {
                // --- WORKSPACE (Fixed Top) ---
                Column(modifier = Modifier.fillMaxWidth()) {
                    Spacer(modifier = Modifier.height(12.dp))

                    // Header
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 16.dp, vertical = 16.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Box(
                            modifier = Modifier
                                .size(40.dp)
                                .clip(RoundedCornerShape(8.dp))
                                .background(MaterialTheme.colorScheme.primaryContainer),
                            contentAlignment = Alignment.Center
                        ) {
                            Image(
                                painter = painterResource(id = com.bubi.nodanotes.R.drawable.ic_launcher_foreground),
                                contentDescription = "Noda Logo",
                                modifier = Modifier.size(36.dp)
                            )
                        }
                        Spacer(modifier = Modifier.width(12.dp))
                        Column {
                            Text(
                                text = "NodaNotes",
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                                color = MaterialTheme.colorScheme.onSurface,
                                fontSize = 18.sp
                            )
                            Text(
                                text = vaultName,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                fontSize = 12.sp,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(8.dp))

                    // WORKSPACE accordion header
                    SidebarSectionHeader(
                        title = "Workspace",
                        icon = Icons.Default.Work,
                        expanded = workspaceExpanded,
                        onHeaderClick = {
                            workspaceExpanded = !workspaceExpanded
                            sharedPrefs.edit().putBoolean("workspace_expanded", workspaceExpanded).apply()
                        }
                    )

                    AnimatedVisibility(
                        visible = workspaceExpanded,
                        enter = expandVertically() + fadeIn(),
                        exit = shrinkVertically() + fadeOut()
                    ) {
                        Column(modifier = Modifier.fillMaxWidth()) {
                            // All Notes
                            NavigationDrawerItem(
                                icon = { Icon(Icons.Default.Description, contentDescription = null, modifier = Modifier.size(18.dp)) },
                                label = { Text("All Notes", fontWeight = FontWeight.SemiBold, fontSize = 15.sp) },
                                selected = currentRoute == Screen.NoteList.route && currentFolder == null && selectedTag == null && !showOnlyEncrypted,
                                onClick = {
                                    scope.launch { drawerState.close() }
                                    com.bubi.nodanotes.ui.screens.notelist.FolderContext.currentFolder = null
                                    com.bubi.nodanotes.ui.screens.notelist.FolderContext.selectedTag = null
                                    com.bubi.nodanotes.ui.screens.notelist.FolderContext.showOnlyEncrypted = false
                                    navController.navigate(Screen.NoteList.route) {
                                        popUpTo(Screen.NoteList.route) { inclusive = true }
                                    }
                                },
                                modifier = Modifier
                                    .padding(horizontal = 8.dp, vertical = 1.dp)
                                    .height(30.dp)
                            )

                            // Daily Notes
                            NavigationDrawerItem(
                                icon = { Icon(Icons.Default.Today, contentDescription = null, modifier = Modifier.size(18.dp)) },
                                label = { Text("Daily Notes", fontWeight = FontWeight.SemiBold, fontSize = 15.sp) },
                                selected = currentRoute == Screen.NoteList.route && currentFolder == "Daily Notes" && selectedTag == null && !showOnlyEncrypted,
                                onClick = {
                                    scope.launch { drawerState.close() }
                                    com.bubi.nodanotes.ui.screens.notelist.FolderContext.currentFolder = "Daily Notes"
                                    com.bubi.nodanotes.ui.screens.notelist.FolderContext.selectedTag = null
                                    com.bubi.nodanotes.ui.screens.notelist.FolderContext.showOnlyEncrypted = false
                                    navController.navigate(Screen.NoteList.route) {
                                        popUpTo(Screen.NoteList.route) { inclusive = true }
                                    }
                                },
                                modifier = Modifier
                                    .padding(horizontal = 8.dp, vertical = 1.dp)
                                    .height(30.dp)
                            )

                            // Encrypted Notes
                            NavigationDrawerItem(
                                icon = { Icon(Icons.Default.Lock, contentDescription = null, modifier = Modifier.size(18.dp)) },
                                label = { Text("Encrypted Notes", fontWeight = FontWeight.SemiBold, fontSize = 15.sp) },
                                selected = currentRoute == Screen.NoteList.route && showOnlyEncrypted && selectedTag == null,
                                onClick = {
                                    scope.launch { drawerState.close() }
                                    com.bubi.nodanotes.ui.screens.notelist.FolderContext.currentFolder = null
                                    com.bubi.nodanotes.ui.screens.notelist.FolderContext.selectedTag = null
                                    com.bubi.nodanotes.ui.screens.notelist.FolderContext.showOnlyEncrypted = true
                                    navController.navigate(Screen.NoteList.route) {
                                        popUpTo(Screen.NoteList.route) { inclusive = true }
                                    }
                                },
                                modifier = Modifier
                                    .padding(horizontal = 8.dp, vertical = 1.dp)
                                    .height(30.dp)
                            )
                        }
                    }
                }

                Spacer(modifier = Modifier.height(8.dp))

                // --- MIDDLE RESIZABLE SECTION: FOLDERS & TAGS ---
                Column(
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxWidth()
                        .onGloballyPositioned { coordinates ->
                            totalHeightPx = coordinates.size.height.toFloat().coerceAtLeast(1f)
                        }
                ) {
                    // --- FOLDERS ACCORDION ---
                    SidebarSectionHeader(
                        title = "Folders",
                        icon = Icons.Default.Folder,
                        expanded = foldersExpanded,
                        onHeaderClick = {
                            foldersExpanded = !foldersExpanded
                            sharedPrefs.edit().putBoolean("folders_expanded", foldersExpanded).apply()
                        },
                        actionIcon = Icons.Default.CreateNewFolder,
                        actionDescription = "New Folder",
                        onActionClick = {
                            folderInputName = ""
                            showCreateTopLevelFolderDialog = true
                        }
                    )

                    if (foldersExpanded) {
                        val filteredFolders = folders.filter { it.name != "Daily Notes" }
                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .weight(if (tagsExpanded) foldersRatio else 1f)
                        ) {
                            if (isFoldersLoading && filteredFolders.isEmpty()) {
                                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                                    CircularProgressIndicator(modifier = Modifier.size(20.dp))
                                }
                            } else if (filteredFolders.isEmpty()) {
                                Row(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .padding(horizontal = 16.dp, vertical = 8.dp),
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Icon(
                                        imageVector = Icons.Default.Folder,
                                        contentDescription = null,
                                        tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.4f),
                                        modifier = Modifier.size(16.dp)
                                    )
                                    Spacer(modifier = Modifier.width(8.dp))
                                    Text(
                                        text = "No folders created yet",
                                        color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f),
                                        fontSize = 14.sp
                                    )
                                }
                            } else {
                                LazyColumn(
                                    modifier = Modifier.fillMaxSize(),
                                    contentPadding = PaddingValues(vertical = 2.dp)
                                ) {
                                    items(filteredFolders) { folder ->
                                        FolderTreeItem(
                                            folder = folder,
                                            depth = 0,
                                            expandedPaths = expandedPaths,
                                            currentFolderPath = currentFolder,
                                            onToggleExpanded = { drawerViewModel.toggleExpanded(it) },
                                            onFolderClick = { path ->
                                                scope.launch { drawerState.close() }
                                                com.bubi.nodanotes.ui.screens.notelist.FolderContext.selectedTag = null
                                                com.bubi.nodanotes.ui.screens.notelist.FolderContext.showOnlyEncrypted = false
                                                com.bubi.nodanotes.ui.screens.notelist.FolderContext.currentFolder = path
                                                navController.navigate(Screen.NoteList.route) {
                                                    popUpTo(Screen.NoteList.route) { inclusive = true }
                                                }
                                            },
                                            onLongPressFolder = { folderSelected ->
                                                activeFolderForAction = folderSelected
                                                showRenameDialog = false
                                                showDeleteConfirmDialog = false
                                                showCreateSubfolderDialog = false
                                            }
                                        )
                                    }
                                }
                            }
                        }
                    }

                    // --- DRAGGABLE SPLITTER ---
                    if (foldersExpanded && tagsExpanded) {
                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(12.dp)
                                .pointerInput(Unit) {
                                    detectDragGestures { change, dragAmount ->
                                        change.consume()
                                        val deltaRatio = dragAmount.y / totalHeightPx
                                        foldersRatio = (foldersRatio + deltaRatio).coerceIn(0.1f, 0.9f)
                                        sharedPrefs.edit().putFloat("folders_ratio", foldersRatio).apply()
                                    }
                                },
                            contentAlignment = Alignment.Center
                        ) {
                            Surface(
                                color = MaterialTheme.colorScheme.primary,
                                shape = RoundedCornerShape(2.dp),
                                modifier = Modifier
                                    .width(40.dp)
                                    .height(4.dp)
                            ) {}
                        }
                    }

                    // --- TAGS ACCORDION ---
                    // Header
                    val tags by drawerViewModel.tags.collectAsState()
                    SidebarSectionHeader(
                        title = "Tags",
                        icon = Icons.Default.LocalOffer,
                        expanded = tagsExpanded,
                        onHeaderClick = {
                            tagsExpanded = !tagsExpanded
                            sharedPrefs.edit().putBoolean("tags_expanded", tagsExpanded).apply()
                        }
                    )

                    if (tagsExpanded) {
                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .weight(if (foldersExpanded) 1f - foldersRatio else 1f)
                        ) {
                            if (tags.isEmpty()) {
                                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                                    Text(
                                        text = "No tags found",
                                        color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f),
                                        fontSize = 14.sp
                                    )
                                }
                            } else {
                                LazyColumn(
                                    modifier = Modifier.fillMaxSize(),
                                    contentPadding = PaddingValues(horizontal = 8.dp, vertical = 2.dp)
                                ) {
                                    items(tags) { tagDto ->
                                        val isSelected = selectedTag == tagDto.name
                                        NavigationDrawerItem(
                                            icon = {
                                                Icon(
                                                    imageVector = Icons.Default.LocalOffer,
                                                    contentDescription = null,
                                                    modifier = Modifier.size(14.dp),
                                                    tint = if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant
                                                )
                                            },
                                            label = {
                                                Row(
                                                    modifier = Modifier.fillMaxWidth(),
                                                    horizontalArrangement = Arrangement.SpaceBetween,
                                                    verticalAlignment = Alignment.CenterVertically
                                                ) {
                                                    Text(
                                                        text = tagDto.name,
                                                        fontWeight = if (isSelected) FontWeight.Bold else FontWeight.Normal,
                                                        fontSize = 14.sp
                                                    )
                                                    Badge(
                                                        containerColor = MaterialTheme.colorScheme.primaryContainer,
                                                        contentColor = MaterialTheme.colorScheme.onPrimaryContainer
                                                    ) {
                                                        Text(tagDto.count.toString(), fontSize = 9.sp)
                                                    }
                                                }
                                            },
                                            selected = isSelected,
                                            onClick = {
                                                scope.launch { drawerState.close() }
                                                com.bubi.nodanotes.ui.screens.notelist.FolderContext.currentFolder = null
                                                com.bubi.nodanotes.ui.screens.notelist.FolderContext.selectedTag = tagDto.name
                                                com.bubi.nodanotes.ui.screens.notelist.FolderContext.showOnlyEncrypted = false
                                                navController.navigate(Screen.NoteList.route) {
                                                    popUpTo(Screen.NoteList.route) { inclusive = true }
                                                }
                                            },
                                            modifier = Modifier
                                                .padding(vertical = 1.dp)
                                                .height(30.dp)
                                        )
                                    }
                                }
                            }
                        }
                    }
                }

                Spacer(modifier = Modifier.height(8.dp))

                // --- MANAGEMENT & SETTINGS (Fixed Bottom) ---
                val trashCount by drawerViewModel.trashCount.collectAsState()
                val conflictCount by drawerViewModel.conflictCount.collectAsState()
                val attachmentCount by drawerViewModel.attachmentCount.collectAsState()

                Column(modifier = Modifier.fillMaxWidth()) {
                    // Header
                    SidebarSectionHeader(
                        title = "Management & Settings",
                        icon = Icons.Default.Settings,
                        expanded = managementExpanded,
                        onHeaderClick = {
                            managementExpanded = !managementExpanded
                            sharedPrefs.edit().putBoolean("management_expanded", managementExpanded).apply()
                        }
                    )

                    AnimatedVisibility(
                        visible = managementExpanded,
                        enter = expandVertically() + fadeIn(),
                        exit = shrinkVertically() + fadeOut()
                    ) {
                        Column(modifier = Modifier.fillMaxWidth()) {
                            // Trash
                            NavigationDrawerItem(
                                icon = { Icon(Icons.Default.Delete, contentDescription = null, modifier = Modifier.size(18.dp)) },
                                label = {
                                    Row(
                                        modifier = Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically
                                    ) {
                                        Text("Trash", fontSize = 15.sp)
                                        if (trashCount > 0) {
                                            Badge(
                                                containerColor = MaterialTheme.colorScheme.errorContainer,
                                                contentColor = MaterialTheme.colorScheme.onErrorContainer
                                            ) {
                                                Text(trashCount.toString(), fontSize = 9.sp)
                                            }
                                        }
                                    }
                                },
                                selected = currentRoute == Screen.Trash.route,
                                onClick = {
                                    scope.launch { drawerState.close() }
                                    navController.navigate(Screen.Trash.route)
                                },
                                modifier = Modifier
                                    .padding(horizontal = 8.dp, vertical = 1.dp)
                                    .height(30.dp)
                            )

                            // Conflicts
                            NavigationDrawerItem(
                                icon = { Icon(Icons.Default.Difference, contentDescription = null, modifier = Modifier.size(18.dp)) },
                                label = {
                                    Row(
                                        modifier = Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically
                                    ) {
                                        Text("Conflicts", fontSize = 15.sp)
                                        if (conflictCount > 0) {
                                            Badge(
                                                containerColor = MaterialTheme.colorScheme.errorContainer,
                                                contentColor = MaterialTheme.colorScheme.onErrorContainer
                                            ) {
                                                Text(conflictCount.toString(), fontSize = 9.sp)
                                            }
                                        }
                                    }
                                },
                                selected = currentRoute == Screen.Conflicts.route,
                                onClick = {
                                    scope.launch { drawerState.close() }
                                    navController.navigate(Screen.Conflicts.route)
                                },
                                modifier = Modifier
                                    .padding(horizontal = 8.dp, vertical = 1.dp)
                                    .height(30.dp)
                            )

                            // Attachments
                            NavigationDrawerItem(
                                icon = { Icon(Icons.Default.Attachment, contentDescription = null, modifier = Modifier.size(18.dp)) },
                                label = {
                                    Row(
                                        modifier = Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically
                                    ) {
                                        Text("Attachments", fontSize = 15.sp)
                                        if (attachmentCount > 0) {
                                            Badge(
                                                containerColor = MaterialTheme.colorScheme.primaryContainer,
                                                contentColor = MaterialTheme.colorScheme.onPrimaryContainer
                                            ) {
                                                Text(attachmentCount.toString(), fontSize = 9.sp)
                                            }
                                        }
                                    }
                                },
                                selected = currentRoute == Screen.Attachments.route,
                                onClick = {
                                    scope.launch { drawerState.close() }
                                    navController.navigate(Screen.Attachments.route)
                                },
                                modifier = Modifier
                                    .padding(horizontal = 8.dp, vertical = 1.dp)
                                    .height(30.dp)
                            )

                            // Settings
                            val updateState by UpdateManager.updateState.collectAsState()
                            val isUpdateAvailable = updateState is UpdateState.FlexibleUpdate
                            NavigationDrawerItem(
                                icon = { Icon(Icons.Default.Settings, contentDescription = null, modifier = Modifier.size(18.dp)) },
                                label = { Text("Settings", fontSize = 15.sp) },
                                badge = {
                                    if (isUpdateAvailable) {
                                        Badge(
                                            containerColor = MaterialTheme.colorScheme.error,
                                            modifier = Modifier.padding(end = 4.dp)
                                        ) {
                                            Text("!", color = MaterialTheme.colorScheme.onError, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                                        }
                                    }
                                },
                                selected = currentRoute == Screen.Settings.route,
                                onClick = {
                                    scope.launch { drawerState.close() }
                                    navController.navigate(Screen.Settings.route)
                                },
                                modifier = Modifier
                                    .padding(horizontal = 8.dp, vertical = 1.dp)
                                    .height(30.dp)
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(12.dp))
                }
            }
        }
    }

    // Modal Sheet or Dropdown for long press folder actions
    activeFolderForAction?.let { folder ->
        AlertDialog(
            onDismissRequest = { activeFolderForAction = null },
            title = { Text(folder.name) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Select action for this folder:")
                }
            },
            confirmButton = {},
            dismissButton = {
                Column(
                    modifier = Modifier.fillMaxWidth(),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Button(
                        onClick = {
                            folderInputName = ""
                            showCreateSubfolderDialog = true
                        },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Icon(Icons.Default.CreateNewFolder, contentDescription = null)
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("Create Subfolder")
                    }
                    Button(
                        onClick = {
                            folderInputName = folder.name
                            showRenameDialog = true
                        },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Icon(Icons.Default.Edit, contentDescription = null)
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("Rename Folder")
                    }
                    Button(
                        onClick = {
                            showDeleteConfirmDialog = true
                        },
                        colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Icon(Icons.Default.Delete, contentDescription = null)
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("Delete Folder")
                    }
                    OutlinedButton(
                        onClick = { activeFolderForAction = null },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Cancel")
                    }
                }
            }
        )
    }

    // Dialogs for Folder CRUD Operations
    if (showCreateTopLevelFolderDialog) {
        AlertDialog(
            onDismissRequest = { showCreateTopLevelFolderDialog = false },
            title = { Text("Create Folder") },
            text = {
                OutlinedTextField(
                    value = folderInputName,
                    onValueChange = { folderInputName = it },
                    label = { Text("Folder Name") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth()
                )
            },
            confirmButton = {
                Button(
                    onClick = {
                        if (folderInputName.trim().isNotEmpty()) {
                            drawerViewModel.createFolder(null, folderInputName.trim())
                            showCreateTopLevelFolderDialog = false
                        }
                    }
                ) {
                    Text("Create")
                }
            },
            dismissButton = {
                TextButton(onClick = { showCreateTopLevelFolderDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }

    if (showCreateSubfolderDialog && activeFolderForAction != null) {
        AlertDialog(
            onDismissRequest = { showCreateSubfolderDialog = false },
            title = { Text("Create Subfolder in ${activeFolderForAction?.name}") },
            text = {
                OutlinedTextField(
                    value = folderInputName,
                    onValueChange = { folderInputName = it },
                    label = { Text("Subfolder Name") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth()
                )
            },
            confirmButton = {
                Button(
                    onClick = {
                        if (folderInputName.trim().isNotEmpty()) {
                            drawerViewModel.createFolder(activeFolderForAction?.path, folderInputName.trim())
                            showCreateSubfolderDialog = false
                            activeFolderForAction = null
                        }
                    }
                ) {
                    Text("Create")
                }
            },
            dismissButton = {
                TextButton(onClick = { showCreateSubfolderDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }

    if (showRenameDialog && activeFolderForAction != null) {
        AlertDialog(
            onDismissRequest = { showRenameDialog = false },
            title = { Text("Rename Folder") },
            text = {
                OutlinedTextField(
                    value = folderInputName,
                    onValueChange = { folderInputName = it },
                    label = { Text("New Name") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth()
                )
            },
            confirmButton = {
                Button(
                    onClick = {
                        if (folderInputName.trim().isNotEmpty() && folderInputName != activeFolderForAction?.name) {
                            drawerViewModel.renameFolder(activeFolderForAction!!.path, folderInputName.trim())
                            showRenameDialog = false
                            activeFolderForAction = null
                        }
                    }
                ) {
                    Text("Rename")
                }
            },
            dismissButton = {
                TextButton(onClick = { showRenameDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }

    if (showDeleteConfirmDialog && activeFolderForAction != null) {
        AlertDialog(
            onDismissRequest = { showDeleteConfirmDialog = false },
            title = { Text("Delete Folder") },
            text = { Text("Are you sure you want to delete folder '${activeFolderForAction?.name}'? All notes inside will be permanently lost.") },
            confirmButton = {
                Button(
                    onClick = {
                        drawerViewModel.deleteFolder(activeFolderForAction!!.path)
                        showDeleteConfirmDialog = false
                        activeFolderForAction = null
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error)
                ) {
                    Text("Delete")
                }
            },
            dismissButton = {
                TextButton(onClick = { showDeleteConfirmDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }

    ModalNavigationDrawer(
        drawerState = drawerState,
        gesturesEnabled = isDrawerEnabled,
        drawerContent = drawerContent
    ) {
        Scaffold(
            modifier = Modifier.fillMaxSize()
        ) { innerPadding ->
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(innerPadding)
            ) {
                NodaNavGraph(
                    navController = navController,
                    onMenuClick = { scope.launch { drawerState.open() } },
                    startDestination = startDestination
                )
            }
        }
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
fun FolderTreeItem(
    folder: FolderDto,
    depth: Int,
    expandedPaths: Set<String>,
    currentFolderPath: String?,
    onToggleExpanded: (String) -> Unit,
    onFolderClick: (String?) -> Unit,
    onLongPressFolder: (FolderDto) -> Unit
) {
    val isExpanded = expandedPaths.contains(folder.path)
    val hasChildren = folder.children.isNotEmpty()
    val isSelected = currentFolderPath == folder.path

    Column {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .height(30.dp)
                .padding(horizontal = 8.dp, vertical = 2.dp)
                .clip(RoundedCornerShape(8.dp))
                .combinedClickable(
                    onClick = { onFolderClick(folder.path) },
                    onLongClick = { onLongPressFolder(folder) }
                )
                .background(
                    if (isSelected) MaterialTheme.colorScheme.secondaryContainer 
                    else androidx.compose.ui.graphics.Color.Transparent
                )
                .padding(
                    start = (12 * depth + 4).dp,
                    top = 6.dp,
                    bottom = 6.dp,
                    end = 12.dp
                ),
            verticalAlignment = Alignment.CenterVertically
        ) {
            IconButton(
                onClick = { onToggleExpanded(folder.path) },
                modifier = Modifier.size(20.dp)
            ) {
                if (hasChildren) {
                    val chevronRotation by animateFloatAsState(
                        targetValue = if (isExpanded) 90f else 0f,
                        animationSpec = tween(durationMillis = 150),
                        label = "folder_chevron"
                    )
                    Icon(
                        imageVector = Icons.Default.ChevronRight,
                        contentDescription = null,
                        modifier = Modifier
                            .size(14.dp)
                            .graphicsLayer(rotationZ = chevronRotation),
                        tint = if (isSelected) MaterialTheme.colorScheme.onSecondaryContainer else MaterialTheme.colorScheme.onSurfaceVariant
                    )
                } else {
                    Spacer(modifier = Modifier.size(14.dp))
                }
            }
            Spacer(modifier = Modifier.width(4.dp))
            Icon(
                imageVector = if (isExpanded) Icons.Default.FolderOpen else Icons.Default.Folder,
                contentDescription = null,
                tint = if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.size(18.dp)
            )
            Spacer(modifier = Modifier.width(8.dp))
            Text(
                text = folder.name,
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = if (isSelected) FontWeight.Bold else FontWeight.Medium,
                fontSize = 15.sp,
                color = if (isSelected) MaterialTheme.colorScheme.onSecondaryContainer else MaterialTheme.colorScheme.onSurface
            )
        }

        if (isExpanded && hasChildren) {
            folder.children.forEach { child ->
                FolderTreeItem(
                    folder = child,
                    depth = depth + 1,
                    expandedPaths = expandedPaths,
                    currentFolderPath = currentFolderPath,
                    onToggleExpanded = onToggleExpanded,
                    onFolderClick = onFolderClick,
                    onLongPressFolder = onLongPressFolder
                )
            }
        }
    }
}

@Composable
private fun SidebarSectionHeader(
    title: String,
    icon: ImageVector,
    expanded: Boolean,
    onHeaderClick: () -> Unit,
    actionIcon: ImageVector? = null,
    actionDescription: String? = null,
    onActionClick: (() -> Unit)? = null
) {
    val rotationState by animateFloatAsState(
        targetValue = if (expanded) 180f else 0f,
        animationSpec = tween(durationMillis = 200, easing = FastOutSlowInEasing),
        label = "chevron_rotation"
    )

    Surface(
        onClick = onHeaderClick,
        color = androidx.compose.ui.graphics.Color.Transparent,
        modifier = Modifier
            .fillMaxWidth()
            .height(40.dp)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.primary,
                modifier = Modifier.size(18.dp)
            )
            Spacer(modifier = Modifier.width(10.dp))
            Text(
                text = title.uppercase(),
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.8f),
                fontWeight = FontWeight.Bold,
                letterSpacing = 1.sp,
                modifier = Modifier.weight(1f)
            )
            if (actionIcon != null && onActionClick != null) {
                IconButton(
                    onClick = onActionClick,
                    modifier = Modifier.size(24.dp)
                ) {
                    Icon(
                        imageVector = actionIcon,
                        contentDescription = actionDescription,
                        modifier = Modifier.size(16.dp),
                        tint = MaterialTheme.colorScheme.primary
                    )
                }
                Spacer(modifier = Modifier.width(8.dp))
            }
            Icon(
                imageVector = Icons.Default.ExpandMore,
                contentDescription = if (expanded) "Collapse" else "Expand",
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier
                    .size(18.dp)
                    .graphicsLayer(rotationZ = rotationState)
            )
        }
    }
}
