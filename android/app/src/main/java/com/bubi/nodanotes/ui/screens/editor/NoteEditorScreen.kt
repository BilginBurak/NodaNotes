package com.bubi.nodanotes.ui.screens.editor

import androidx.compose.animation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.bubi.nodanotes.ui.components.FormattingToolbar
import com.bubi.nodanotes.ui.components.NoteInfoSheet
import com.bubi.nodanotes.ui.components.TagInputBar

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NoteEditorScreen(
    noteId: String,
    onBackClick: () -> Unit,
    onNavigateToHistory: (String) -> Unit,
    viewModel: NoteEditorViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsState()
    val saveState by viewModel.saveState.collectAsState()
    val metadata by viewModel.metadata.collectAsState()
    val tagSuggestions by viewModel.tagSuggestions.collectAsState()

    var showInfoSheet by remember { mutableStateOf(false) }
    var isEditorFocused by remember { mutableStateOf(false) }

    LaunchedEffect(noteId) {
        viewModel.loadNote(noteId)
    }

    Scaffold(
        topBar = {
            val successState = uiState as? NoteEditorUiState.Success
            TopAppBar(
                title = {
                    if (successState != null) {
                        TextField(
                            value = successState.note.title,
                            onValueChange = { viewModel.onTitleChanged(it) },
                            textStyle = LocalTextStyle.current.copy(
                                fontWeight = FontWeight.Bold,
                                fontSize = 18.sp
                            ),
                            colors = TextFieldDefaults.colors(
                                focusedContainerColor = androidx.compose.ui.graphics.Color.Transparent,
                                unfocusedContainerColor = androidx.compose.ui.graphics.Color.Transparent,
                                focusedIndicatorColor = androidx.compose.ui.graphics.Color.Transparent,
                                unfocusedIndicatorColor = androidx.compose.ui.graphics.Color.Transparent
                            ),
                            placeholder = { Text("Title...", style = TextStyle(fontWeight = FontWeight.Bold, fontSize = 18.sp)) },
                            modifier = Modifier.fillMaxWidth()
                        )
                    } else {
                        Text("Note Editor")
                    }
                },
                navigationIcon = {
                    IconButton(onClick = {
                        viewModel.saveNoteImmediately()
                        onBackClick()
                    }) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(onClick = { onNavigateToHistory(noteId) }) {
                        Icon(Icons.Default.History, contentDescription = "Version History")
                    }
                    IconButton(onClick = { showInfoSheet = true }) {
                        Icon(Icons.Default.Info, contentDescription = "Note Info")
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
                is NoteEditorUiState.Loading -> {
                    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        CircularProgressIndicator()
                    }
                }
                is NoteEditorUiState.Error -> {
                    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.padding(24.dp)) {
                            Icon(Icons.Default.Error, contentDescription = null, tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(48.dp))
                            Spacer(modifier = Modifier.height(16.dp))
                            Text(state.message, color = MaterialTheme.colorScheme.error)
                            Spacer(modifier = Modifier.height(16.dp))
                            Button(onClick = onBackClick) { Text("Go Back") }
                        }
                    }
                }
                is NoteEditorUiState.Success -> {
                    val note = state.note

                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .imePadding() // Automatically shifts up above keyboard
                    ) {
                        // Dynamic Save Status Bar
                        SaveStatusBar(
                            saveState = saveState,
                            wordCount = metadata?.word_count ?: 0
                        )

                        // Monospace full-width editor body
                        Box(
                            modifier = Modifier
                                .weight(1f)
                                .fillMaxWidth()
                                .padding(horizontal = 16.dp, vertical = 8.dp)
                        ) {
                            BasicTextField(
                                value = note.body,
                                onValueChange = { viewModel.onContentChanged(it) },
                                textStyle = TextStyle(
                                    fontFamily = FontFamily.Monospace,
                                    fontSize = 15.sp,
                                    lineHeight = 24.sp,
                                    color = MaterialTheme.colorScheme.onSurface
                                ),
                                modifier = Modifier
                                    .fillMaxSize()
                                    .verticalScroll(rememberScrollState())
                                    .onFocusChanged { isEditorFocused = it.isFocused }
                            )
                        }

                        // Tags Input Bar
                        TagInputBar(
                            tags = note.tags,
                            onTagsChanged = { viewModel.onTagsChanged(it) },
                            suggestions = tagSuggestions,
                            onPrefixChanged = { prefix -> viewModel.loadSuggestions(prefix) }
                        )

                        // Formatting Toolbar (Only show when editing body)
                        AnimatedVisibility(
                            visible = isEditorFocused,
                            enter = slideInVertically { it } + fadeIn(),
                            exit = slideOutVertically { it } + fadeOut()
                        ) {
                            FormattingToolbar(
                                onInsertText = { shortcutText ->
                                    // Append or insert formatting text
                                    viewModel.onContentChanged(note.body + shortcutText)
                                }
                            )
                        }
                    }

                    // Metadata Bottom Sheet
                    if (showInfoSheet && metadata != null) {
                        NoteInfoSheet(
                            metadata = metadata!!,
                            onDismissRequest = { showInfoSheet = false }
                        )
                    }
                }
            }
        }
    }
}

@Composable
fun SaveStatusBar(
    saveState: SaveState,
    wordCount: Int,
    modifier: Modifier = Modifier
) {
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f),
        modifier = modifier
            .fillMaxWidth()
            .height(28.dp)
    ) {
        Row(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = 16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Left: Auto-save status
            Row(verticalAlignment = Alignment.CenterVertically) {
                val statusText = when (saveState) {
                    is SaveState.Saved -> "Saved"
                    is SaveState.Unsaved -> "Unsaved Changes"
                    is SaveState.Saving -> "Saving..."
                    is SaveState.Error -> "Error saving note"
                }
                val icon = when (saveState) {
                    is SaveState.Saved -> Icons.Default.CloudDone
                    is SaveState.Unsaved -> Icons.Default.CloudQueue
                    is SaveState.Saving -> Icons.Default.Sync
                    is SaveState.Error -> Icons.Default.CloudOff
                }
                val color = when (saveState) {
                    is SaveState.Saved -> MaterialTheme.colorScheme.primary
                    is SaveState.Unsaved -> MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f)
                    is SaveState.Saving -> MaterialTheme.colorScheme.secondary
                    is SaveState.Error -> MaterialTheme.colorScheme.error
                }

                Icon(
                    imageVector = icon,
                    contentDescription = null,
                    tint = color,
                    modifier = Modifier.size(14.dp)
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = statusText,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.Bold,
                    color = color
                )
            }

            // Right: Word count
            Text(
                text = "$wordCount words",
                fontSize = 11.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f)
            )
        }
    }
}
