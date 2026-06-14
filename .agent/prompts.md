# NodaNotes — AI Agent Prompt Templates

Bu dosya, yeni bir AI sohbeti açıldığında kullanılacak hazır prompt şablonlarını içerir.
İlgili şablonu kopyalayıp yeni sohbete yapıştır.

---

## 1. Evrensel Başlangıç Promptu (Her Görev İçin)

```markdown
Sen Antigravity'sin. NodaNotes cross-platform Markdown vault uygulaması üzerinde çalışacağız.

### ZORUNLU OKUMA (Başlamadan Önce — Sırayla)

1. `.agent/DEVLOG.md` — TÜM projenin master kayıt defteri. En önemli dosya. MUTLAKA oku.
2. Görevin kapsamına göre:
   - **Rust Core** (crates/core) → `.agent/core/steering.md`
   - **Android** (Kotlin/JNI) → `.agent/android/steering.md` + `.agent/android/bridge-spec.md`
   - **Tauri/macOS** (SvelteKit) → `.agent/core/steering.md` + `.agent/core/design.md`

### TEMEL KURALLAR

- **Felsefe:** Rust-First, UI-Second. Kotlin ve Svelte sadece "Dumb Monitor" (Aptal Ekran).
- **Çelişki Durumu:** Herhangi bir döküman ile `.agent/DEVLOG.md` çelişirse, DEVLOG kazanır.
- **File-System Is Truth:** `.md` dosyaları yegane veritabanıdır. SQLite sadece FTS5 önbellektir.
- **Dil:** Kod + yorum + commit = İngilizce. Seninle konuşma = Türkçe.

### PROJE DİZİNLERİ

- **Kök:** `/Users/burakbilgin/Documents/Kodlar/Rust/NodaNotes`
- **Rust Core:** `crates/core/`
- **Tauri Shell:** `crates/tauri-shell/`
- **Android JNI Bridge:** `crates/android-bridge/`
- **SvelteKit Frontend:** `frontend/`
- **Android Proje:** `android/`

---

**Görev:** [Buraya görevi yaz]
```

---

## 2. Sadece Rust Core İçin

```markdown
Sen Antigravity'sin. NodaNotes projesinin Rust Core katmanında çalışacağız.

### ZORUNLU OKUMA

1. `.agent/DEVLOG.md` — Tüm kritik Rust kararları burada. İlk oku.
2. `.agent/core/steering.md` — Değişmez kurallar ve yasak kalıplar.

### ÖNEMLİ HATIRLATMALAR

- `crates/core` Tauri'ye, WebView'e, JNI'ya bağımlı OLAMAZ. Standalone derlenmeli.
- SQLite schema version **8** aktif. Tablolar: notes, tags, note_tags, sync_file_states, sync_device_states, peer_file_states.
- `.unwrap()`, `.expect()`, `panic!()` production path'lerde YASAK.
- Lock guard'ları `.await` boundary'lerinde tutma. Kısa scope kullan.
- History: `.noda/history/` altında flat file, `[NoteID]_[YYYYMMDD-HHMMSS]_[reason].md` formatı.
- Sync: Git-style manifest tree (DEVLOG §44-45), XXH3 hash, 0-byte .sync markers.

### PROJE DİZİNİ

`/Users/burakbilgin/Documents/Kodlar/Rust/NodaNotes`

---

**Görev:** [Buraya görevi yaz]
```

---

## 3. Android (Kotlin/JNI) Geliştirmesi İçin

```markdown
Sen Antigravity'sin. NodaNotes Android uygulamasında çalışacağız.

### ZORUNLU OKUMA (Sırayla)

1. `.agent/DEVLOG.md` — Master kayıt. En önemli. Rust Core kararları burada.
2. `.agent/android/steering.md` — Android değişmez kurallar ve yasaklar.
3. `.agent/android/bridge-spec.md` — JNI fonksiyon imzaları ve JSON şemaları.
4. `.agent/android/design.md` — Sistem mimarisi ve veri akışları.
5. `.agent/android/ui-spec.md` — Ekran ve bileşen spesifikasyonları. (UI görevi ise)

İhtiyaç duyarsan:
- `.agent/android/spec.md` — Fonksiyonel gereksinimler

### TEMEL KURALLAR

- **Dumb Monitor:** Kotlin sadece render eder. İş mantığı = sıfır.
- **Tüm JNI çağrıları `Dispatchers.IO` üzerinde** — asla main thread'de değil.
- **Vault:** Kullanıcı seçer → `MANAGE_EXTERNAL_STORAGE`. Yol `SharedPreferences`'te saklanır.
- **Tema:** Silent Sanctuary Japandi paleti (DEVLOG §48). Hardcoded renk yok.
- **Attachment:** `RustCore.getAttachmentData()` → base64 → BitmapFactory. `noda://` URI DEĞİL.
- **Çelişki:** DEVLOG kazanır.

### PROJE DİZİNLERİ

- **Kök:** `/Users/burakbilgin/Documents/Kodlar/Rust/NodaNotes`
- **Android proje:** `android/`
- **JNI bridge:** `crates/android-bridge/src/lib.rs`
- **Kotlin kaynak:** `android/app/src/main/java/com/bubi/nodanotes/`
- **Derleme:** `cd android && JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home" ./gradlew assembleDebug`

---

**Görev:** [Buraya görevi yaz]
```

---

## 4. Tauri/macOS (SvelteKit) Geliştirmesi İçin

```markdown
Sen Antigravity'sin. NodaNotes Tauri/macOS frontend'inde çalışacağız.

### ZORUNLU OKUMA

1. `.agent/DEVLOG.md` — Rust Core kararları ve Tauri-spesifik tuzaklar burada.
2. `.agent/core/steering.md` — Değişmez kurallar.
3. `.agent/core/design.md` — Sistem mimarisi ve Svelte bileşen yapısı.

### ÖNEMLİ HATIRLATMALAR (Tauri-Spesifik Tuzaklar)

- **Svelte 5:** `on:click` YASAK → `onclick` kullan (DEVLOG §noda-steering §11).
- **Crate aliasing:** `noda_core = { package = "core" }` — `core` crate'i standart Rust `core`'u gölgeler.
- **Adapter-static zorunlu.** SSR YOK.
- **Bun** package manager (npm değil).
- **noda://** custom protocol → attachment'lar bu yolla serve edilir (Android'de değil).
- **macOS window:** `transparent: true`, `hiddenTitle: true`, `titleBarStyle: "overlay"`.
- **IPC:** Tüm komutlar `Result<T, AppError>` döner. `.unwrap()` YASAK.

### PROJE DİZİNLERİ

- **Kök:** `/Users/burakbilgin/Documents/Kodlar/Rust/NodaNotes`
- **Frontend:** `frontend/`
- **Tauri Shell:** `crates/tauri-shell/`
- **Dev başlat:** `cd crates/tauri-shell && cargo tauri dev`

---

**Görev:** [Buraya görevi yaz]
```

---

## 5. Çift Platform (Rust Core + Android/Tauri aynı anda) İçin

```markdown
Sen Antigravity'sin. NodaNotes projesinde hem Rust Core hem de [Android/Tauri] tarafını aynı anda değiştireceğiz.

### ZORUNLU OKUMA

1. `.agent/DEVLOG.md` — İLK oku. Tüm kritik kararlar burada.
2. `.agent/android/steering.md` + `.agent/android/bridge-spec.md` (Android varsa)
3. `.agent/core/steering.md` (Tauri varsa)

### SINIR KURALLARI (Kotlin ↔ Rust ↔ Svelte)

- Her yeni Rust Core fonksiyonu için:
  - Android: `crates/android-bridge/src/lib.rs`'e JNI wrapper ekle
  - Tauri: `crates/tauri-shell/src/commands/`'a `#[tauri::command]` ekle
- JNI şablonu için `android/bridge-spec.md §1.2`'e bak.
- JSON input/output her iki platformda aynı şema.

---

**Görev:** [Buraya görevi yaz]
```
