package com.bubi.nodanotes.ui.screens.search

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.bubi.nodanotes.data.model.SearchResultDto
import com.bubi.nodanotes.data.preferences.VaultPreferences
import com.bubi.nodanotes.data.repository.SearchRepository
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.flatMapLatest
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

sealed interface SearchUiState {
    object Idle : SearchUiState
    object Loading : SearchUiState
    data class Success(val results: List<SearchResultDto>) : SearchUiState
    data class Error(val message: String) : SearchUiState
}

class SearchViewModel(application: Application) : AndroidViewModel(application) {

    private val searchRepository = SearchRepository()
    private val preferences = VaultPreferences(application)

    private val _query = MutableStateFlow("")
    val query: StateFlow<String> = _query.asStateFlow()

    private val _recentSearches = MutableStateFlow(preferences.getRecentSearches())
    val recentSearches: StateFlow<List<String>> = _recentSearches.asStateFlow()

    @OptIn(FlowPreview::class, kotlinx.coroutines.ExperimentalCoroutinesApi::class)
    val uiState: StateFlow<SearchUiState> = _query
        .debounce(300)
        .distinctUntilChanged()
        .flatMapLatest { q ->
            flow {
                if (q.trim().isEmpty()) {
                    emit(SearchUiState.Idle)
                    return@flow
                }
                emit(SearchUiState.Loading)
                searchRepository.searchNotes(q)
                    .onSuccess { results ->
                        emit(SearchUiState.Success(results))
                    }
                    .onFailure { error ->
                        emit(SearchUiState.Error(error.message ?: "Unknown search error"))
                    }
            }
        }
        .stateIn(
            scope = viewModelScope,
            started = SharingStarted.WhileSubscribed(5000),
            initialValue = SearchUiState.Idle
        )

    fun onQueryChanged(newQuery: String) {
        _query.value = newQuery
    }

    fun searchTriggered(q: String) {
        if (q.trim().isNotEmpty()) {
            preferences.addRecentSearch(q)
            _recentSearches.value = preferences.getRecentSearches()
        }
    }

    fun selectRecentSearch(search: String) {
        _query.value = search
        searchTriggered(search)
    }

    fun clearRecentSearches() {
        preferences.clearRecentSearches()
        _recentSearches.value = emptyList()
    }
}
