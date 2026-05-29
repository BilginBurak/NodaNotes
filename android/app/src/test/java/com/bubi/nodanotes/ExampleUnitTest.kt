package com.bubi.nodanotes

import com.bubi.nodanotes.data.model.NoteDto
import kotlinx.serialization.json.Json
import org.junit.Test
import org.junit.Assert.*

class ExampleUnitTest {
    @Test
    fun addition_isCorrect() {
        assertEquals(4, 2 + 2)
    }

    @Test
    fun noteDtoSerialization_isCorrect() {
        val jsonStr = """
            {
                "id": "01JXYZ",
                "parent_id": null,
                "title": "Test Title",
                "body": "Test Body",
                "color": "#FF0000",
                "pinned": true,
                "tags": ["tag1", "tag2"],
                "created_at": "2026-05-28T21:00:00Z",
                "updated_at": "2026-05-28T22:30:00Z",
                "file_path": "test.md"
            }
        """.trimIndent()

        val note = Json.decodeFromString<NoteDto>(jsonStr)
        assertEquals("01JXYZ", note.id)
        assertNull(note.parent_id)
        assertEquals("Test Title", note.title)
        assertEquals("Test Body", note.body)
        assertEquals("#FF0000", note.color)
        assertTrue(note.pinned)
        assertEquals(2, note.tags.size)
        assertEquals("test.md", note.file_path)
    }
}