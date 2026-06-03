package com.bubi.nodanotes.ui.components

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable
fun FormattingToolbar(
    onInsertText: (String, String) -> Unit,
    onAttachmentClick: () -> Unit,
    onQuickAttachmentClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.85f),
        tonalElevation = 4.dp,
        modifier = modifier.fillMaxWidth()
    ) {
        LazyRow(
            contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(4.dp),
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.fillMaxWidth()
        ) {
            item {
                ToolbarButton(icon = Icons.Default.FormatBold, label = "Bold", onClick = { onInsertText("**", "**") })
            }
            item {
                ToolbarButton(icon = Icons.Default.FormatItalic, label = "Italic", onClick = { onInsertText("*", "*") })
            }
            item {
                ToolbarButton(icon = Icons.Default.Title, label = "Heading 1", onClick = { onInsertText("# ", "") })
            }
            item {
                TextButton(
                    onClick = { onInsertText("## ", "") },
                    modifier = Modifier.height(40.dp)
                ) {
                    Text("H2", fontWeight = FontWeight.Bold, fontSize = 14.sp)
                }
            }
            item {
                TextButton(
                    onClick = { onInsertText("### ", "") },
                    modifier = Modifier.height(40.dp)
                ) {
                    Text("H3", fontWeight = FontWeight.Bold, fontSize = 14.sp)
                }
            }
            item {
                ToolbarButton(icon = Icons.Default.FormatListBulleted, label = "List", onClick = { onInsertText("- ", "") })
            }
            item {
                ToolbarButton(icon = Icons.Default.CheckCircleOutline, label = "Checklist", onClick = { onInsertText("- [ ] ", "") })
            }
            item {
                ToolbarButton(icon = Icons.Default.FormatListNumbered, label = "Numbered List", onClick = { onInsertText("1. ", "") })
            }
            item {
                ToolbarButton(icon = Icons.Default.Code, label = "Code Block", onClick = { onInsertText("```\n", "\n```") })
            }
            item {
                ToolbarButton(icon = Icons.Default.FormatQuote, label = "Quote", onClick = { onInsertText("> ", "") })
            }
            item {
                ToolbarButton(icon = Icons.Default.Link, label = "Link", onClick = { onInsertText("[", "](url)") })
            }
            item {
                ToolbarButton(icon = Icons.Default.Attachment, label = "Attachment", onClick = onAttachmentClick)
            }
            item {
                ToolbarButton(icon = Icons.Default.PhotoLibrary, label = "Recent Attachments", onClick = onQuickAttachmentClick)
            }
        }
    }
}

@Composable
private fun ToolbarButton(
    icon: ImageVector,
    label: String,
    onClick: () -> Unit
) {
    IconButton(
        onClick = onClick,
        modifier = Modifier.size(40.dp)
    ) {
        Icon(
            imageVector = icon,
            contentDescription = label,
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.size(20.dp)
        )
    }
}
