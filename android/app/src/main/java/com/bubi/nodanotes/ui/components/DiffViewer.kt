package com.bubi.nodanotes.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.bubi.nodanotes.data.model.DiffChunk

@Composable
fun DiffViewer(
    chunks: List<DiffChunk>,
    modifier: Modifier = Modifier
) {
    val scrollState = rememberScrollState()

    LazyColumn(
        modifier = modifier
            .fillMaxWidth()
            .horizontalScroll(scrollState)
    ) {
        items(chunks) { chunk ->
            DiffChunkRow(chunk = chunk)
        }
    }
}

@Composable
private fun DiffChunkRow(
    chunk: DiffChunk,
    modifier: Modifier = Modifier
) {
    val textStyle = MaterialTheme.typography.bodyMedium.copy(
        fontFamily = FontFamily.Monospace,
        fontSize = 13.sp,
        lineHeight = 18.sp
    )

    when (chunk.tag) {
        "Equal" -> {
            // Split by lines so it prints nicely
            val lines = chunk.text.split("\n")
            lines.forEach { line ->
                if (line.isNotEmpty()) {
                    Text(
                        text = "   $line",
                        style = textStyle,
                        color = MaterialTheme.colorScheme.onSurface,
                        modifier = modifier
                            .fillMaxWidth()
                            .padding(vertical = 1.dp, horizontal = 16.dp)
                    )
                }
            }
        }
        "Insert" -> {
            val lines = chunk.text.split("\n")
            lines.forEach { line ->
                if (line.isNotEmpty()) {
                    Text(
                        text = buildAnnotatedString {
                            withStyle(SpanStyle(color = MaterialTheme.colorScheme.primary)) {
                                append(" + ")
                            }
                            append(line)
                        },
                        style = textStyle,
                        color = MaterialTheme.colorScheme.onPrimaryContainer,
                        modifier = modifier
                            .fillMaxWidth()
                            .background(MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.25f))
                            .padding(vertical = 2.dp, horizontal = 16.dp)
                    )
                }
            }
        }
        "Delete" -> {
            val lines = chunk.text.split("\n")
            lines.forEach { line ->
                if (line.isNotEmpty()) {
                    Text(
                        text = buildAnnotatedString {
                            withStyle(SpanStyle(color = MaterialTheme.colorScheme.error)) {
                                append(" - ")
                            }
                            append(line)
                        },
                        style = textStyle,
                        color = MaterialTheme.colorScheme.onErrorContainer,
                        modifier = modifier
                            .fillMaxWidth()
                            .background(MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.25f))
                            .padding(vertical = 2.dp, horizontal = 16.dp)
                    )
                }
            }
        }
        "Separator" -> {
            // Skips context indicator
            Text(
                text = "  ⋯  (unchanged lines skipped)",
                style = textStyle,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f),
                modifier = modifier
                    .fillMaxWidth()
                    .padding(vertical = 6.dp, horizontal = 16.dp)
            )
        }
    }
}
