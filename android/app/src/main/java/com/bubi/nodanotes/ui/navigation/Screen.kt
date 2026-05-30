package com.bubi.nodanotes.ui.navigation

sealed class Screen(val route: String) {
    object VaultSelector : Screen("vault_selector")
    object NoteList : Screen("note_list")
    
    object NoteEditor : Screen("note_editor/{noteId}") {
        const val ROUTE = "note_editor/{noteId}"
        fun createRoute(noteId: String) = "note_editor/$noteId"
    }
    
    object Search : Screen("search")
    
    object History : Screen("history/{noteId}") {
        const val ROUTE = "history/{noteId}"
        fun createRoute(noteId: String) = "history/$noteId"
    }
    
    object Trash : Screen("trash")
    object Conflicts : Screen("conflicts")
    object Settings : Screen("settings")
    object Maintenance : Screen("maintenance")
    object Attachments : Screen("attachments")
    
    object SyncReport : Screen("sync_report")
}
