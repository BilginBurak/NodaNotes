package com.bubi.nodanotes.data.repository

import com.bubi.nodanotes.RustCore
import com.bubi.nodanotes.data.model.SearchResultDto
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer

class SearchRepository : BaseRepository() {

    @Serializable
    private data class SearchParams(val query: String)

    suspend fun searchNotes(query: String): Result<List<SearchResultDto>> = withContext(Dispatchers.IO) {
        runCatching {
            val params = SearchParams(query)
            val jsonInput = json.encodeToString(SearchParams.serializer(), params)
            val result = RustCore.searchNotes(jsonInput)
            parseRustResult(result, ListSerializer(SearchResultDto.serializer()))
        }
    }
}
