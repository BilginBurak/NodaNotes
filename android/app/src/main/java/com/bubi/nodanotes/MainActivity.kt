package com.bubi.nodanotes

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import com.bubi.nodanotes.ui.theme.NodaNotesTheme

import android.content.Intent
import android.net.Uri
import android.os.Environment
import android.provider.Settings
import java.io.File

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        
        // Request "All Files Access" if not already granted (required for ~/Documents/NodaVault)
        if (!Environment.isExternalStorageManager()) {
            try {
                val intent = Intent(Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION).apply {
                    data = Uri.parse("package:${packageName}")
                }
                startActivity(intent)
            } catch (e: Exception) {
                val intent = Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION)
                startActivity(intent)
            }
        }

        // Target path: /storage/emulated/0/Documents/NodaVault (corresponds to ~/Documents/NodaVault)
        val vaultDir = File(Environment.getExternalStorageDirectory(), "Documents/NodaVault")
        if (Environment.isExternalStorageManager() && !vaultDir.exists()) {
            vaultDir.mkdirs()
        }

        // Initialize the vault using our Rust Core bridge
        val initResult = try {
            if (Environment.isExternalStorageManager()) {
                RustCore.initVault(vaultDir.absolutePath)
            } else {
                "Storage Access Required: Please grant 'All Files Access' in the settings screen and restart the app."
            }
        } catch (e: Throwable) {
            "JNI Error: ${e.message}"
        }

        setContent {
            NodaNotesTheme {
                Scaffold(modifier = Modifier.fillMaxSize()) { innerPadding ->
                    Greeting(
                        name = initResult,
                        modifier = Modifier.padding(innerPadding)
                    )
                }
            }
        }
    }
}

@Composable
fun Greeting(name: String, modifier: Modifier = Modifier) {
    Text(
        text = "Hello $name!",
        modifier = modifier
    )
}

@Preview(showBackground = true)
@Composable
fun GreetingPreview() {
    NodaNotesTheme {
        Greeting("Android")
    }
}