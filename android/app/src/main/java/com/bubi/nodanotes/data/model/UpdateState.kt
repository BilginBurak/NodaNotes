package com.bubi.nodanotes.data.model

import kotlinx.serialization.Serializable

sealed class UpdateState {
    object NoUpdate : UpdateState()
    data class FlexibleUpdate(val tagName: String, val apkUrl: String, val releaseNotes: String = "") : UpdateState()
    data class MandatoryUpdate(val tagName: String, val apkUrl: String, val releaseNotes: String = "") : UpdateState()
}

@Serializable
data class GitHubAsset(
    val name: String,
    val browser_download_url: String
)

@Serializable
data class GitHubReleaseResponse(
    val tag_name: String,
    val body: String,
    val assets: List<GitHubAsset>
)
