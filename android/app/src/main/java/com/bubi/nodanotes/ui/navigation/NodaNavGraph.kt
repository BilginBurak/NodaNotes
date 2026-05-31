package com.bubi.nodanotes.ui.navigation

import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.navigation.NavHostController
import androidx.navigation.NavType
import androidx.navigation.compose.composable
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import androidx.navigation.NavBackStackEntry
import androidx.compose.animation.AnimatedContentTransitionScope

import com.bubi.nodanotes.ui.screens.history.HistoryScreen
import com.bubi.nodanotes.ui.screens.vault.VaultSelectorScreen

import com.bubi.nodanotes.ui.screens.search.SearchScreen
import com.bubi.nodanotes.ui.screens.settings.SettingsScreen

import com.bubi.nodanotes.ui.screens.editor.NoteEditorScreen
import com.bubi.nodanotes.ui.screens.notelist.NoteListScreen
import com.bubi.nodanotes.ui.screens.attachments.AttachmentsScreen

@Composable
fun NodaNavGraph(
    navController: NavHostController,
    onMenuClick: () -> Unit,
    startDestination: String = Screen.VaultSelector.route
) {
    NavHost(
        navController = navController,
        startDestination = startDestination,
        modifier = Modifier.fillMaxSize(),
        enterTransition = {
            slideIntoContainer(
                towards = AnimatedContentTransitionScope.SlideDirection.Left,
                animationSpec = androidx.compose.animation.core.tween(300, easing = androidx.compose.animation.core.FastOutSlowInEasing)
            ) + fadeIn(animationSpec = androidx.compose.animation.core.tween(300))
        },
        exitTransition = {
            slideOutOfContainer(
                towards = AnimatedContentTransitionScope.SlideDirection.Left,
                animationSpec = androidx.compose.animation.core.tween(300, easing = androidx.compose.animation.core.FastOutSlowInEasing)
            ) + fadeOut(animationSpec = androidx.compose.animation.core.tween(300))
        },
        popEnterTransition = {
            slideIntoContainer(
                towards = AnimatedContentTransitionScope.SlideDirection.Right,
                animationSpec = androidx.compose.animation.core.tween(300, easing = androidx.compose.animation.core.FastOutSlowInEasing)
            ) + fadeIn(animationSpec = androidx.compose.animation.core.tween(300))
        },
        popExitTransition = {
            slideOutOfContainer(
                towards = AnimatedContentTransitionScope.SlideDirection.Right,
                animationSpec = androidx.compose.animation.core.tween(300, easing = androidx.compose.animation.core.FastOutSlowInEasing)
            ) + fadeOut(animationSpec = androidx.compose.animation.core.tween(300))
        }
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
                },
                onNavigateToSyncReport = { navController.navigate(Screen.SyncReport.route) }
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
            com.bubi.nodanotes.ui.screens.trash.TrashScreen(
                onBackClick = { navController.popBackStack() }
            )
        }
        composable(Screen.Conflicts.route) {
            com.bubi.nodanotes.ui.screens.conflict.ConflictScreen(
                onBackClick = { navController.popBackStack() }
            )
        }
        composable(Screen.Settings.route) {
            SettingsScreen(
                onBackClick = { navController.popBackStack() },
                onNavigateToVaultSelector = {
                    navController.navigate(Screen.VaultSelector.route) {
                        popUpTo(0) { inclusive = true }
                    }
                }
            )
        }
        composable(Screen.Attachments.route) {
            AttachmentsScreen(
                onBackClick = { navController.popBackStack() }
            )
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
