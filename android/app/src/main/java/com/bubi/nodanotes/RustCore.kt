package com.bubi.nodanotes

import android.util.Log

object RustCore {
    private const val TAG = "RustCore"

    init {
        try {
            System.loadLibrary("android_bridge")
            Log.d(TAG, "Rust Core library loaded successfully.")
        } catch (e: UnsatisfiedLinkError) {
            Log.e(TAG, "Failed to load Rust Core library: ${e.message}")
        }
    }

    // Vault Functions
    external fun initVault(path: String): String
    external fun getVaultInfo(inputJson: String): String
    external fun refreshVault(inputJson: String): String

    // Note Functions
    external fun listNotes(inputJson: String): String
    external fun getAllNotes(inputJson: String): String
    external fun getNote(inputJson: String): String
    external fun createNote(inputJson: String): String
    external fun updateNote(inputJson: String): String
    external fun renameNote(inputJson: String): String
    external fun deleteNote(inputJson: String): String
    external fun moveNote(inputJson: String): String
    external fun getNoteMetadata(inputJson: String): String
    external fun getAllTags(inputJson: String): String

    // Folder Functions
    external fun listFolders(inputJson: String): String
    external fun createFolder(inputJson: String): String
    external fun renameFolder(inputJson: String): String
    external fun deleteFolder(inputJson: String): String
    external fun moveFolder(inputJson: String): String

    // Search Functions
    external fun searchNotes(inputJson: String): String

    // History Functions
    external fun listSnapshots(inputJson: String): String
    external fun getSnapshotDiff(inputJson: String): String
    external fun restoreSnapshot(inputJson: String): String
    external fun deleteSnapshot(inputJson: String): String

    // Trash Functions
    external fun listTrash(inputJson: String): String
    external fun restoreFromTrash(inputJson: String): String
    external fun permanentDelete(inputJson: String): String
    external fun emptyTrash(inputJson: String): String

    // Attachment Functions
    external fun addAttachment(inputJson: String): String
    external fun listAttachments(inputJson: String): String
    external fun getAttachmentData(inputJson: String): String
    external fun deleteAttachment(inputJson: String): String

    // Sync Functions
    external fun saveSyncConfig(inputJson: String): String
    external fun loadSyncConfig(inputJson: String): String
    external fun testWebdavConnection(inputJson: String): String
    external fun syncNow(inputJson: String): String
    external fun getSyncStatus(inputJson: String): String

    // Conflict Functions
    external fun listConflicts(inputJson: String): String
    external fun getConflictNote(inputJson: String): String
    external fun resolveConflict(inputJson: String): String

    // Maintenance Functions
    external fun rebuildCache(inputJson: String): String
    external fun optimizeFts(inputJson: String): String
    external fun getDuplicateNotes(inputJson: String): String
    external fun deleteDuplicateFile(inputJson: String): String
    external fun getOrphanedRemnants(inputJson: String): String
    external fun deleteOrphanedFile(inputJson: String): String
    external fun deleteAllOrphanedRemnants(inputJson: String): String
    external fun getOrphanedAttachments(inputJson: String): String
    external fun clearRemoteTrackingCache(inputJson: String): String
    external fun resetSyncQueue(inputJson: String): String

    // Settings Functions
    external fun getSettings(inputJson: String): String
    external fun updateSettings(inputJson: String): String

    // Daily & Task Functions
    external fun triggerDailyNote(inputJson: String): String
    external fun toggleTaskStatus(inputJson: String): String
    external fun listTagsWithCounts(inputJson: String): String

    // Clipper Functions
    external fun check_url_history(url: String): String
    external fun clipUrl(inputJson: String): String

    // Cryptographic & Vault Session Functions
    external fun isVaultConfigured(inputJson: String): String
    external fun setMasterPassword(inputJson: String): String
    external fun checkVaultStatus(inputJson: String): String
    external fun unlockVaultSession(inputJson: String): String
    external fun lockVaultInstantly(inputJson: String): String
    external fun toggleNoteEncryption(inputJson: String): String
    external fun getVaultTimeoutSetting(inputJson: String): String
    external fun setVaultTimeoutSetting(inputJson: String): String
    external fun changeMasterPassword(inputJson: String): String

    interface SyncProgressListener {
        fun onProgress(json: String)
    }

    private val progressListeners = mutableListOf<SyncProgressListener>()

    fun addProgressListener(listener: SyncProgressListener) {
        synchronized(progressListeners) {
            progressListeners.add(listener)
        }
    }

    fun removeProgressListener(listener: SyncProgressListener) {
        synchronized(progressListeners) {
            progressListeners.remove(listener)
        }
    }

    @JvmStatic
    fun onSyncProgress(json: String) {
        Log.d(TAG, "onSyncProgress: $json")
        synchronized(progressListeners) {
            for (listener in progressListeners) {
                try {
                    listener.onProgress(json)
                } catch (e: Exception) {
                    Log.e(TAG, "Error in progress listener", e)
                }
            }
        }
    }
}
