package com.bubi.nodanotes.ui.screens.attachments

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.bubi.nodanotes.data.model.AttachmentInfoDto
import com.bubi.nodanotes.ui.components.FilePreviewDialog
import java.io.File

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AttachmentsScreen(
    onBackClick: () -> Unit,
    viewModel: AttachmentsViewModel = viewModel()
) {
    val uiState by viewModel.uiState.collectAsState()
    val vaultPath = viewModel.vaultPath

    var previewAttachment by remember { mutableStateOf<AttachmentInfoDto?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Attachments Gallery", fontWeight = FontWeight.Bold) },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { innerPadding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            when (val state = uiState) {
                is AttachmentsUiState.Loading -> {
                    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        CircularProgressIndicator()
                    }
                }
                is AttachmentsUiState.Error -> {
                    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        Column(
                            horizontalAlignment = Alignment.CenterHorizontally,
                            modifier = Modifier.padding(24.dp)
                        ) {
                            Icon(Icons.Default.Error, contentDescription = null, tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(48.dp))
                            Spacer(modifier = Modifier.height(16.dp))
                            Text(state.message, color = MaterialTheme.colorScheme.error)
                            Spacer(modifier = Modifier.height(16.dp))
                            Button(onClick = { viewModel.loadAttachments() }) {
                                Text("Retry")
                            }
                        }
                    }
                }
                is AttachmentsUiState.Success -> {
                    val list = state.attachments
                    if (list.isEmpty()) {
                        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                                Icon(Icons.Default.FolderOpen, contentDescription = null, modifier = Modifier.size(80.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.4f))
                                Spacer(modifier = Modifier.height(16.dp))
                                Text("No attachments inside the vault.", color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                        }
                    } else {
                        LazyVerticalGrid(
                            columns = GridCells.Adaptive(minSize = 140.dp),
                            contentPadding = PaddingValues(16.dp),
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                            verticalArrangement = Arrangement.spacedBy(12.dp),
                            modifier = Modifier.fillMaxSize()
                        ) {
                            items(list) { attachment ->
                                val isImage = attachment.mime_type.startsWith("image/")
                                val filePath = "$vaultPath/.noda/attachments/${attachment.name}"

                                Card(
                                    onClick = { previewAttachment = attachment },
                                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f)),
                                    modifier = Modifier.fillMaxWidth()
                                ) {
                                    Column {
                                        Box(
                                            modifier = Modifier
                                                .fillMaxWidth()
                                                .height(120.dp)
                                                .background(
                                                    color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)
                                                ),
                                            contentAlignment = Alignment.Center
                                        ) {
                                            if (isImage && vaultPath != null) {
                                                val file = File(filePath)
                                                if (file.exists()) {
                                                    val bitmap = remember(filePath) {
                                                        android.graphics.BitmapFactory.decodeFile(filePath)?.let { bmp ->
                                                            android.graphics.Bitmap.createScaledBitmap(bmp, 200, 200, true)
                                                        }
                                                    }
                                                    if (bitmap != null) {
                                                        Image(
                                                            bitmap = bitmap.asImageBitmap(),
                                                            contentDescription = null,
                                                            modifier = Modifier.fillMaxSize(),
                                                            contentScale = ContentScale.Crop
                                                        )
                                                    } else {
                                                        Icon(Icons.Default.Image, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(36.dp))
                                                    }
                                                } else {
                                                    Icon(Icons.Default.Image, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(36.dp))
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
                                                    tint = MaterialTheme.colorScheme.primary,
                                                    modifier = Modifier.size(36.dp)
                                                )
                                            }
                                        }
                                        Column(modifier = Modifier.padding(8.dp)) {
                                            Text(
                                                text = attachment.name.substringAfter('_'),
                                                style = MaterialTheme.typography.bodySmall,
                                                fontWeight = FontWeight.Bold,
                                                maxLines = 1,
                                                overflow = TextOverflow.Ellipsis
                                            )
                                            Spacer(modifier = Modifier.height(2.dp))
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

            // Preview Dialog Overlay
            previewAttachment?.let { attachment ->
                val filePath = "$vaultPath/.noda/attachments/${attachment.name}"
                val file = File(filePath)
                val content = if (file.exists() && !attachment.mime_type.startsWith("image/")) {
                    try {
                        file.readText()
                    } catch (e: Exception) {
                        "Binary or unreadable file content."
                    }
                } else {
                    ""
                }

                FilePreviewDialog(
                    title = attachment.name.substringAfter('_'),
                    content = content,
                    filePath = filePath,
                    mimeType = attachment.mime_type,
                    onDismiss = { previewAttachment = null }
                )
            }
        }
    }
}
