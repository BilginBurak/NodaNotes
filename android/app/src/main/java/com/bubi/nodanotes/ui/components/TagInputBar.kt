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
    yamlTags: List<String>,
    inlineTags: List<String>,
    onYamlTagsChanged: (List<String>) -> Unit,
    onInlineTagClick: (String) -> Unit,
    suggestions: List<String>,
    onPrefixChanged: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    var tagText by remember { mutableStateOf("") }

    Column(
        modifier = modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp, horizontal = 8.dp)
    ) {
        // Suggestions Autocomplete row
        val filteredSuggestions = suggestions.filter { it !in yamlTags && it !in inlineTags }
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
                            val updated = yamlTags + suggestion
                            onYamlTagsChanged(updated)
                            tagText = ""
                            onPrefixChanged("")
                        },
                        label = { Text(suggestion) }
                    )
                }
            }
            Spacer(modifier = Modifier.height(4.dp))
        }

        // Horizontal Tag Bar containing tags & input
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = Icons.Default.LocalOffer,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.primary, // Solid color
                modifier = Modifier.size(18.dp)
            )
            Spacer(modifier = Modifier.width(8.dp))

            LazyRow(
                modifier = Modifier.weight(1f),
                contentPadding = PaddingValues(end = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                // YAML tags with (X) delete button, solid borders/darker backgrounds
                items(yamlTags) { tag ->
                    InputChip(
                        selected = false,
                        onClick = { onYamlTagsChanged(yamlTags - tag) },
                        label = { Text(tag) },
                        border = InputChipDefaults.inputChipBorder(
                            enabled = true,
                            selected = false,
                            borderColor = MaterialTheme.colorScheme.primary, // Solid color
                            borderWidth = 1.dp
                        ),
                        colors = InputChipDefaults.inputChipColors(
                            containerColor = MaterialTheme.colorScheme.primaryContainer // Solid color
                        ),
                        trailingIcon = {
                            Icon(
                                imageVector = Icons.Default.Close,
                                contentDescription = "Remove tag",
                                modifier = Modifier.size(14.dp)
                            )
                        }
                    )
                }

                // Inline tags, soft border styling, no X button, clicking highlights tag in editor
                items(inlineTags) { tag ->
                    InputChip(
                        selected = false,
                        onClick = { onInlineTagClick(tag) },
                        label = { Text(tag) },
                        border = InputChipDefaults.inputChipBorder(
                            enabled = true,
                            selected = false,
                            borderColor = MaterialTheme.colorScheme.primary, // Solid color
                            borderWidth = 1.dp
                        ),
                        colors = InputChipDefaults.inputChipColors(
                            containerColor = MaterialTheme.colorScheme.surfaceVariant // Solid color
                        )
                    )
                }

                item {
                    BasicTextFieldWithPlaceholder(
                        value = tagText,
                        onValueChange = { newValue ->
                            if (newValue.endsWith(" ") || newValue.endsWith(",")) {
                                val newTag = newValue.dropLast(1).trim().lowercase()
                                if (newTag.isNotEmpty() && newTag !in yamlTags) {
                                    onYamlTagsChanged(yamlTags + newTag)
                                }
                                tagText = ""
                                onPrefixChanged("")
                            } else {
                                tagText = newValue
                                onPrefixChanged(newValue)
                            }
                        },
                        placeholder = "Add tags...",
                        keyboardOptions = KeyboardOptions.Default.copy(
                            imeAction = ImeAction.Done
                        ),
                        keyboardActions = KeyboardActions(
                            onDone = {
                                val clean = tagText.trim().lowercase()
                                if (clean.isNotEmpty() && clean !in yamlTags) {
                                    onYamlTagsChanged(yamlTags + clean)
                                }
                                tagText = ""
                                onPrefixChanged("")
                            }
                        )
                    )
                }
            }
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
                color = MaterialTheme.colorScheme.onSurfaceVariant, // Solid color
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
            cursorBrush = androidx.compose.ui.graphics.SolidColor(MaterialTheme.colorScheme.primary),
            keyboardOptions = keyboardOptions,
            keyboardActions = keyboardActions,
            modifier = Modifier.fillMaxWidth()
        )
    }
}
