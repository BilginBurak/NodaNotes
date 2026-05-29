package com.bubi.nodanotes.ui.components

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.LocalOffer
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
fun TagInputBar(
    tags: List<String>,
    onTagsChanged: (List<String>) -> Unit,
    suggestions: List<String>,
    onPrefixChanged: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    var tagText by remember { mutableStateOf("") }

    Column(
        modifier = modifier
            .fillMaxWidth()
            .padding(8.dp)
    ) {
        // Suggestions Autocomplete row
        val filteredSuggestions = suggestions.filter { it !in tags }
        if (tagText.isNotEmpty() && filteredSuggestions.isNotEmpty()) {
            Text(
                text = "Suggestions:",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp)
            )
            LazyRow(
                contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                items(filteredSuggestions) { suggestion ->
                    SuggestionChip(
                        onClick = {
                            val updated = tags + suggestion
                            onTagsChanged(updated)
                            tagText = ""
                            onPrefixChanged("")
                        },
                        label = { Text(suggestion) }
                    )
                }
            }
            Spacer(modifier = Modifier.height(4.dp))
        }

        // Tags List + Input Field Row
        FlowRow(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp)
        ) {
            // Render existing tags as chips
            tags.forEach { tag ->
                InputChip(
                    selected = false,
                    onClick = {
                        // Remove tag on click
                        onTagsChanged(tags - tag)
                    },
                    label = { Text(tag) },
                    trailingIcon = {
                        Icon(
                            imageVector = Icons.Default.Close,
                            contentDescription = "Remove tag",
                            modifier = Modifier.size(12.dp)
                        )
                    }
                )
            }

            // Text field to add tags
            BasicTextFieldWithPlaceholder(
                value = tagText,
                onValueChange = {
                    tagText = it
                    onPrefixChanged(it)
                },
                placeholder = "Add tags...",
                keyboardOptions = KeyboardOptions.Default.copy(
                    imeAction = ImeAction.Done
                ),
                keyboardActions = KeyboardActions(
                    onDone = {
                        val clean = tagText.trim().lowercase()
                        if (clean.isNotEmpty() && clean !in tags) {
                            onTagsChanged(tags + clean)
                        }
                        tagText = ""
                        onPrefixChanged("")
                    }
                )
            )
        }
    }
}

@Composable
private fun BasicTextFieldWithPlaceholder(
    value: String,
    onValueChange: (String) -> Unit,
    placeholder: String,
    keyboardOptions: KeyboardOptions,
    keyboardActions: KeyboardActions,
    modifier: Modifier = Modifier
) {
    Box(
        contentAlignment = Alignment.CenterStart,
        modifier = modifier
            .widthIn(min = 100.dp)
            .height(32.dp)
            .padding(horizontal = 8.dp)
    ) {
        if (value.isEmpty()) {
            Text(
                text = placeholder,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f),
                fontSize = 14.sp
            )
        }
        androidx.compose.foundation.text.BasicTextField(
            value = value,
            onValueChange = onValueChange,
            singleLine = true,
            textStyle = LocalTextStyle.current.copy(
                color = MaterialTheme.colorScheme.onSurface,
                fontSize = 14.sp
            ),
            keyboardOptions = keyboardOptions,
            keyboardActions = keyboardActions,
            modifier = Modifier.fillMaxWidth()
        )
    }
}
