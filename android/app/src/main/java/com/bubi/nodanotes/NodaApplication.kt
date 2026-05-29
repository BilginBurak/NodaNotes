package com.bubi.nodanotes

import android.app.Application
import android.app.NotificationChannel
import android.app.NotificationManager
import android.os.Build

class NodaApplication : Application() {

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channelId = "noda_sync"
            val channelName = "Sync Notifications"
            val channelDescription = "Notifications related to NodaNotes database synchronization"
            val importance = NotificationManager.IMPORTANCE_LOW

            val channel = NotificationChannel(channelId, channelName, importance).apply {
                description = channelDescription
            }

            val notificationManager = getSystemService(NotificationManager::class.java)
            notificationManager?.createNotificationChannel(channel)
        }
    }
}
