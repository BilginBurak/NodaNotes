import Testing
@testable import Noda

// MARK: - PathSanitizerTests

@Suite("PathSanitizer")
struct PathSanitizerTests {

    let sanitizer = PathSanitizer()

    @Test func removesInvalidCharacters() {
        #expect(sanitizer.sanitize("note/name") == "notename")
        #expect(sanitizer.sanitize("note:name") == "notename")
        #expect(sanitizer.sanitize("note*name") == "notename")
        #expect(sanitizer.sanitize("note?name") == "notename")
        #expect(sanitizer.sanitize("note\"name") == "notename")
        #expect(sanitizer.sanitize("note<name>") == "notename")
        #expect(sanitizer.sanitize("note|name") == "notename")
        #expect(sanitizer.sanitize("note\\name") == "notename")
    }

    @Test func stripsLeadingTrailingSpacesAndDots() {
        #expect(sanitizer.sanitize("  note  ") == "note")
        #expect(sanitizer.sanitize("...note...") == "note")
        #expect(sanitizer.sanitize(". note .") == "note")
    }

    @Test func enforcesMaxLength() {
        let long = String(repeating: "a", count: 300)
        #expect(sanitizer.sanitize(long).count == 255)
    }

    @Test func emptyBecomesUntitled() {
        #expect(sanitizer.sanitize("") == "Untitled")
        #expect(sanitizer.sanitize("...") == "Untitled")
        #expect(sanitizer.sanitize("   ") == "Untitled")
    }

    @Test func validFilenamePassesThrough() {
        #expect(sanitizer.sanitize("My Note") == "My Note")
        #expect(sanitizer.sanitize("meeting-2026") == "meeting-2026")
    }

    @Test func isValidReturnsTrueForCleanName() {
        #expect(sanitizer.isValid("My Note") == true)
        #expect(sanitizer.isValid("note/bad") == false)
        #expect(sanitizer.isValid("") == false)
    }
}
