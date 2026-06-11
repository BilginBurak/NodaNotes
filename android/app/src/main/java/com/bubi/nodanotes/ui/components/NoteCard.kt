package com.bubi.nodanotes.ui.components

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PushPin
import androidx.compose.material.icons.filled.Folder
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.bubi.nodanotes.data.model.NoteListItemDto

@OptIn(ExperimentalFoundationApi::class)
@Composable
fun NoteCard(
    note: NoteListItemDto,
    onNoteClick: (String) -> Unit,
    onPinToggle: (NoteListItemDto) -> Unit,
    onDelete: (NoteListItemDto) -> Unit,
    modifier: Modifier = Modifier,
    isSelectionMode: Boolean = false,
    isSelected: Boolean = false,
    onToggleSelection: (NoteListItemDto) -> Unit = {}
) {
    val themeColor = MaterialTheme.colorScheme.primary
    val accentColor = remember(note.color) {
        parseHexColor(note.color, themeColor)
    }

    val cardPadding = PaddingValues(horizontal = 8.dp, vertical = 6.dp)

    // Custom Flat Box Container replacing boilerplate M3 Card
    Box(
        modifier = modifier
            .fillMaxWidth()
            .padding(cardPadding)
            .background(
                color = if (isSelected) {
                    MaterialTheme.colorScheme.primaryContainer
                } else {
                    MaterialTheme.colorScheme.surface
                },
                shape = RoundedCornerShape(8.dp) // Zen corners
            )
            .combinedClickable(
                onClick = {
                    if (isSelectionMode) {
                        onToggleSelection(note)
                    } else {
                        onNoteClick(note.id)
                    }
                },
                onLongClick = {
                    onToggleSelection(note)
                }
            )
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .height(IntrinsicSize.Min)
        ) {
            // Elegant vertical accent color bar (shows note custom color or primary if pinned)
            val showLeftBar = !note.color.isNullOrEmpty() || note.pinned
            if (showLeftBar) {
                Box(
                    modifier = Modifier
                        .fillMaxHeight()
                        .width(4.dp)
                        .background(
                            color = accentColor,
                            shape = RoundedCornerShape(topStart = 8.dp, bottomStart = 8.dp)
                        )
                )
            }

            Column(
                modifier = Modifier
                    .weight(1f)
                    .padding(horizontal = 16.dp, vertical = 14.dp)
            ) {
                // Folder hierarchy & Pinned row
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    val folderPath = note.file_path.substringBeforeLast('/', "")
                    if (folderPath.isNotEmpty()) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier
                                .background(
                                    color = MaterialTheme.colorScheme.surfaceVariant, // Solid color
                                    shape = RoundedCornerShape(4.dp)
                                )
                                .padding(horizontal = 6.dp, vertical = 2.dp)
                        ) {
                            Icon(
                                imageVector = Icons.Default.Folder,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.size(10.dp)
                            )
                            Spacer(modifier = Modifier.width(4.dp))
                            Text(
                                text = folderPath,
                                style = MaterialTheme.typography.labelSmall.copy(
                                    fontSize = 10.sp,
                                    fontWeight = FontWeight.Bold
                                ),
                                color = MaterialTheme.colorScheme.primary,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis
                            )
                        }
                    } else {
                        Spacer(modifier = Modifier.size(1.dp))
                    }

                    if (note.pinned) {
                        Icon(
                            imageVector = Icons.Default.PushPin,
                            contentDescription = "Pinned",
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(14.dp)
                        )
                    }
                }

                Spacer(modifier = Modifier.height(6.dp))

                // Title - utilizes editorial titleMedium scale
                Text(
                    text = note.title.ifEmpty { "Untitled" },
                    style = MaterialTheme.typography.titleMedium,
                    color = if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.fillMaxWidth()
                )

                Spacer(modifier = Modifier.height(10.dp))

                // Bottom row: Date & Tags
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = formatDate(note.updated_at),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant // Solid color
                    )

                    // Tags
                    if (note.tags.isNotEmpty()) {
                        Row(
                            horizontalArrangement = Arrangement.spacedBy(4.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            note.tags.take(2).forEach { tag ->
                                Surface(
                                    shape = RoundedCornerShape(4.dp),
                                    color = MaterialTheme.colorScheme.primaryContainer, // Solid color
                                    modifier = Modifier.height(18.dp)
                                ) {
                                    Box(
                                        contentAlignment = Alignment.Center,
                                        modifier = Modifier.padding(horizontal = 5.dp, vertical = 1.dp)
                                    ) {
                                        Text(
                                            text = "#$tag",
                                            style = MaterialTheme.typography.labelSmall.copy(
                                                fontSize = 9.sp,
                                                fontWeight = FontWeight.Bold
                                            ),
                                            color = MaterialTheme.colorScheme.onPrimaryContainer
                                        )
                                    }
                                }
                            }
                            if (note.tags.size > 2) {
                                Surface(
                                    shape = RoundedCornerShape(4.dp),
                                    color = MaterialTheme.colorScheme.surfaceVariant, // Solid color
                                    modifier = Modifier.height(18.dp)
                                ) {
                                    Box(
                                        contentAlignment = Alignment.Center,
                                        modifier = Modifier.padding(horizontal = 5.dp, vertical = 1.dp)
                                    ) {
                                        Text(
                                            text = "+${note.tags.size - 2}",
                                            style = MaterialTheme.typography.labelSmall.copy(
                                                fontSize = 9.sp,
                                                fontWeight = FontWeight.Bold
                                            ),
                                            color = MaterialTheme.colorScheme.onSurfaceVariant
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

/**
 * Parses a hex string to a Compose Color safely.
 */
private fun parseHexColor(hex: String?, fallback: Color): Color {
    if (hex.isNullOrEmpty()) return fallback
    return try {
        val clean = hex.removePrefix("#")
        val colorInt = when (clean.length) {
            6 -> android.graphics.Color.parseColor("#$clean")
            8 -> android.graphics.Color.parseColor("#$clean")
            else -> return fallback
        }
        Color(colorInt)
    } catch (e: Exception) {
        fallback
    }
}

/**
 * Formats dates into human-readable relative time or standard dates.
 */
private fun formatDate(dateStr: String): String {
    return try {
        val instant = java.time.Instant.parse(dateStr)
        val now = java.time.Instant.now()
        val duration = java.time.Duration.between(instant, now)
        val seconds = duration.seconds

        when {
            seconds < 60 -> "Just now"
            seconds < 3600 -> "${seconds / 60}m ago"
            seconds < 86400 -> "${seconds / 3600}h ago"
            seconds < 172800 -> "Yesterday"
            else -> {
                val dt = java.time.ZonedDateTime.ofInstant(instant, java.time.ZoneId.systemDefault())
                val formatter = java.time.format.DateTimeFormatter.ofPattern("MMM dd, yyyy")
                dt.format(formatter)
            }
        }
    } catch (e: Exception) {
        try {
            val clean = dateStr.substringBefore('T')
            val parts = clean.split('-')
            if (parts.size == 3) {
                val months = listOf(
                    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
                    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"
                )
                val year = parts[0]
                val monthIdx = parts[1].toIntOrNull()?.minus(1) ?: 0
                val day = parts[2].toIntOrNull() ?: 1
                "${months[monthIdx]} $day, $year"
            } else {
                dateStr
            }
        } catch (ex: Exception) {
            dateStr
        }
    }
}
