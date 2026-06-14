# pc 
Sen Antigravity'sin. NodaNotes Rust + Tauri uygulamasını geliştireceğiz. 

### 1. ZORUNLU OKUMA (Başlamadan Önce)

Bu sırayla oku ve sistemi kavra:
1. `.agent/DEVLOG.md` — Tüm projenin master kayıt defteri. En önemli dosya. MUTLAKA oku.
2. `.agent/core/steering.md` — Değişmez kurallar ve yasak kalıplar
3. `.agent/core/spec.md` — Fonksiyonel gereksinimler
4. `.agent/core/design.md` — Sistem mimarisi
5. `.agent/core/requirements.md` — FR/NFR/CON listesi


### 2. SANA ÖZEL ÇALIŞMA KURALLARI (ÇOK ÖNEMLİ)

* **Dumb Monitor:** Tauri-Svelte frontend sadece görüntüler, komut gönderir. Dosya okuma/yazma, sync, search = Rust.
* **Çelişki Durumu:** Herhangi bir döküman ile `.agent/DEVLOG.md` çelişirse, DEVLOG kazanır.
* **Dil Kuralları:** Benimle (kullanıcıyla) chat üzerindeki tüm iletişimin **Türkçe** olmalıdır. Ancak bunun dışındaki her şey (yazdığın kodlar, yorum satırları, commit mesajları, hata çıktıları ve teknik dokümantasyonlar) tamamen **İngilizce** olmalıdır.
* **Task İşaretleme (Checkboxes):** Herhangi bir task üzerinde çalışırken ve o task'i tamamladığında, mutlaka  ilgili checkbox'ı (`[ ]` -> `[x]`) işaretle/güncelle.
* **Olağan Dışı Bulgular & Direksiyon Rehberi:** Kod yazarken veya sistemi incelerken olağan dışı, kritik veya çok önemli bir bulgu/öğrenim elde edersen (her basit task'ten sonra değil, sadece gerçekten önemli ve geleceğe ışık tutacak durumlarda), kullanıcıya: `"Bu bulguyu core/steering.md dosyasına Project-Specific Patterns başlığı altına eklemek ister misiniz?"` diye sor. Kullanıcı onay verirse bu bulguyu ilgili yere ekle.
* **Rust-First:** Tauri-Svelte tarafında HİÇBİR iş mantığı yazılmaz. Tüm veriler RustCore singleton üzerinden JSON String olarak alınır. Svelte sadece "Dumb Monitor" (Aptal Ekran) olarak görev yapar.

### 3. PROJE BİLGİLERİ VE DIZINLER

* **Kök dizin:** `/Users/burakbilgin/Documents/Kodlar/Rust/NodaNotes`
* **tauri-shell:** `crates/tauri-shell` klasörü cargo tauri dev komutu burada çalıştırılacak.
* **vault klasörü:** `/Users/burakbilgin/NodaVault/NodaRust`

---

### ILK GÖREVİN: aşağıdaki hata ve eksikler listesi ile İşe Koyul!
liste:
#### 🚨 ACİL TAMİR: Mantık Hataları ve Bug Temizliği
- [ ] **Settings Modalı Tamiri (Tauri/Rust):** settings-modal'da inputların neredeyse hiç biri çalışmıyor. şu uyarıyı veriyor tarayıcı console "[Warning] [svelte] binding_property_non_reactive (client.js, line 3356)
`bind:group={localConfig.appearance.theme}` (src/lib/components/settings/SettingsModal.svelte:453:36) is binding to a non-reactive property
https://svelte.dev/e/binding_property_non_reactive" bununla alakalı olabilir. `nav-tab-maintenance` altındaki settings-section -> maintenance-card -> butonlarını incele. birçoğu çalışmıyor.
çalışmayan tüm fonksiyonları (indeks temizliği, vault kontrolü, gereksiz duplicate/attachments/remnants silme, reset queue, clear cache, rebuil cache, vacuum db vb.) incele.
ayrıca sanırım "90ddea0" commitinden sonra başladı sanırım bu sorun. farketmeden birşeyleri bozmuş olabilirim. 
- [ ] **Inline Tag İzolasyonu (Rust):** `save_note` fonksiyonunu revize et. `#inline_tag` yapıları regex ile yakalanıp sadece SQLite'taki `note_tags` tablosuna (`source = 'inline'`) yazılmalı; KESİNLİKLE dosyanın üstündeki YAML alanına enjekte edilmemeli. Metinden silindiğinde SQLite'tan da temizlenmeli.
- [ ] **Tag Manager UI İyileştirmesi:** tag-manager svelte-1mnw26j tagın inline mi yoksa yaml mı olduğunu göstersin. inline ise tıklayınca notta ilgili satıra odaklansın. yaml ise ekleyip çıkarma imkanı versin.
- [ ] **Akıllı History Yönetimi (Debounce/Milestone):** Autosave mekanizmasını değiştir. Her autosave tetiklendiğinde history klasörüne dosya YAZMA. History yedeklemesini sadece: 1) Başka bir nota geçildiğinde (blur), 2) Kesintisiz yazımda en az 5 dakika geçince (ayarlar kısmından süreyi değiştirebilsin), 3) Sync motoru ateşlenmeden hemen önce, 4) uygulama kapanmadan önce tetikle. dosya içerisinde bir değişiklik olmamışsa yukarıdaki durumlar gerçekleşse bile snapshot oluşturma. 
- [ ] **Live Preview Task Toggle (Svelte & Rust):** Live Preview ve Reading modda render edilen `- [ ]` checkbox elemanlarına tıklama eventi ekle. Tıklandığı an Rust metindeki ilgili satırı `- [x]` (veya tersi) olarak güncellesin. değşiiklik olduğunda snapshot kuralları varsayılan şekilde işlesin. 

---

Yukarıdaki tüm dokümanları ve kuralları okuyup anladıktan sonra, listedeki tüm görevleri sırasıyla tamamla, testlerini yap, eğer testler başarılıysa doğrudan bir sonraki taska geç. 
sorunları çözmeye çalışırken veya geliştirme yaparken rust ile alaklı olmadığına 100% eminsen frontende odaklanabilirsin. 
adımları tamamladıktan sonra taskların checkbox'ları işaretle. her adımı otomatik ve kendi içinde onayla(implementation plan, test sonuçları vs bana sorma). bana sadece tüm tamamlanış halde olan taskları tek seferde teslim et.
Herhangi bir aşamada tıkanırsan veya kodda kırılma yaşarsan süreci durdurma; en rasyonel fallback (geri çekilme) mekanizmasını kurarak bir sonraki adıma geç ve nihai raporda bana nerede ne yaptığını açıkça belirt.




---------
# android




Sen Antigravity'sin. NodaNotes Android uygulamasını geliştireceğiz. 

### 1. ZORUNLU OKUMA (Başlamadan Önce)

Bu sırayla oku ve sistemi kavra:
1. `.agent/DEVLOG.md` — Tüm projenin master kayıt defteri. En önemli dosya. MUTLAKA oku.
2. `.agent/android/steering.md` — Değişmez kurallar ve yasak kalıplar
3. `.agent/android/bridge-spec.md` — JNI fonksiyon imzaları ve JSON şemaları
4. `.agent/android/design.md` — Sistem mimarisi ve veri akışları
5. `.agent/android/ui-spec.md` — Ekran ve bileşen spesifikasyonları

İhtiyaç duyarsan:
- `.agent/android/spec.md` — Fonksiyonel gereksinimler

### 2. SANA ÖZEL ÇALIŞMA KURALLARI (MUTLAK ZORUNLULUK)
* **Dumb Monitor:** Kotlin sadece görüntüler, komut gönderir. Dosya okuma/yazma, DB operasyonları, sync, search işlemleri tamamen Rust Core sorumluluğundadır. Kotlin tarafında HİÇBİR iş mantığı (business logic) yazılamaz.
* **Çelişki Durumu:** Herhangi bir döküman ile `.agent/DEVLOG.md` çelişirse, DEVLOG kazanır.
* **Threading:** Tüm JNI çağrıları Kotlin tarafında kesinlikle `Dispatchers.IO` üzerinde çalıştırılmalıdır, asla Main Thread bloklanamaz.
* **Vault Path:** Kullanıcı tarafından SAF/Storage Access Framework ile seçilir (`MANAGE_EXTERNAL_STORAGE`). `SharedPreferences` veya `DataStore` üzerinde kalıcı saklanır ve Rust Core'a pointer olarak geçilir.
* **Tema:** Tamamen Material 3 Dynamic Color (Monet) destekli olmalıdır. Kod içinde hardcoded renk kullanımı yasaktır.
* **Dil Kuralları:** Kullanıcıyla chat üzerindeki tüm iletişimin **Türkçe** olmalıdır. Ancak bunun dışındaki her şey (yazdığın kodlar, yorum satırları, commit mesajları, hata çıktıları ve teknik dokümantasyonlar) tamamen **İngilizce** olmalıdır.
* **Task İşaretleme (Checkboxes):** Herhangi bir task üzerinde çalışırken ve o task'i tamamladığında, mutlaka ilgili checkbox'ı (`[ ]` -> `[x]`) işaretle/güncelle.
* **Olağan Dışı Bulgular & Direksiyon Rehberi:** Kod yazarken gerçekten önemli ve geleceğe ışık tutacak kritik bir bulgu/öğrenim elde edersen kullanıcıya: `"Bu bulguyu android/steering.md dosyasına Project-Specific Patterns başlığı altına eklemek ister misiniz?"` diye sor.
* **No Half-Baked Commits:** Adımları tamamladıktan sonra taskların checkbox'larını işaretle. Her adımı otomatik ve kendi içinde onayla (implementation plan, test sonuçları vs kullanıcıya sorma). Bana sadece tüm tamamlanmış halde olan taskları tek seferde teslim et.

---

### 3. PROJE BİLGİLERİ VE DIZINLER
* **Kök dizin:** `/Users/burakbilgin/Documents/Kodlar/Rust/NodaNotes`
* **Vault klasörü:** `/Users/burakbilgin/NodaVault/NodaSQLite`
* **Android proje dizini:** `android/` klasörü
* **Rust bridge:** `crates/android-bridge/src/lib.rs`
* **Kotlin kaynak dizini:** `android/app/src/main/java/com/bubi/nodanotes/`
* **Derleme Komutu:** `cd android && JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home" ./gradlew installDebug` (Rust bridge'i de otomatik derler ve yükler)

#### WebDAV Sunucu Bilgileri (InfiniCloud):
- URL: `https://rausu.infini-cloud.net/dav/rusttest`
- Kullanıcı adı: `kerimaydinn168`
- Parola: `jgRFjaSZL3rP2f4q`

---

### 4. İSTEK LİSTESİ (İLK GÖREVİN):aşağıdaki hata ve eksikler listesi ile İşe Koyul!
liste:
- uygulama artık tamamen sqlite entegre olacak. yereldeki vault klasörü: /Users/burakbilgin/NodaVault/NodaSQLite bu dizindeki özellikle .noda klasörünü oku. bu klasörde sync ve diğer meta veriler tutuluyor. uygulama hala markdown dosyalarını okuyup işlemeye devam edecek. filesystem first yaklaşımımız hala devam ediyor. attachments, conflicts, history, trash klasörleri dosyaların gerçek hallerini saklayacak. sqlite config verileri, queue, sync durumu, remote state, hızlı indexleme ve arama için kullanılmaya devam edecek.  


---

### 5. RAPORLAMA VE DEVLOG

Herhangi bir aşamada tıkanırsan veya kodda kırılma yaşarsan süreci durdurma; en rasyonel fallback (geri çekilme) mekanizmasını kurarak bir sonraki adıma geç ve nihai raporda bana nerede ne yaptığını açıkça belirt.

Yaptığın tüm güncellemeleri, şema değişikliklerini ve karşılaşılan OS-level/JNI engellerini `.agent/DEVLOG.md` dosyası içerisine teknik ve detaylıca İngilizce olarak kaydet. Görev tamamlandığında bana tüm doğrulanmış taskları tek seferde teslim et.

İşe koyul!