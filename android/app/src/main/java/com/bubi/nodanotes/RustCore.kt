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

    /**
     * Exposes the JNI function to initialize a vault at a given filesystem path.
     */
    external fun initVault(path: String): String
}
