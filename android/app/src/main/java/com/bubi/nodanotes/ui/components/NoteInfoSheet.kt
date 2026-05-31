package com.bubi.nodanotes.ui.components

import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Info
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.bubi.nodanotes.data.model.NoteMetadataDto

import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NoteInfoSheet(
    metadata: NoteMetadataDto,
    onDismissRequest: () -> Unit,
    modifier: Modifier = Modifier
) {
    ModalBottomSheet(
        onDismissRequest = onDismissRequest,
        modifier = modifier
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(24.dp)
                .navigationBarsPadding()
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.padding(bottom = 16.dp)
            ) {

                Text(
                    text = "Note Information",
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold
                )
            }

            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
            Spacer(modifier = Modifier.height(16.dp))

            InfoRow(label = "Title", value = metadata.title.ifEmpty { "Untitled" })
            InfoRow(label = "Filename", value = metadata.file_name)
            InfoRow(label = "Folder Path", value = metadata.relative_path.ifEmpty { "Root Vault" })
            InfoRow(label = "Size", value = formatBytes(metadata.file_size_bytes))
            InfoRow(label = "Word Count", value = "${metadata.word_count} words")
            InfoRow(label = "Character Count", value = "${metadata.char_count} characters")
            InfoRow(label = "Snapshots Saved", value = "${metadata.history_count} versions")
            InfoRow(
                label = "Last Sync Upload",
                value = metadata.last_upload_time?.let { formatDate(it) } ?: "Never"
            )
            
            InfoRow(label = "Created At", value = formatDate(metadata.created_at))
            InfoRow(label = "Modified At", value = formatDate(metadata.updated_at))

            Spacer(modifier = Modifier.height(16.dp))
        }
    }
}

@Composable
private fun InfoRow(label: String, value: String) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 6.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurface,
            maxLines = 1
        )
    }
}

private val dateTimeFormatter = DateTimeFormatter.ofPattern("MMM dd, yyyy 'at' HH:mm", Locale.ENGLISH)

private fun formatBytes(bytes: Long): String {
    if (bytes < 1024) return "$bytes B"
    val exp = (Math.log(bytes.toDouble()) / Math.log(1024.0)).toInt()
    val pre = "KMGTPE"[exp - 1] + "i"
    return String.format("%.1f %sB", bytes / Math.pow(1024.0, exp.toDouble()), pre)
}

private fun formatDate(dateStr: String): String {
    return try {
        val instant = Instant.parse(dateStr)
        val zonedDateTime = instant.atZone(ZoneId.systemDefault())
        dateTimeFormatter.format(zonedDateTime)
    } catch (e: Exception) {
        try {
            val clean = dateStr.replace("T", " ").substringBefore('.')
            clean
        } catch (ex: Exception) {
            dateStr
        }
    }
}
