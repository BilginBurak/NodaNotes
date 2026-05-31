package com.bubi.nodanotes.ui.components

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
import androidx.navigation.NavHostController
import androidx.navigation.compose.currentBackStackEntryAsState
import com.bubi.nodanotes.data.model.FolderDto
import com.bubi.nodanotes.data.preferences.VaultPreferences
import com.bubi.nodanotes.ui.navigation.NodaNavGraph
import com.bubi.nodanotes.ui.navigation.Screen
import kotlinx.coroutines.launch

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
        ModalDrawerSheet(
            modifier = Modifier.width(280.dp),
            drawerShape = RoundedCornerShape(topEnd = 16.dp, bottomEnd = 16.dp)
        ) {
            Spacer(modifier = Modifier.height(12.dp))

            // Drawer Header
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(
                    imageVector = Icons.Default.StickyNote2,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(28.dp)
                )
                Spacer(modifier = Modifier.width(12.dp))
                Column {
                    Text(
                        text = "NodaNotes",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface,
                        fontSize = 16.sp
                    )
                    Text(
                        text = vaultName,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        fontSize = 13.sp
                    )
                }
            }

            HorizontalDivider(modifier = Modifier.padding(vertical = 16.dp), color = MaterialTheme.colorScheme.outlineVariant)

            val currentFolder by com.bubi.nodanotes.ui.screens.notelist.FolderContext.currentFolderState.collectAsState()

            // Drawer Items
            NavigationDrawerItem(
                icon = { Icon(Icons.Default.Description, contentDescription = null, modifier = Modifier.size(18.dp)) },
                label = { Text("All Notes", fontWeight = FontWeight.SemiBold, fontSize = 16.sp) },
                selected = currentRoute == Screen.NoteList.route && currentFolder == null && com.bubi.nodanotes.ui.screens.notelist.FolderContext.selectedTag == null,
                onClick = {
                    scope.launch { drawerState.close() }
                    com.bubi.nodanotes.ui.screens.notelist.FolderContext.currentFolder = null
                    com.bubi.nodanotes.ui.screens.notelist.FolderContext.selectedTag = null
                    navController.navigate(Screen.NoteList.route) {
                        popUpTo(Screen.NoteList.route) { inclusive = true }
                    }
                },
                modifier = Modifier
                    .padding(horizontal = 8.dp, vertical = 1.dp)
                    .height(40.dp)
            )

            // Folders Section Header
            Spacer(modifier = Modifier.height(8.dp))
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 2.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "FOLDERS",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
                    fontWeight = FontWeight.Bold,
                    fontSize = 16.sp
                )
                IconButton(
                    onClick = {
                        folderInputName = ""
                        showCreateTopLevelFolderDialog = true
                    },
                    modifier = Modifier.size(20.dp)
                ) {
                    Icon(
                        imageVector = Icons.Default.CreateNewFolder,
                        contentDescription = "New Folder",
                        modifier = Modifier.size(16.dp),
                        tint = MaterialTheme.colorScheme.primary
                    )
                }
            }

            // Folder Tree list inside Navigation Drawer
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .weight(0.5f)
            ) {
                if (isFoldersLoading && folders.isEmpty()) {
                    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        CircularProgressIndicator(modifier = Modifier.size(20.dp))
                    }
                } else if (folders.isEmpty()) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 16.dp, vertical = 4.dp),
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
                            fontSize = 15.sp
                        )
                    }
                } else {
                    LazyColumn(
                        modifier = Modifier.fillMaxSize(),
                        contentPadding = PaddingValues(vertical = 2.dp)
                    ) {
                        items(folders) { folder ->
                            FolderTreeItem(
                                folder = folder,
                                depth = 0,
                                expandedPaths = expandedPaths,
                                onToggleExpanded = { drawerViewModel.toggleExpanded(it) },
                                onFolderClick = { path ->
                                    scope.launch { drawerState.close() }
                                    com.bubi.nodanotes.ui.screens.notelist.FolderContext.selectedTag = null
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

            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant, modifier = Modifier.padding(vertical = 4.dp))

            // TAGS Section Header
            val tags by drawerViewModel.tags.collectAsState()
            val selectedTag by com.bubi.nodanotes.ui.screens.notelist.FolderContext.selectedTagState.collectAsState()

            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(
                    imageVector = Icons.Default.LocalOffer,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(16.dp)
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = "TAGS",
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
                    fontWeight = FontWeight.Bold,
                    fontSize = 16.sp
                )
            }

            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .weight(0.5f)
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
                        items(tags) { tag ->
                            val isSelected = selectedTag == tag
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
                                    Text(
                                        text = tag,
                                        fontWeight = if (isSelected) FontWeight.Bold else FontWeight.Normal,
                                        fontSize = 14.sp
                                    )
                                },
                                selected = isSelected,
                                onClick = {
                                    scope.launch { drawerState.close() }
                                    com.bubi.nodanotes.ui.screens.notelist.FolderContext.currentFolder = null
                                    com.bubi.nodanotes.ui.screens.notelist.FolderContext.selectedTag = tag
                                    navController.navigate(Screen.NoteList.route) {
                                        popUpTo(Screen.NoteList.route) { inclusive = true }
                                    }
                                },
                                modifier = Modifier
                                    .padding(vertical = 1.dp)
                                    .height(36.dp)
                            )
                        }
                    }
                }
            }

            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant, modifier = Modifier.padding(vertical = 6.dp))

            val trashCount by drawerViewModel.trashCount.collectAsState()
            val conflictCount by drawerViewModel.conflictCount.collectAsState()
            val attachmentCount by drawerViewModel.attachmentCount.collectAsState()

            // Static links section at the bottom
            NavigationDrawerItem(
                icon = { Icon(Icons.Default.Delete, contentDescription = null, modifier = Modifier.size(18.dp)) },
                label = {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text("Trash", fontSize = 16.sp)
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

            NavigationDrawerItem(
                icon = { Icon(Icons.Default.Difference, contentDescription = null, modifier = Modifier.size(18.dp)) },
                label = {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text("Conflicts", fontSize = 16.sp)
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

            NavigationDrawerItem(
                icon = { Icon(Icons.Default.Attachment, contentDescription = null, modifier = Modifier.size(18.dp)) },
                label = {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text("Attachments", fontSize = 16.sp)
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

            NavigationDrawerItem(
                icon = { Icon(Icons.Default.Settings, contentDescription = null, modifier = Modifier.size(18.dp)) },
                label = { Text("Settings", fontSize = 16.sp) },
                selected = currentRoute == Screen.Settings.route,
                onClick = {
                    scope.launch { drawerState.close() }
                    navController.navigate(Screen.Settings.route)
                },
                modifier = Modifier
                    .padding(horizontal = 8.dp, vertical = 1.dp)
                    .height(30.dp)
            )

            Spacer(modifier = Modifier.height(12.dp))
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
    onToggleExpanded: (String) -> Unit,
    onFolderClick: (String?) -> Unit,
    onLongPressFolder: (FolderDto) -> Unit
) {
    val isExpanded = expandedPaths.contains(folder.path)
    val hasChildren = folder.children.isNotEmpty()

    Column {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .combinedClickable(
                    onClick = { onFolderClick(folder.path) },
                    onLongClick = { onLongPressFolder(folder) }
                )
                .padding(start = (12 * depth + 12).dp, top = 4.dp, bottom = 4.dp, end = 12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            IconButton(
                onClick = { onToggleExpanded(folder.path) },
                modifier = Modifier.size(20.dp)
            ) {
                if (hasChildren) {
                    Icon(
                        imageVector = if (isExpanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                        contentDescription = null,
                        modifier = Modifier.size(14.dp)
                    )
                } else {
                    Spacer(modifier = Modifier.size(14.dp))
                }
            }
            Spacer(modifier = Modifier.width(4.dp))
            Icon(
                imageVector = Icons.Default.Folder,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.primary,
                modifier = Modifier.size(18.dp)
            )
            Spacer(modifier = Modifier.width(8.dp))
            Text(
                text = folder.name,
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.Medium,
                fontSize = 15.sp,
                color = MaterialTheme.colorScheme.onSurface
            )
        }

        if (isExpanded && hasChildren) {
            folder.children.forEach { child ->
                FolderTreeItem(
                    folder = child,
                    depth = depth + 1,
                    expandedPaths = expandedPaths,
                    onToggleExpanded = onToggleExpanded,
                    onFolderClick = onFolderClick,
                    onLongPressFolder = onLongPressFolder
                )
            }
        }
    }
}
