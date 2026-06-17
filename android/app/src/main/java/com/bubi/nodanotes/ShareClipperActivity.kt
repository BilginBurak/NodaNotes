package com.bubi.nodanotes

import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.net.Uri
import android.os.Bundle
import android.util.Patterns
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.lifecycleScope
import com.bubi.nodanotes.data.preferences.VaultPreferences
import com.bubi.nodanotes.ui.theme.NodaTheme
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

sealed interface ClipperUiState {
    object Loading : ClipperUiState
    data class Ready(
        val exists: Boolean,
        val noteId: String?,
        val url: String,
        val title: String,
        val contentMarkdown: String?
    ) : ClipperUiState
}

@Serializable
data class HistoryCheckResult(
    val exists: Boolean,
    val note_id: String? = null
)

@Serializable
data class ClipperPayload(
    val title: String,
    val url: String,
    val content_markdown: String?,
    val tags: List<String>,
    val append: Boolean?,
    val author: String? = null,
    val published_date: String? = null
)

@Serializable
data class ErrorDto(
    val error: String
)

@OptIn(ExperimentalMaterial3Api::class)
class ShareClipperActivity : ComponentActivity() {

    private lateinit var vaultPreferences: VaultPreferences

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        vaultPreferences = VaultPreferences(this)
        val savedVaultPath = vaultPreferences.getVaultPath()
        if (savedVaultPath == null) {
            Toast.makeText(this, "Please configure NodaNotes vault first", Toast.LENGTH_LONG).show()
            finish()
            return
        }

        try {
            RustCore.initVault(savedVaultPath)
        } catch (e: Exception) {
            Toast.makeText(this, "Failed to initialize vault", Toast.LENGTH_LONG).show()
            finish()
            return
        }

        val action = intent.action
        var isolatedUrl = ""
        var contentMarkdown: String? = null
        var pageTitle = ""

        if (action == Intent.ACTION_PROCESS_TEXT) {
            // Contextual text menu sharing selection
            val rawText = intent.getCharSequenceExtra(Intent.EXTRA_PROCESS_TEXT)?.toString()?.trim() ?: ""
            isolatedUrl = ""
            contentMarkdown = if (rawText.isNotEmpty()) rawText else null
            pageTitle = "Clipped Selection"
        } else {
            // Standard SEND share sheet
            val sharedText = intent.getStringExtra(Intent.EXTRA_TEXT)?.trim() ?: ""
            val sharedSubject = intent.getStringExtra(Intent.EXTRA_SUBJECT)?.trim() ?: ""
            val sharedTitle = intent.getStringExtra(Intent.EXTRA_TITLE)?.trim() ?: ""

            // Enforce Native Android URL Extractor using WEB_URL matcher
            val matcher = Patterns.WEB_URL.matcher(sharedText)
            if (matcher.find()) {
                isolatedUrl = matcher.group() ?: ""
            }

            val remainingText = if (isolatedUrl.isNotEmpty()) {
                sharedText.replace(isolatedUrl, "").trim()
            } else {
                sharedText
            }
            contentMarkdown = if (remainingText.isNotEmpty()) remainingText else null

            pageTitle = if (sharedSubject.isNotEmpty()) {
                sharedSubject
            } else if (sharedTitle.isNotEmpty()) {
                sharedTitle
            } else {
                if (isolatedUrl.isNotEmpty()) {
                    try {
                        val uri = Uri.parse(isolatedUrl)
                        val host = uri.host ?: ""
                        val lastPath = uri.lastPathSegment
                        if (!lastPath.isNullOrEmpty()) {
                            lastPath.replace("-", " ").replace("_", " ")
                                .split(" ")
                                .joinToString(" ") { it.replaceFirstChar { c -> c.uppercase() } }
                        } else {
                            host
                        }
                    } catch (e: Exception) {
                        "Clipped Webpage"
                    }
                } else {
                    "Clipped Note"
                }
            }
        }

        // Initialize state
        var uiState by mutableStateOf<ClipperUiState>(ClipperUiState.Loading)

        if (isolatedUrl.isEmpty()) {
            uiState = ClipperUiState.Ready(
                exists = false,
                noteId = null,
                url = "",
                title = pageTitle,
                contentMarkdown = contentMarkdown
            )
        } else {
            lifecycleScope.launch(Dispatchers.IO) {
                // Target JNI Handshake Safely: Pass ONLY the isolatedUrl variable
                val resultJson = try {
                    RustCore.check_url_history(isolatedUrl)
                } catch (e: Exception) {
                    "{\"exists\":false}"
                }
                val checkResult = try {
                    Json.decodeFromString<HistoryCheckResult>(resultJson)
                } catch (e: Exception) {
                    HistoryCheckResult(exists = false)
                }

                withContext(Dispatchers.Main) {
                    uiState = ClipperUiState.Ready(
                        exists = checkResult.exists,
                        noteId = checkResult.note_id,
                        url = isolatedUrl,
                        title = pageTitle,
                        contentMarkdown = contentMarkdown
                    )
                }
            }
        }

        setContent {
            val context = androidx.compose.ui.platform.LocalContext.current
            val sharedPreferences = remember { context.getSharedPreferences("noda_prefs", android.content.Context.MODE_PRIVATE) }
            var darkModePref by remember { mutableStateOf(sharedPreferences.getString("dark_mode", "system") ?: "system") }
            
            DisposableEffect(sharedPreferences) {
                val listener = SharedPreferences.OnSharedPreferenceChangeListener { prefs, key ->
                    if (key == "dark_mode") {
                        darkModePref = prefs.getString("dark_mode", "system") ?: "system"
                    }
                }
                sharedPreferences.registerOnSharedPreferenceChangeListener(listener)
                onDispose {
                    sharedPreferences.unregisterOnSharedPreferenceChangeListener(listener)
                }
            }

            val darkTheme = when (darkModePref) {
                "dark" -> true
                "light" -> false
                else -> androidx.compose.foundation.isSystemInDarkTheme()
            }

            NodaTheme(darkTheme = darkTheme) {
                var isSheetOpen by remember { mutableStateOf(true) }

                if (isSheetOpen) {
                    ModalBottomSheet(
                        onDismissRequest = {
                            isSheetOpen = false
                            finish()
                        },
                        containerColor = MaterialTheme.colorScheme.background,
                        contentColor = MaterialTheme.colorScheme.onBackground
                    ) {
                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(24.dp)
                                .navigationBarsPadding(),
                            contentAlignment = Alignment.Center
                        ) {
                            when (val state = uiState) {
                                ClipperUiState.Loading -> {
                                    CircularProgressIndicator(
                                        color = MaterialTheme.colorScheme.primary
                                    )
                                }
                                is ClipperUiState.Ready -> {
                                    ClipperScreen(
                                        state = state,
                                        onClip = { editedTitle, parsedTags, append ->
                                            performClip(state, editedTitle, parsedTags, append)
                                        }
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    private fun performClip(state: ClipperUiState.Ready, title: String, tags: List<String>, append: Boolean) {
        lifecycleScope.launch(Dispatchers.IO) {
            val payload = ClipperPayload(
                title = title,
                url = state.url,
                content_markdown = state.contentMarkdown,
                tags = tags,
                append = append
            )
            val jsonInput = Json.encodeToString(ClipperPayload.serializer(), payload)
            val resultJson = try {
                RustCore.clipUrl(jsonInput)
            } catch (e: Exception) {
                "{\"error\":\"${e.message}\"}"
            }

            withContext(Dispatchers.Main) {
                if (resultJson.contains("\"success\":true")) {
                    Toast.makeText(this@ShareClipperActivity, "Clipped to Noda!", Toast.LENGTH_SHORT).show()
                } else {
                    val errMsg = try {
                        val parsed = Json.decodeFromString<ErrorDto>(resultJson)
                        parsed.error
                    } catch (e: Exception) {
                        "Clip failed"
                    }
                    Toast.makeText(this@ShareClipperActivity, errMsg, Toast.LENGTH_LONG).show()
                }
                finish()
            }
        }
    }
}

@Composable
fun ClipperScreen(
    state: ClipperUiState.Ready,
    onClip: (title: String, tags: List<String>, append: Boolean) -> Unit
) {
    var editedTitle by remember { mutableStateOf(state.title) }
    var tagsInput by remember { mutableStateOf("") }
    var previewExpanded by remember { mutableStateOf(false) }

    val focusManager = LocalFocusManager.current
    val keyboardController = LocalSoftwareKeyboardController.current

    // Focus state tracking for conditional BackHandler
    var isTitleFocused by remember { mutableStateOf(false) }
    var isTagsFocused by remember { mutableStateOf(false) }
    val isAnyFieldFocused = isTitleFocused || isTagsFocused

    // BackHandler to intercept soft keyboard drop
    BackHandler(enabled = isAnyFieldFocused) {
        focusManager.clearFocus()
    }

    val previewMarkdown = remember(editedTitle, tagsInput, state.contentMarkdown, state.url) {
        buildString {
            if (state.contentMarkdown != null) {
                append(state.contentMarkdown)
                append("\n\n")
            } else {
                append("(Full webpage content will be fetched and converted to Markdown)\n\n")
            }
            append("<!-- noda-webclipper -->\n")
            append("—\n")
            append("*Added via NodaNotes #webclipper*\n\n")
            append("**Source:** [Link](${state.url})\n")
            append("**Author:** Unknown\n")
            append("**Published:** Unknown\n")
            
            val compiledTags = tagsInput.split(",")
                .map { it.trim() }
                .filter { it.isNotEmpty() }
                .joinToString(" ") { if (it.startsWith("#")) it else "#$it" }
            append("**Tags:** $compiledTags\n")
            append("**Added:** 2026-06-17")
        }
    }

    val parsedTagsList = remember(tagsInput) {
        tagsInput.split(",")
            .map { it.trim() }
            .filter { it.isNotEmpty() }
    }

    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text(
            text = "Web Clipper",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onBackground
        )

        OutlinedTextField(
            value = editedTitle,
            onValueChange = { editedTitle = it },
            label = { Text("Title") },
            modifier = Modifier
                .fillMaxWidth()
                .onFocusChanged { isTitleFocused = it.isFocused },
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                unfocusedBorderColor = MaterialTheme.colorScheme.outline
            )
        )

        OutlinedTextField(
            value = tagsInput,
            onValueChange = { tagsInput = it },
            label = { Text("Tags (comma-separated)") },
            placeholder = { Text("e.g. software, rust, local") },
            modifier = Modifier
                .fillMaxWidth()
                .onFocusChanged { isTagsFocused = it.isFocused },
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                unfocusedBorderColor = MaterialTheme.colorScheme.outline
            )
        )

        // Collapsible Markdown Preview
        Column(
            modifier = Modifier.fillMaxWidth(),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "Markdown Preview",
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.onBackground
                )
                TextButton(onClick = { previewExpanded = !previewExpanded }) {
                    Text(if (previewExpanded) "Hide" else "Show")
                }
            }
            
            if (previewExpanded) {
                Surface(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(140.dp),
                    shape = MaterialTheme.shapes.small,
                    color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.5f))
                ) {
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(12.dp)
                            .verticalScroll(rememberScrollState())
                    ) {
                        Text(
                            text = previewMarkdown,
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
            }
        }

        Spacer(modifier = Modifier.height(8.dp))

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            val buttonLabel = if (state.exists) "Append to Existing Note" else "Clip to NodaNotes"
            
            Button(
                onClick = { 
                    keyboardController?.hide()
                    focusManager.clearFocus()
                    onClip(editedTitle, parsedTagsList, state.exists) 
                },
                modifier = Modifier.weight(1f),
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.primary,
                    contentColor = MaterialTheme.colorScheme.onPrimary
                )
            ) {
                Text(
                    text = buttonLabel,
                    style = MaterialTheme.typography.labelLarge
                )
            }

            if (state.exists) {
                var expanded by remember { mutableStateOf(false) }
                Box {
                    IconButton(
                        onClick = { expanded = true }
                    ) {
                        Icon(
                            imageVector = Icons.Default.MoreVert,
                            contentDescription = "Options",
                            tint = MaterialTheme.colorScheme.onBackground
                        )
                    }
                    DropdownMenu(
                        expanded = expanded,
                        onDismissRequest = { expanded = false }
                    ) {
                        DropdownMenuItem(
                            text = { Text("Create New Note") },
                            onClick = {
                                expanded = false
                                keyboardController?.hide()
                                focusManager.clearFocus()
                                onClip(editedTitle, parsedTagsList, false)
                            }
                        )
                    }
                }
            }
        }
    }
}
