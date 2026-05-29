package com.bubi.nodanotes.ui.navigation

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.navigation.NavHostController
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.navArgument

import com.bubi.nodanotes.ui.screens.history.HistoryScreen
import com.bubi.nodanotes.ui.screens.vault.VaultSelectorScreen

import com.bubi.nodanotes.ui.screens.search.SearchScreen
import com.bubi.nodanotes.ui.screens.settings.SettingsScreen

import com.bubi.nodanotes.ui.screens.editor.NoteEditorScreen
import com.bubi.nodanotes.ui.screens.notelist.NoteListScreen

@Composable
fun NodaNavGraph(
    navController: NavHostController,
    onMenuClick: () -> Unit,
    startDestination: String = Screen.VaultSelector.route
) {
    NavHost(
        navController = navController,
        startDestination = startDestination,
        modifier = Modifier.fillMaxSize()
    ) {
        composable(Screen.VaultSelector.route) {
            VaultSelectorScreen(
                onVaultSelected = { vaultPath ->
                    navController.navigate(Screen.NoteList.route) {
                        popUpTo(Screen.VaultSelector.route) { inclusive = true }
                    }
                }
            )
        }
        composable(Screen.NoteList.route) {
            NoteListScreen(
                onMenuClick = onMenuClick,
                onSearchClick = { navController.navigate(Screen.Search.route) },
                onNavigateToEditor = { noteId ->
                    navController.navigate(Screen.NoteEditor.createRoute(noteId))
                }
            )
        }

        composable(
            route = Screen.NoteEditor.ROUTE,
            arguments = listOf(navArgument("noteId") { type = NavType.StringType })
        ) { backStackEntry ->
            val noteId = backStackEntry.arguments?.getString("noteId") ?: ""
            NoteEditorScreen(
                noteId = noteId,
                onBackClick = { navController.popBackStack() },
                onNavigateToHistory = { id -> navController.navigate(Screen.History.createRoute(id)) }
            )
        }
        composable(Screen.Search.route) {
            SearchScreen(
                onBackClick = { navController.popBackStack() },
                onNavigateToEditor = { noteId ->
                    navController.navigate(Screen.NoteEditor.createRoute(noteId)) {
                        // Pop up to NoteList to prevent accumulation of screens in the stack
                        popUpTo(Screen.NoteList.route)
                    }
                }
            )
        }
        composable(
            route = Screen.History.ROUTE,
            arguments = listOf(navArgument("noteId") { type = NavType.StringType })
        ) { backStackEntry ->
            val noteId = backStackEntry.arguments?.getString("noteId") ?: ""
            HistoryScreen(
                noteId = noteId,
                onBackClick = { navController.popBackStack() }
            )
        }
        composable(Screen.Trash.route) {
            PlaceholderScreen(name = "Trash Screen")
        }
        composable(Screen.Conflicts.route) {
            com.bubi.nodanotes.ui.screens.conflict.ConflictScreen(
                onBackClick = { navController.popBackStack() }
            )
        }
        composable(Screen.Settings.route) {
            SettingsScreen(
                onBackClick = { navController.popBackStack() },
                onNavigateToMaintenance = { navController.navigate(Screen.Maintenance.route) },
                onNavigateToVaultSelector = {
                    navController.navigate(Screen.VaultSelector.route) {
                        popUpTo(0) { inclusive = true }
                    }
                }
            )
        }
        composable(Screen.Maintenance.route) {
            PlaceholderScreen(name = "Maintenance Screen")
        }
        composable(Screen.SyncReport.route) {
            val vaultPreferences = remember { com.bubi.nodanotes.data.preferences.VaultPreferences(navController.context) }
            com.bubi.nodanotes.ui.screens.sync.SyncReportScreen(
                onBackClick = { navController.popBackStack() },
                onNavigateToConflicts = { navController.navigate(Screen.Conflicts.route) },
                vaultPreferences = vaultPreferences
            )
        }
    }
}

@Composable
fun PlaceholderScreen(name: String) {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center
    ) {
        Text(text = name)
    }
}
