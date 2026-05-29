package com.bubi.nodanotes.data.repository

import com.bubi.nodanotes.data.model.AppErrorDto
import kotlinx.serialization.DeserializationStrategy
import kotlinx.serialization.json.Json

open class BaseRepository {

    protected val json = Json {
        ignoreUnknownKeys = true
        coerceInputValues = true
    }

    class RustCoreException(message: String) : Exception(message)

    /**
     * Checks if the JNI response is an error JSON and throws RustCoreException if so.
     */
    protected fun checkError(responseJson: String) {
        if (responseJson.contains("\"error\":")) {
            try {
                val errorDto = json.decodeFromString(AppErrorDto.serializer(), responseJson)
                throw RustCoreException(errorDto.error)
            } catch (e: Exception) {
                if (e is RustCoreException) {
                    throw e
                }
                // If it is not a valid AppErrorDto but contains error, throw the raw string
                throw RustCoreException("JNI error: $responseJson")
            }
        }
    }

    /**
     * Parses the JNI response into the target type T, after checking for errors.
     */
    protected fun <T> parseRustResult(responseJson: String, strategy: DeserializationStrategy<T>): T {
        checkError(responseJson)
        return json.decodeFromString(strategy, responseJson)
    }
}
