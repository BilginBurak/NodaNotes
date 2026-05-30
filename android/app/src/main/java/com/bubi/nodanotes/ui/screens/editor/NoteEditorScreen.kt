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
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.ui.graphics.toArgb
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.foundation.background
import androidx.compose.foundation.shape.RoundedCornerShape
import android.webkit.WebView
import android.webkit.WebViewClient
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
    var showRecentAttachmentsSheet by remember { mutableStateOf(false) }

    var showInfoSheet by remember { mutableStateOf(false) }
    var isEditorFocused by remember { mutableStateOf(false) }

    // Media and File Picker for attachments
    val context = androidx.compose.ui.platform.LocalContext.current
    val filePickerLauncher = androidx.activity.compose.rememberLauncherForActivityResult(
        contract = androidx.activity.result.contract.ActivityResultContracts.GetContent()
    ) { uri: android.net.Uri? ->
        uri?.let {
            // Helper to get absolute path from Uri (usually we copy content to a temporary cache file first)
            try {
                val inputStream = context.contentResolver.openInputStream(it)
                val tempFile = java.io.File.createTempFile("noda_attach_", "_" + (it.lastPathSegment ?: "file"), context.cacheDir)
                tempFile.deleteOnExit()
                tempFile.outputStream().use { output ->
                    inputStream?.copyTo(output)
                }
                
                // Add attachment to Rust
                viewModel.addAttachment(tempFile.absolutePath) { markdownLink ->
                    val successState = uiState as? NoteEditorUiState.Success
                    if (successState != null) {
                        viewModel.onContentChanged(successState.note.body + "\n" + markdownLink)
                    }
                }
            } catch (e: Exception) {
                // handle error or show error state via viewModel
            }
        }
    }

    val isReaderMode by viewModel.isReaderMode.collectAsState()

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
                    IconButton(onClick = { viewModel.toggleReaderMode() }) {
                        Icon(
                            imageVector = if (isReaderMode) Icons.Default.EditNote else Icons.Default.MenuBook,
                            contentDescription = if (isReaderMode) "Editor Mode" else "Reader Mode"
                        )
                    }
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
                            if (isReaderMode) {
                                val vaultPath = viewModel.vaultPath
                                val primaryColor = MaterialTheme.colorScheme.primary
                                val backgroundColor = MaterialTheme.colorScheme.surface
                                val textColor = MaterialTheme.colorScheme.onSurface
                                val linkColor = MaterialTheme.colorScheme.secondary
                                val codeBgColor = MaterialTheme.colorScheme.surfaceVariant
                                val codeTextColor = MaterialTheme.colorScheme.onSurfaceVariant
                                val dividerColor = MaterialTheme.colorScheme.outlineVariant

                                val primaryHex = String.format("#%06X", 0xFFFFFF and primaryColor.toArgb())
                                val bgHex = String.format("#%06X", 0xFFFFFF and backgroundColor.toArgb())
                                val textHex = String.format("#%06X", 0xFFFFFF and textColor.toArgb())
                                val linkHex = String.format("#%06X", 0xFFFFFF and linkColor.toArgb())
                                val codeBgHex = String.format("#%06X", 0xFFFFFF and codeBgColor.toArgb())
                                val codeTextHex = String.format("#%06X", 0xFFFFFF and codeTextColor.toArgb())
                                val dividerHex = String.format("#%06X", 0xFFFFFF and dividerColor.toArgb())

                                AndroidView(
                                    factory = { ctx ->
                                        WebView(ctx).apply {
                                            webViewClient = WebViewClient()
                                            settings.javaScriptEnabled = false
                                            settings.allowFileAccess = true
                                            settings.allowContentAccess = true
                                        }
                                    },
                                    update = { webView ->
                                        val parsedHtml = parseMarkdownToHtml(note.body, vaultPath)
                                        val html = """
                                            <html>
                                            <head>
                                            <style>
                                                body {
                                                    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
                                                    padding: 16px;
                                                    line-height: 1.6;
                                                    color: $textHex;
                                                    background-color: $bgHex;
                                                }
                                                h1, h2, h3, h4, h5, h6 {
                                                    color: $primaryHex;
                                                    margin-top: 24px;
                                                    margin-bottom: 12px;
                                                    font-weight: 600;
                                                }
                                                h1 { font-size: 1.6em; border-bottom: 1px solid $dividerHex; padding-bottom: 8px; }
                                                h2 { font-size: 1.4em; }
                                                h3 { font-size: 1.2em; }
                                                a {
                                                    color: $linkHex;
                                                    text-decoration: none;
                                                }
                                                a:hover {
                                                    text-decoration: underline;
                                                }
                                                pre {
                                                    background-color: $codeBgHex;
                                                    color: $codeTextHex;
                                                    padding: 12px;
                                                    border-radius: 8px;
                                                    overflow-x: auto;
                                                    font-family: monospace;
                                                    font-size: 0.9em;
                                                    border: 1px solid $dividerHex;
                                                }
                                                code {
                                                    background-color: $codeBgHex;
                                                    color: $codeTextHex;
                                                    padding: 2px 6px;
                                                    border-radius: 4px;
                                                    font-family: monospace;
                                                    font-size: 0.9em;
                                                }
                                                pre code {
                                                    padding: 0;
                                                    background-color: transparent;
                                                    color: inherit;
                                                    border-radius: 0;
                                                }
                                                blockquote {
                                                    border-left: 4px solid $primaryHex;
                                                    margin: 16px 0;
                                                    padding-left: 16px;
                                                    color: $codeTextHex;
                                                    font-style: italic;
                                                }
                                                hr {
                                                    border: 0;
                                                    border-top: 1px solid $dividerHex;
                                                    margin: 24px 0;
                                                }
                                                ul, ol {
                                                    padding-left: 24px;
                                                    margin-bottom: 16px;
                                                }
                                                li {
                                                    margin-bottom: 6px;
                                                }
                                                input[type="checkbox"] {
                                                    margin-right: 8px;
                                                    transform: scale(1.1);
                                                    vertical-align: middle;
                                                }
                                                img {
                                                    max-width: 100%;
                                                    height: auto;
                                                    border-radius: 12px;
                                                    margin: 16px 0;
                                                    box-shadow: 0 4px 12px rgba(0,0,0,0.15);
                                                    display: block;
                                                }
                                            </style>
                                            </head>
                                            <body>
                                                $parsedHtml
                                            </body>
                                            </html>
                                        """.trimIndent()
                                        webView.loadDataWithBaseURL(null, html, "text/html", "utf-8", null)
                                    },
                                    modifier = Modifier.fillMaxSize()
                                )
                            } else {
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
                                },
                                onAttachmentClick = {
                                    filePickerLauncher.launch("*/*")
                                },
                                onQuickAttachmentClick = {
                                    showRecentAttachmentsSheet = true
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

                    // Recent Attachments Bottom Sheet
                    if (showRecentAttachmentsSheet) {
                        val recentAttachments by viewModel.recentAttachments.collectAsState()
                        val vaultPath = viewModel.vaultPath
                        
                        LaunchedEffect(Unit) {
                            viewModel.loadRecentAttachments()
                        }

                        ModalBottomSheet(
                            onDismissRequest = { showRecentAttachmentsSheet = false }
                        ) {
                            Column(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(16.dp)
                            ) {
                                Text(
                                    text = "Recent Attachments",
                                    style = MaterialTheme.typography.titleMedium,
                                    fontWeight = FontWeight.Bold,
                                    color = MaterialTheme.colorScheme.primary,
                                    modifier = Modifier.padding(bottom = 16.dp)
                                )

                                if (recentAttachments.isEmpty()) {
                                    Box(
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .height(150.dp),
                                        contentAlignment = Alignment.Center
                                    ) {
                                        Text(
                                            text = "No attachments found.",
                                            color = MaterialTheme.colorScheme.onSurfaceVariant
                                        )
                                    }
                                } else {
                                    LazyVerticalGrid(
                                        columns = GridCells.Fixed(2),
                                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                                        verticalArrangement = Arrangement.spacedBy(8.dp),
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .heightIn(max = 400.dp)
                                    ) {
                                        items(recentAttachments) { attachment ->
                                            val isImage = attachment.mime_type.startsWith("image/")
                                            val filePath = "$vaultPath/.noda/attachments/${attachment.name}"
                                            
                                            Card(
                                                onClick = {
                                                    val link = if (isImage) "![${attachment.name}](noda://attachments/${attachment.name})" else "[${attachment.name}](noda://attachments/${attachment.name})"
                                                    viewModel.onContentChanged(note.body + "\n" + link)
                                                    showRecentAttachmentsSheet = false
                                                },
                                                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)),
                                                modifier = Modifier.fillMaxWidth()
                                            ) {
                                                Row(
                                                    modifier = Modifier.padding(8.dp),
                                                    verticalAlignment = Alignment.CenterVertically
                                                ) {
                                                    Box(
                                                        modifier = Modifier
                                                            .size(48.dp)
                                                            .background(
                                                                color = MaterialTheme.colorScheme.surfaceVariant,
                                                                shape = RoundedCornerShape(6.dp)
                                                            ),
                                                        contentAlignment = Alignment.Center
                                                    ) {
                                                        if (isImage && vaultPath != null) {
                                                            val file = java.io.File(filePath)
                                                            if (file.exists()) {
                                                                val bitmap = remember(filePath) {
                                                                    android.graphics.BitmapFactory.decodeFile(filePath)?.let { bmp ->
                                                                        android.graphics.Bitmap.createScaledBitmap(bmp, 96, 96, true)
                                                                    }
                                                                }
                                                                if (bitmap != null) {
                                                                    androidx.compose.foundation.Image(
                                                                        bitmap = bitmap.asImageBitmap(),
                                                                        contentDescription = null,
                                                                        modifier = Modifier.fillMaxSize(),
                                                                        contentScale = androidx.compose.ui.layout.ContentScale.Crop
                                                                    )
                                                                } else {
                                                                    Icon(Icons.Default.Image, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                                                                }
                                                            } else {
                                                                Icon(Icons.Default.Image, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                                                            }
                                                        } else {
                                                            Icon(
                                                                imageVector = when {
                                                                    attachment.mime_type.startsWith("text/") -> Icons.Default.Article
                                                                    attachment.mime_type.startsWith("video/") -> Icons.Default.VideoFile
                                                                    attachment.mime_type.startsWith("audio/") -> Icons.Default.AudioFile
                                                                    else -> Icons.Default.InsertDriveFile
                                                                },
                                                                contentDescription = null,
                                                                tint = MaterialTheme.colorScheme.primary
                                                            )
                                                        }
                                                    }
                                                    Spacer(modifier = Modifier.width(8.dp))
                                                    Column(modifier = Modifier.weight(1f)) {
                                                        Text(
                                                            text = attachment.name.substringAfter('_'),
                                                            style = MaterialTheme.typography.bodySmall,
                                                            fontWeight = FontWeight.Medium,
                                                            maxLines = 1,
                                                            overflow = TextOverflow.Ellipsis
                                                        )
                                                        Text(
                                                            text = "${attachment.size_bytes / 1024} KB",
                                                            style = MaterialTheme.typography.labelSmall,
                                                            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f)
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

fun parseMarkdownToHtml(markdown: String, vaultPath: String?): String {
    val lines = markdown.lines()
    val htmlBuilder = StringBuilder()
    
    var inList = false
    var listType: String? = null // "ul" or "ol"
    var inCodeBlock = false
    var codeBlockLang: String? = null
    val codeBlockContent = StringBuilder()

    for (line in lines) {
        val trimmed = line.trim()
        
        // Handle Code Blocks
        if (trimmed.startsWith("```")) {
            if (inCodeBlock) {
                // Close code block
                htmlBuilder.append("<pre><code")
                if (codeBlockLang != null) {
                    htmlBuilder.append(" class=\"language-${codeBlockLang}\"")
                }
                htmlBuilder.append(">")
                // Escape HTML inside code block
                val escapedCode = codeBlockContent.toString()
                    .replace("&", "&amp;")
                    .replace("<", "&lt;")
                    .replace(">", "&gt;")
                htmlBuilder.append(escapedCode)
                htmlBuilder.append("</code></pre>\n")
                
                inCodeBlock = false
                codeBlockContent.clear()
            } else {
                inCodeBlock = true
                codeBlockLang = trimmed.substring(3).trim().ifEmpty { null }
            }
            continue
        }
        
        if (inCodeBlock) {
            codeBlockContent.append(line).append("\n")
            continue
        }
        
        // Close list if line is empty or doesn't start with list item
        val isListItem = trimmed.startsWith("- ") || trimmed.startsWith("* ") || 
                         trimmed.startsWith("1. ") || Regex("^\\d+\\.\\s.*").matches(trimmed)
        if (inList && (!isListItem || line.isEmpty())) {
            htmlBuilder.append("</").append(listType).append(">\n")
            inList = false
            listType = null
        }
        
        if (line.isEmpty()) {
            htmlBuilder.append("<br/>\n")
            continue
        }
        
        // Horizontal Rules
        if (trimmed == "---" || trimmed == "***" || trimmed == "___") {
            htmlBuilder.append("<hr/>\n")
            continue
        }
        
        // Headings
        if (trimmed.startsWith("#")) {
            val level = trimmed.takeWhile { it == '#' }.length
            if (level in 1..6) {
                val content = trimmed.substring(level).trim()
                htmlBuilder.append("<h").append(level).append(">")
                    .append(parseInlineMarkdown(content, vaultPath))
                    .append("</h").append(level).append(">\n")
                continue
            }
        }
        
        // Blockquotes
        if (trimmed.startsWith(">")) {
            val content = trimmed.substring(1).trim()
            htmlBuilder.append("<blockquote>")
                .append(parseInlineMarkdown(content, vaultPath))
                .append("</blockquote>\n")
            continue
        }
        
        // Task / Checklist Items & Unordered lists
        if (trimmed.startsWith("- [ ] ") || trimmed.startsWith("- [x] ") || trimmed.startsWith("- [X] ") ||
            trimmed.startsWith("* [ ] ") || trimmed.startsWith("* [x] ") || trimmed.startsWith("* [X] ")) {
            if (!inList) {
                inList = true
                listType = "ul"
                htmlBuilder.append("<ul>\n")
            }
            val checked = trimmed.contains("[x]") || trimmed.contains("[X]")
            val content = trimmed.substring(6).trim()
            htmlBuilder.append("<li style=\"list-style:none;\">")
                .append("<input type=\"checkbox\" disabled ").append(if (checked) "checked" else "").append("/>")
                .append(parseInlineMarkdown(content, vaultPath))
                .append("</li>\n")
            continue
        }
        
        if (trimmed.startsWith("- ") || trimmed.startsWith("* ")) {
            if (!inList) {
                inList = true
                listType = "ul"
                htmlBuilder.append("<ul>\n")
            }
            val content = trimmed.substring(2).trim()
            htmlBuilder.append("<li>")
                .append(parseInlineMarkdown(content, vaultPath))
                .append("</li>\n")
            continue
        }
        
        // Ordered Lists
        val orderedMatch = Regex("^(\\d+)\\.\\s(.*)").find(trimmed)
        if (orderedMatch != null) {
            if (!inList) {
                inList = true
                listType = "ol"
                htmlBuilder.append("<ol>\n")
            }
            val content = orderedMatch.groupValues[2].trim()
            htmlBuilder.append("<li>")
                .append(parseInlineMarkdown(content, vaultPath))
                .append("</li>\n")
            continue
        }
        
        // Regular Paragraph
        htmlBuilder.append("<p>")
            .append(parseInlineMarkdown(line, vaultPath))
            .append("</p>\n")
    }
    
    // Close any open list at EOF
    if (inList) {
        htmlBuilder.append("</").append(listType).append(">\n")
    }
    
    return htmlBuilder.toString()
}

fun parseInlineMarkdown(text: String, vaultPath: String?): String {
    var result = text
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
    
    // Parse Image Attachments: ![alt](noda://attachments/{name}) or general images
    val imageRegex = Regex("!\\[(.*?)\\]\\((.*?)\\)")
    result = imageRegex.replace(result) { matchResult ->
        val alt = matchResult.groupValues[1]
        var url = matchResult.groupValues[2]
        if (url.startsWith("noda://attachments/") && vaultPath != null) {
            val name = url.substringAfter("noda://attachments/")
            url = "file://$vaultPath/.noda/attachments/$name"
        }
        "<img src=\"$url\" alt=\"$alt\" />"
    }

    // Parse normal links: [text](url)
    val linkRegex = Regex("\\[(.*?)\\]\\((.*?)\\)")
    result = linkRegex.replace(result) { matchResult ->
        val linkText = matchResult.groupValues[1]
        var url = matchResult.groupValues[2]
        if (url.startsWith("noda://attachments/") && vaultPath != null) {
            val name = url.substringAfter("noda://attachments/")
            url = "file://$vaultPath/.noda/attachments/$name"
        }
        "<a href=\"$url\" target=\"_blank\">$linkText</a>"
    }

    // Bold `**text**` or `__text__`
    result = result.replace(Regex("\\*\\*(.*?)\\*\\*"), "<strong>$1</strong>")
    result = result.replace(Regex("__(.*?)__"), "<strong>$1</strong>")

    // Italic `*text*` or `_text_`
    result = result.replace(Regex("\\*(.*?)\\*"), "<em>$1</em>")
    result = result.replace(Regex("_(.*?)_"), "<em>$1</em>")

    // Inline Code `` `code` ``
    result = result.replace(Regex("`(.*?)`"), "<code>$1</code>")

    return result
}
