package com.bubi.nodanotes.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable

import androidx.compose.ui.graphics.Color

private val DarkColorScheme = darkColorScheme(
    primary = DarkAccent,
    onPrimary = DarkBackground,
    primaryContainer = DarkPrimaryContainer,
    onPrimaryContainer = DarkText,
    secondary = DarkAccent,
    onSecondary = DarkText,
    secondaryContainer = DarkSurface,
    onSecondaryContainer = DarkText,
    background = DarkBackground,
    onBackground = DarkText,
    surface = DarkSurface,
    onSurface = DarkText,
    surfaceVariant = DarkSurface, // Flat design: surfaceVariant matches surface or slightly darker
    onSurfaceVariant = DarkMutedText,
    outline = DarkMutedText,
    outlineVariant = DarkSurface, // Eliminate border stroke visual lines
    surfaceTint = Color.Transparent
)

private val LightColorScheme = lightColorScheme(
    primary = LightAccent,
    onPrimary = LightSurface,
    primaryContainer = LightPrimaryContainer,
    onPrimaryContainer = LightText,
    secondary = LightAccent,
    onSecondary = LightText,
    secondaryContainer = LightSurface,
    onSecondaryContainer = LightText,
    background = LightBackground,
    onBackground = LightText,
    surface = LightSurface,
    onSurface = LightText,
    surfaceVariant = LightSurface, // Flat design: surfaceVariant matches surface or slightly darker
    onSurfaceVariant = LightMutedText,
    outline = LightMutedText,
    outlineVariant = LightSurface, // Eliminate border stroke visual lines
    surfaceTint = Color.Transparent
)

@Composable
fun NodaTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = false, // Disable Monet/dynamic colors by default
    customColorScheme: ColorScheme? = null,
    content: @Composable () -> Unit
) {
    // Dynamic color (Monet) is explicitly disabled to enforce the "Silent Sanctuary" palette
    val colorScheme = when {
        customColorScheme != null -> customColorScheme
        darkTheme -> DarkColorScheme
        else -> LightColorScheme
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = NodaTypography,
        content = content
    )
}