# Noda Gelişim Logu ve Teknik Geliştirmeler Raporu

Bu döküman, NodaNotes projesi üzerinde gerçekleştirilen tüm geliştirme aşamalarını, karşılaşılan problemleri, uygulanan çözüm yollarını ve elde edilen teknik kazanımları özetlemektedir. Dökümantasyon, yapılan gerçek geliştirmelere sadık kalınarak hazırlanmıştır.

---

## 1. Vault Yönetimi, WebDAV Senkronizasyonu ve Dosya İzleme Sistemleri

### Karşılaşılan Problemler
* **Kalıcı Vault Seçimi:** Uygulama kapatılıp açıldığında seçilen yerel Vault dizini hafızada tutulmuyor, her seferinde kullanıcıya yeniden seçtiriliyordu.
* **Yeni Not Butonu:** Arayüzdeki "New Note" butonu işlevsizdi ve yeni dosya oluşturamıyordu.
* **WebDAV Bilgilerinin Kaybolması:** Sunucu bağlantı bilgileri kalıcı olarak diske yazılmıyor, doğrulama ve senkronizasyon motoru çalışmıyordu.
* **Harici Dosya Değişiklikleri:** Vault klasörüne işletim sistemi üzerinden manuel olarak atılan `.md` uzantılı yeni dosyalar uygulama tarafından otomatik fark edilip listeye eklenmiyordu.

### Çözüm Yöntemi ve Güncellemeler
* **Konfigürasyon Katmanı:** Rust backend tarafında Vault yolu ve WebDAV bağlantı parametrelerinin `.noda/config.json` dosyasına asenkron olarak kalıcı olarak yazılması ve uygulama açılışında otomatik okunması sağlandı.
* **Yeni Not handler'ı:** Frontend tarafındaki "New Note" tetikleyicisi, Rust tarafındaki `create_note` IPC handler'ına bağlanarak benzersiz kimlikli ve otomatik isimlendirilen notların diskte oluşturulması sağlandı.
* **WebDAV Doğrulama ve Senkronizasyon:** Ayarlar kaydedilirken WebDAV bağlantısının aktif olarak test edilmesi ve manuel/otomatik senkronizasyon adımları arka planda işlenebilir hale getirildi.
* **Rust File Watcher:** Rust tarafında `notify` kütüphanesi entegre edilerek Vault dizini sürekli dinlemeye alındı. Harici bir dosya eklendiğinde, silindiğinde veya değiştirildiğinde Rust bunu yakalayıp frontend'e anında olay (event) fırlatır. Frontend bu olayı dinleyerek not listesini (`notesList`) anında ve akıcı bir şekilde günceller.

### Sonuç
Uygulama açılışında doğrudan aktif olan Vault ile başlamakta, harici dosya değişiklikleri anında senkronize bir şekilde arayüze yansımakta ve tüm ayarlar kalıcı olarak korunmaktadır.

---

## 2. Not Tarihleri (Invalid Date) ve Gelişmiş Etiket Yönetimi (Tag Manager)

### Karşılaşılan Problemler
* **Tarih Formatı Hatası (Invalid Date):** Not listesindeki her notun altında "Invalid Date" ibaresi yer alıyordu.
* **Etiket Ekleme Alanının Olmaması:** Arayüzde notların altında "No tag" yazmasına rağmen kullanıcıların etiket girebileceği, silebileceği veya yönetebileceği hiçbir arayüz bileşeni bulunmuyordu.

### Çözüm Yöntemi ve Güncellemeler
* **Tarih Hatalarının Çözümü:**
  - Rust backend katmanında not oluşturulma ve güncellenme zaman damgaları (`created_at`, `updated_at`) standart **RFC 3339 / ISO 8601** formatında serialize edilecek şekilde güncellendi.
  - Svelte tarafında bu tarihlerin JS `Date` motoru tarafından pürüzsüzce parse edilmesi sağlandı.
* **Etiket Yönetim Paneli (Tag Manager):**
  - Editörün hemen altında, durum çubuğunun (Status Bar) hemen üzerinde yer alan premium bir etiket yönetimi bileşeni geliştirildi.
  - **Akıllı Otomatik Tamamlama (Suggestions):** Kullanıcı etiket yazarken, diğer notlarda daha önceden kullanılmış olan tüm benzersiz etiketler taranarak anlık öneri listesi sunulur.
  - **Klavye Navigasyonu:** Öneri listesinde `ArrowDown`, `ArrowUp` tuşlarıyla gezinilebilir; `Enter`, `Tab`, `Virgül (,)` veya `Boşluk (Space)` tuşlarıyla etiketler hızlıca pill (hap) formatında eklenebilir.
  - **Etiket Silme:** Eklenen her etiketin yanındaki silme butonuyla etiketler nottan kolayca temizlenebilir.

### Sonuç
Notların oluşturulma/düzenlenme tarihleri hatasız olarak yüklenmekte ve Obsidian kalitesinde, klavye dostu, tam otomatik tamamlama destekli bir etiketleme deneyimi sunulmaktadır.

---

## 3. Tarih Biçimlendirmeleri ve Dosya Bazlı Durum Çubuğu (StatusBar)

### Karşılaşılan Problemler
* **Tarih Gösterimi:** Not listesinde tarihlerin ham ve okunaksız görünmesi.
* **Global Durum Gösterimi:** Durum çubuğunda yer alan "All changes saved" ve "Sync status" bilgilerinin, o an açık olan dosya yerine tüm uygulama bazında global verileri göstermesi.

### Çözüm Yöntemi ve Güncellemeler
* **Tarih Formatlama:** Not listesindeki son düzenleme zamanları Türkiye ve Avrupa standartlarına uygun olarak `DD.MM.YYYY HH.MM` (Örn: `22.05.2026 22.30`) formatına dönüştürüldü.
* **Dosya Bazlı Kayıt Durumu:** Durum çubuğundaki kayıt bilgisi global store yerine doğrudan aktif notun `updated_at` niteliğine bağlandı. Not üzerinde değişiklik yapıldığında durum anında `Unsaved` olarak güncellenir ve otomatik/manuel kaydetme tamamlandığında anlık göreli zaman formatına (`Just saved`, `Saved 10m ago`, `Saved 2h ago` vb.) dönüşür.
* **Görsel Sadeleştirme:** Kullanıcı talebi doğrultusunda durum çubuğunda kafa karıştıran "Never synced" gibi son senkronizasyon zamanı ve raporlama verileri tamamen kaldırılarak odaklanmış ve minimal bir tasarım elde edildi.

### Sonuç
Hatasız, okuması kolay tarih formatları ve sadece üzerinde çalışılan aktif dosyaya odaklanmış minimal bir durum çubuğu yapısı kuruldu.

---

## 4. macOS Pencere Boşluğu (macos-gap) Temizliği

### Karşılaşılan Problemler
* macOS platformunda çalışan uygulamalarda, pencerenin sol üst köşesindeki yerel işletim sistemi butonları (kapatma, simge durumuna küçültme, büyütme) için ayrılan yapay sol boşluk (`macos-gap`), pencere tasarımının simetrisini bozuyor ve gereksiz bir boşluk yaratıyordu.

### Çözüm Yöntemi ve Güncellemeler
* `Toolbar.svelte` bileşeni içerisindeki `.macos-gap` div yapısı tamamen kod tabanından kaldırıldı.
* macOS (Darwin) platformuna özel olarak yazılmış olan margin ve dolgu CSS kuralları temizlenerek arayüzün pencerenin en solundan itibaren doğal ve akıcı bir şekilde başlaması sağlandı.

### Sonuç
macOS üzerinde pencere kenarlarında hiçbir yapay boşluk kalmaksızın, pürüzsüz ve tam ekran simetrisine sahip premium bir başlık barı yerleşimi elde edildi.

---

## 5. Obsidian Tarzı Live Preview (WYSIWYG) ve Kararlılık Çözümü

### Karşılaşılan Problemler
* **Live Preview Modunun Aktifleşmemesi (Böcek Çözümü):**
  - Önceki aşamada yazılan `livePreviewPlugin` eklentisi, not geçişlerinde veya uygulamanın ilk açılışında `setState` çağrılarak CodeMirror durumu sıfırlandığı için çalışmıyordu. `editorView` referansı ve `viewMode` değişmediğinden compartment reconfigure edilemiyor ve Live Preview modu sürekli ham (raw) monospace editör görünümünde kalıyordu.
* **Estetik Yetersizlik:** Live Preview modu aktifleşmediğinden başlık boyutları, listeler ve bloklar ham markdown sembolleriyle çirkin ve eski bir HTML dosyası gibi görünüyordu.

### Çözüm Yöntemi ve Güncellemeler
* **Kararlı Bölme Entegrasyonu (Compartment Bug Fix):**
  - `getEditorExtensions` fonksiyonu, o anki `viewMode` değerini argüman olarak alacak şekilde güncellendi.
  - Bu sayede editör ilk defa mount edildiğinde veya not değiştirildiğinde (`setState` esnasında), bölme (`livePreviewCompartment`) doğrudan o anki aktif moda uygun uzantılarla kurulur. Modlar arası geçişler anında ve kayıpsız yansır.
* **Dinamik Yazı Tipi Yönetimi:**
  - `editorModeCompartment` eklenerek moda göre `.cm-mode-live` veya `.cm-mode-edit` sınıflarının CodeMirror kapsayıcısına otomatik basılması sağlandı.
  - Yüksek öncelikli CSS kurallarıyla:
    - **Live Preview (`live`)**: Editör fontu Reading View ile birebir aynı olan sans-serif **Apple System Fontu** (`var(--font-sans)`) ile ezilir.
    - **Raw Edit (`edit`)**: Yazılım odaklı **Monospace** (`var(--font-mono)`) fontu aktifleşir.
* **RangeSetBuilder Çökme Koruması (Stable Sorting):**
  - CodeMirror 6'nın sıralama kurallarını ihlal etmemek adına tüm dekorasyonlar önce bir dizide toplanır; başlangıç konumuna (ASC), aynı konumda iseler önce satır (line) dekorasyonlarına ve nesting kuralları gereği genişliğe (to DESC) göre sıralanıp tekilleştirilerek eklenir. Bu sayede hiçbir CodeMirror çökmesi yaşanmaz.
* **WYSIWYG Görsel Zenginleştirmeler (Reading View Kalitesinde):**
  - **Başlıklar (h1-h4):** Markdown `#` işaretleri aktif olmayan satırlarda gizlenir; başlık metinleri Reading View kalitesinde boyut, renk ve border-bottom çizgileriyle zenginleştirilir.
  - **Kalın/İtalik/Inline Code:** `**`, `*` ve `` ` `` sembolleri gizlenerek metinler doğrudan kalınlaştırılır, eğikleştirilir veya kutu içerisine alınır.
  - **Gelişmiş Bullet Listeler:** Listenin başındaki `-`, `*` veya `+` işaretleri gizlenerek, yerlerine `::before` pseudo elementi yardımıyla tam olarak tema renginde (`var(--accent)`) estetik yuvarlak bullet ikonları yerleştirilir.
  - **Numaralı Listeler:** Sayı belirteçleri şık bir accent rengine boyanır.
  - **BlockQuotes (Alıntılar):** Satır seviyesinde arka plan rengi ve sol kenarlık (`border-left`) eklenerek Reading View görüntüsü yakalanır.
  - **Yatay Çizgiler (Horizontal Rules):** Ham markdown yatay çizgileri gizlenerek yerine ince, şık bir bölücü çizgi çekilir.
  - **Kürsör Koruması (Obsidian Tarzı):** Kürsörün bulunduğu satırdaki tüm dekorasyonlar anlık olarak devre dışı kalır, böylece kullanıcı tıkladığı veya üzerinde olduğu satırı ham markdown olarak düzenleyebilir.
* **Odaklanmış Tek Panel Düzeni:**
  - Eski ikiye bölünmüş (split) ekran yapısı tamamen arındırıldı. Arayüzsegmented control aracılığıyla tekil odaklı panele geçirildi: Raw Markdown (Edit), Live Preview (WYSIWYG - Varsayılan) ve Reading View (Preview).

### Sonuç
Kürsörle üzerine gelindiğinde ham markdown moduna geçen, kürsör çıktığında ise tamamen Reading View estetiğinde sans-serif fontlu, şık bullet'lı, blok alıntılı ve yatay çizgili premium bir WYSIWYG düzenleme deneyimi elde edilmiştir.

---

## 6. Sürükle-Bırak, Sağ Tık Menüleri, Satır İçi İsimlendirme, Arayüz Düzenlemeleri ve Özel Not Modları (Çöp Kutusu & Çakışmalar)

### Karşılaşılan Problemler
* **Klasör ve Not İsimlendirme / Ekleme Hataları:**
  - Sağ tık menüsünden klasör ekleme ve isim değiştirme işlemleri çalışmıyordu. Not adı sağ tıkla değiştirilemiyordu.
  - İsim değişiklikleri çirkin modal pencereleriyle yapılıyordu. İsim değiştirildiğinde, Rust backend tarafında eski klasör yenisinin içine taşınmak yerine içiçe geçerek ağaç yapısını bozuyordu (Örn: `ilk folder` -> `ikinci folder/ilk folder` oluyordu).
  - Klasör ismini değiştirdikten sonra `active.file_path.starts_with is not a function` hatası alınıyordu.
  - Klasör ve not ekleme butonları seçili klasör yerine her zaman kök dizine (root) işlem yapıyordu.
* **Sağ Tık Menüsü Z-Index ve Tetiklenme Sorunları:**
  - Sağ tık menüsü `note-list-panel` alanının arkasında kalıyor ve not listesi üzerinde sağ tık menüsü hiç tetiklenmiyordu.
* **Sürükle-Bırak (Drag & Drop) Sorunları:**
  - Notlar sürüklenebiliyor ancak bir klasörün içine veya dışına bırakılamıyordu; sadece sürükleme animasyonu gösteriliyordu.
* **Svelte 5 Runes Geçiş Hataları:**
  - `NoteList.svelte` dosyası derlenirken runes modunda reaktif `$:` kullanımına izin verilmediği için (`$: notes = $notesList`) uygulama açılışta Vite derleme hatası vererek çöküyordu.
* **Arayüz Kalabalığı ve Arama Widget'ı Engeli:**
  - Eski `sidebar-header` ve sekmeler (`notes`/`trash`) gereksiz kalabalık yaratıyordu.
  - Arama paneli (`search-widget`) editör alanının arkasında kalıyor ve z-index/transform kısıtlamaları yüzünden macOS WKWebView üzerinde akıcı açılmıyordu.
* **Çöp Kutusu ve Çakışmaların Yanlış Yapılandırılması:**
  - Silinen notlar ve senkronizasyon çakışmaları için sidebar'da açılır-kapanır akordeon yapısı tercih edilmiyordu.
  - Silinen notlar sayacında 8 adet not görünmesine rağmen, alt klasörlerde bulunan silinmiş notlar listelenmiyordu.

### Çözüm Yöntemi ve Güncellemeler
* **Satır İçi (Inline) İsimlendirme ve Hata Giderme:**
  - Modal pencereleri tamamen kaldırılarak, yeni klasör oluşturulduğunda veya isim değiştirildiğinde doğrudan ağaç üzerinde odaklanmış (focus) şık bir satır içi `input` alanı açıldı.
  - Klasör taşıma/yeniden adlandırma mantığı düzeltilerek içiçe klasör oluşturma problemi ve `active.file_path.starts_with` hataları tamamen giderildi.
  - Klasör ekleme ve yeni not oluşturma butonları, o an seçili olan aktif klasöre dinamik olarak alt eleman ekleyecek şekilde güncellendi.
* **Sağ Tık Menüsü Entegrasyonu:**
  - Sağ tık menüsü katmanlama (`z-index`) değeri artırılarak öne çıkarıldı. `note-list-panel` ve not elemanlarına özel context menu handler'ları eklenerek menünün her yerde doğru tetiklenmesi sağlandı.
* **Sürükle-Bırak (Drag & Drop) Kararlılığı:**
  - Rust backend dosya taşıma komutları ve frontend drop handler'ları senkronize edilerek notların klasör içine/dışına taşınması tam olarak çalışır hale getirildi.
* **Svelte 5 Runes Moduna Uyum:**
  - `NoteList.svelte` içerisindeki tüm eski `$:` ifadeleri temizlenerek modern `$derived` ve `$effect` yapılarına dönüştürüldü ve derleme hatası çözüldü.
* **Spotlight Arama ve Sidebar/Toolbar Yenilikleri:**
  - `sidebar-header` tamamen kaldırıldı; Noda logosu ve marka ismi sol üstteki `vault-pill` yanına taşındı. macOS Darwin platformu için native buton boşluğu (`padding-left: 80px`) bırakıldı.
  - Arama widget'ı yerel arayüzden çıkarılarak global bir `<SearchModal />` bileşeni haline getirildi ve DOM'un en kökünde (`+page.svelte`) konumlandırıldı. Svelte'in donanım ivmeli `fade` ve `scale` geçişleriyle pürüzsüz Spotlight deneyimi sunuldu.
* **Özel Not Modları ve Birleşik Yönetim:**
  - Sidebar'ın altına "Management & Settings" paneli yerleştirildi.
  - Silinen Notlar veya Çakışmalar tıklandığında, elemanlar sol taraftaki `note-list-panel` alanında listelenir. Not tıklandığında ise editörde salt okunur modda açılır.
  - Editörün üst kısmındaki başlık barına (`note-titlebar`) özel durum şeritleri (banner) eklendi:
    - **Çöp Kutusu Modu:** Salt okunur şeridi ile birlikte "Geri Yükle" (`recoverFromTrash`) ve "Kalıcı Olarak Sil" (`emptyTrashPermanently`) butonları.
    - **Çakışma Modu:** "Çakışan Sürüm" uyarısıyla "Yerel Sürümü Tut" veya "Sunucu Sürümünü Tut" butonları.
  - Rust tarafında `list_trash` komutu, alt klasörleri de derinlemesine tarayarak tüm silinmiş notları eksiksiz listeleyecek şekilde güncellendi.

### Sonuç
Sağ tık ve sürükle-bırak işlemleri Rust ve modern frontend entegrasyonuyla mükemmel çalışan, modal pencerelerden arındırılmış satır içi isimlendirme sunan, Svelte 5 runes mimarisiyle kararlı, macOS standardında Spotlight aramalı ve çöp kutusu/çakışma notlarını özel banner'larla yönetebilen kusursuz ve premium bir uygulama arayüzü elde edilmiştir.

---

## 7. Arka Plan Senkronizasyon Bildirimleri, Derleme Uyarıları ve Svelte 5/TypeScript Kararlılık Güncellemeleri

### Karşılaşılan Problemler
* **Arka Plan Senkronizasyon Bildirimlerinin Eksikliği:** Manuel senkronizasyon (sync) işlemi tamamlandığında kullanıcıya başarılı bir şekilde özet bildirim (sync-toast) kutusu gösterilirken, zamanlanmış (auto/scheduled) arka plan senkronizasyonu tamamlandığında kullanıcıya hiçbir bildirim sunulmuyordu. Bu durum, arka planda hangi dosyaların yüklendiği, indirildiği veya çakışmaların oluşup oluşmadığı konusunda kullanıcının bilgisiz kalmasına yol açıyordu.
* **Svelte 5 ve TypeScript Derleme Uyarıları:** Vite ve `svelte-check` derleme esnasında şu uyarılarda bulunuyordu:
  - `Sidebar.svelte` ve `Editor.svelte` içinde artık kullanılmayan eski akordeon, sağ tık menüsü ve dikey bölücü (`.panel-divider`) CSS seçicileri uyarı üretiyordu.
  - `SearchModal.svelte` içerisinde `bind:this` ile bağlanan `inputEl` değişkeni reaktif olarak beyan edilmediğinden Svelte 5 uyarı veriyordu.
  - `FolderTreeItem.svelte` ve `NoteListItem.svelte` içerisinde `$props` üzerinden doğrudan kopyalanan `renameValue` durumu, başlangıç değerini sabitlediği için reaktif prop kopyalama uyarısı üretiyordu.
  - `FolderTreeItem.svelte` bileşeninin alt klasörleri çizerken kullandığı `<svelte:self>` yapısı Svelte 5'te eskimiş (deprecated) durumdaydı.
  - Sürükle-bırak drop handler'ında, global `draggedItem` store'undan okunan opsiyonel `data.relPath` değeri ve `.pop()` metodundan dönen opsiyonel `folderName` değeri nedeniyle `moveFolder` fonksiyonuna geçirilen parametrelerde TypeScript tür uyumsuzluğu hatası oluşuyordu.

### Çözüm Yöntemi ve Güncellemeler
* **Rust-First Senkronizasyon Olay Yapısı:**
  - **Rust Core (`crates/core`)**: UI-agnostic (arayüzden bağımsız) `SyncEngine` yapısına thread-safe `sync_finished_callback` callback alanı ve bunu ayarlayacak olan `set_sync_finished_callback` fonksiyonu eklendi. Arka plan veya manuel senkronizasyon başarıyla tamamlandığında bu callback tetiklenerek elde edilen `SyncReport` verisi gönderildi.
  - **Tauri Olay Köprüsü (`tauri-shell`)**: `AppState::init_vault` içerisinde `SyncEngine`'in `sync_finished_callback`'ine kanca (hook) atılarak, her başarılı senkronizasyon sonrası tauri `sync_finished` adlı özel olay (event) üzerinden `SyncReport` verisi frontend'e fırlatıldı (`emit_sync_finished`).
  - **Dumb Arayüz / Svelte Entegrasyonu**: Frontend'de `events.ts` içerisine `listenToSyncFinished` event dinleyicisi eklendi. `+layout.svelte` altında bu dinleyici onMount aşamasında kaydedildi. Event yakalandığında senkronizasyon raporu zenginleştirilip (`uploads + downloads + deletes...` hesaplanarak `total` alanı çıkarıldı ve o anki zaman damgası `completed_at` olarak eklendi), `lastSyncReport` ve `showSyncReport` store'larına set edilerek görsel bildirim kutusunun (toast) otomatik açılması sağlandı. `onDestroy` altında ise abonelik temizlendi.
* **Vite & TypeScript Uyarılarının Giderilmesi:**
  - **Unused CSS Selectors**: `Sidebar.svelte` içindeki kullanılmayan eski `.accordion-trigger`, `.accordion-content` gibi stiller ve sağ tık menüsü CSS kuralları ile `Editor.svelte` içindeki `.panel-divider` kuralları temizlendi.
  - **Reactive State Update**: `SearchModal.svelte` içerisinde elemente bağlanan `inputEl` değişkeni, Svelte 5 runes standartlarına uygun şekilde reaktif `$state<HTMLInputElement>()` olarak tanımlandı.
  - **Prop Reference State Capture**: `FolderTreeItem.svelte` ve `NoteListItem.svelte` dosyalarında props üzerinden doğrudan kopyalanan `renameValue` durumu, başlangıçta `$state('')` olarak tanımlanıp sadece yeniden adlandırma tetiklendiğinde `$effect` bloğu içerisinde güncellenecek şekilde revize edilerek uyarılar giderildi.
  - **svelte:self Deprecation**: `FolderTreeItem.svelte` bileşeninin alt klasörleri çizerken kullandığı eski `<svelte:self>` yapısı, Svelte 5 standartlarına uygun self-import (`import FolderTreeItem from './FolderTreeItem.svelte'`) ve `<FolderTreeItem>` etiketiyle değiştirildi.
  - **TypeScript Type Narrowing**: `FolderTreeItem.svelte` içerisindeki drop handler'ında, opsiyonel olan `data.relPath` değeri yerel bir sabit olan `const draggedPath` değişkenine atanıp kontrol edilerek `string` türüne güvenli şekilde daraltıldı. Ayrıca `.pop() || ''` fallback yapısı eklenerek `newPath` değerinin tür uyuşmazlığı giderildi.

### Sonuç
Arka plan otomatik senkronizasyon işlemi bittiğinde de tıpkı manuel sync gibi premium özet bildirimlerin anlık görüntülenmesi sağlanmıştır. Ayrıca Svelte 5 ve TypeScript uyumluluğu %100 kararlı hale getirilerek tüm derleme uyarıları ve tip hataları giderilmiş, temiz ve uyarı vermeyen bir kod tabanı elde edilmiştir.

---

## 8. Senkronizasyon Çakışmalarını Yan Yana Karşılaştırma, Otomatik Klasör Senkronizasyonu, Çift Tıklama Desteği ve Dinamik Arayüz Güncellemeleri

### Karşılaşılan Problemler
* **Çakışma Karşılaştırma Ekranının Olmaması:** Senkronizasyon esnasında çakışan (conflict) bir not seçildiğinde, kullanıcının yerel sürüm ile sunucudan gelen uzak sürümün içeriklerini, başlıklarını, düzenlenme zamanlarını ve etiketlerini yan yana inceleyip hangisini saklayacağına karar verebileceği premium bir arayüz bulunmuyordu.
* **Klasör ve Not Seçimi Tutarsızlığı:** Sol taraftaki klasör ağacından bir nota tıklandığında, orta kısımda yer alan not listesi paneli otomatik olarak o notun ait olduğu klasöre geçiş yapmıyordu. Ayrıca silinmiş notlar veya çakışmalar listelenirken bir normal klasöre tıklandığında çöp kutusu/çakışma görünümünden çıkılamıyordu.
* **Derinlemesine Gizlenen Seçili Notlar:** Arama motorundan veya "Tüm Notlar" listesinden çok derindeki bir not (örneğin `folder1/folder2/folder3/folder4/kimya.md`) seçildiğinde, sol klasör ağacında ilgili dallar otomatik genişlemediğinden seçili not ağaçta gizli kalıyordu.
* **Klasörleri Genişletme Zorluğu:** Kullanıcıların klasörleri açıp kapatmak için sadece küçük chevron ok butonlarına tıklamak zorunda kalması kullanımı zorlaştırıyordu.
* **Görsel Tasarım Yetersizlikleri (Ağaç Görünümü):**
  - Klasör ağacı satırlarının hover arka planları sadece metin ve ikon etrafını sarıyor, VS Code'daki gibi tüm satır boyunca uzanmıyordu.
  - Klasör girinti çizgileri (indent guides) statik kenar boşluklarıyla yapıldığından satırların tam genişliğe yayılması engelleniyordu.
  - "All Notes" butonu ile "Ana Dizin" başlıklarında kasa ismi yerine statik yazılar yer alıyordu.

### Çözüm Yöntemi ve Güncellemeler
* **Yan Yana Çakışma Karşılaştırma Paneli (Sync Conflict Compare UI):**
  - `Editor.svelte` üzerinde çakışma durumunda salt okunur bölünmüş karşılaştırma ekranı geliştirildi.
  - **Yerel Sürüm (Sol):** HSL Mavi renginde yerel başlık, değiştirilme tarihi, etiket hapları, markdown önizlemesi ve "Keep Local Version" butonu.
  - **Uzak Sürüm (Sağ):** HSL Turuncu renginde uzak başlık, değiştirilme tarihi, etiket hapları, markdown önizlemesi ve "Use Remote Version" butonu.
  - Çakışma çözümü ekranında görsel karmaşayı azaltmak için alt etiket yöneticisi ve durum çubuğu geçici olarak gizlendi. Uzak sürüm verileri `.noda/conflicts/` dizininden asenkron `ipc.getConflictNote` aracılığıyla çekildi.
* **Otomatik Klasör Senkronizasyonu (`FolderTreeItem.svelte` & `NoteList.svelte`):**
  - Klasör ağacında bir nota tıklandığında, notun dosya yolundan üst klasörü (`parentFolder`) hesaplanarak `selectedFolder` store'una atandı. Böylece orta panel otomatik o klasörün içeriğini listelemeye başladı.
  - Bir klasör tıklandığında da görünüm modunun çöp kutusu/çakışma modundan normal moda geçmesi için `activeViewMode` değeri otomatik `'normal'` yapıldı.
* **Reaktif Akıllı Ağaç Genişletme ($effect Entegrasyonu):**
  - `Sidebar.svelte` bileşenine eklenen `$effect` kancası ile aktif notun (`$activeNote`) dosya yolu reaktif olarak dinlenmeye alındı. Not değiştiğinde, en derindeki notun bile tüm üst klasör yolları parçalanarak `expandedFolders` nesnesinde `true` yapıldı; böylece ağaçtaki ilgili dal otomatik genişletildi.
* **Çift Tıklama (Double-Click) Desteği:**
  - Klasör satırlarına `ondblclick={handleRowDblClick}` olayı eklenerek, satırın herhangi bir yerine çift tıklandığında klasörün anında genişletilmesi veya daraltılması sağlandı.
* **VS Code Tarzı Full-Bleed Görsel Düzenlemeler:**
  - Kapsayıcı `.folder-tree` bileşenine `-8px` negatif kenar marjı verilerek satırların **sidebar sınırlarına kadar tam genişlikte** uzanması sağlandı.
  - Satır köşeleri (`border-radius: 0`) sıfırlandı ve seçilen satırın soluna `3px` kalınlığında aktiflik vurgusu (`border-left`) yerleştirildi.
  - Dikey girinti çizgileri (`.indent-guide`) her satırın derinliğine göre mutlak konumlandırılmış şık `<span>` elemanlarına dönüştürüldü ve klasör bölümünün üzerine gelindiğinde yumuşak bir şekilde aydınlanacak (`transition: border-color`) şekilde ayarlandı.
* **Dinamik Kasa (Vault) İsmi Gösterimi:**
  - Ana dizindeki notlar listelenirken üst başlıkta "All Notes" yerine doğrudan kasanın gerçek adı (`info.name`) gösterildi.
  - Klasör ağacı başlığındaki statik "Folders" yazısı da reaktif olarak kasanın adıyla (`info?.name || 'Folders'`) güncellendi.

### Sonuç
Klasör ağacında VS Code kalitesinde tam genişlikte hover efektleri, dinamik kılavuz çizgileri, çift tıklama desteği, seçilen nota göre otomatik ağaç dallarının genişlemesi, arayüzde dinamik kasa ismi kullanımı ve çakışan notları pürüzsüzce yan yana karşılaştırıp tek tuşla çözen premium ve üst seviye bir kullanıcı deneyimi elde edilmiştir.


---

## 9. Boş Senkronizasyon Geçmişi için Akıllı Çözüm (Newer Wins) ve Rapor Geliştirmeleri

### Karşılaşılan Problemler
* **Cache Silinmesi Sonrası Toplu Çakışmalar:**
  - Ayarlar altındaki **"Clear Remote Tracking Cache"** seçeneği uygulanıp `remote_state.json` (senkronizasyon geçmişi baseline referansı) temizlendiğinde, bir sonraki senkronizasyon döngüsünde yerel ile sunucudaki tüm notlar doğrudan **Conflict (Çakışma)** listesine düşüyordu.
  - **Sebebi:** WebDAV sunucuları dosya yükleme esnasında yerel dosyanın değiştirilme tarihini korumaz; kendi sunucu saatine göre yeni bir `last_modified` zamanı atar. `remote_state.json` silindiğinde algoritma geçmiş referansı (`previous_state`) bulamaz. Yereldeki notun `updated_at` değeri sunucunun `last_modified` zamanıyla (içerik aynı olsa dahi) uyuşmadığı için, üç yollu senkronizasyon motoru veri güvenliğini korumak amacıyla her dosyayı manuel çakışma (Conflict) moduna zorluyordu. Bu da 2.000 notu olan bir kullanıcının 2.000 adet yapay çakışmayla karşılaşmasına neden oluyordu.
* **Senkronizasyon Raporunda Kriptik Dosya Adları:**
  - Senkronizasyon işlemi tamamlandıktan sonra açılan **Sync Report** detay modalında, işlem gören notlar diskteki ham isimleriyle (Örn: `01KSE2D8T9VT8F72XQ9HAE98Y1.md` veya `.noda/history/01KSE2D8T9VT8F72XQ9HAE98Y1/20260525_004838_967.md`) listeleniyordu. Kullanıcının hangi notun yüklendiğini veya indirildiğini bu ULID kodlarından anlaması imkansızdı.

### Çözüm Yöntemi ve Güncellemeler
* **Boş Geçmiş Durumu için "Newer Wins" Entegrasyonu:**
  - `crates/core/src/sync/delta.rs` dosyasında, dosyanın her iki tarafta da bulunduğu ama geçmiş verinin olmadığı `(Some(local_note), Some(remote_entry), None)` durumu yeniden yapılandırıldı.
  - **5 Saniye Güvenlik Aralığı (Clock Drift Buffer):** Yerel notun güncellenme zamanı (`local_note.updated_at`) ile sunucu zamanı (`remote_lm`) arasında karşılaştırma yapıldı:
    - Yerel dosya sunucudakinden **5 saniyeden daha yeniyse**: Otomatik olarak sunucuya yükleme aksiyonu üretildi (**Upload**).
    - Sunucudaki dosya yereldekinden **5 saniyeden daha yeniyse**: Otomatik olarak yereli güncelleme aksiyonu üretildi (**Download**).
    - Eğer zaman damgaları birbirine **5 saniye veya daha yakınsa** ama boyutları/içerikleri farklıysa: Mutlak veri güvenliği için dosya manuel karar verilmek üzere **Conflict** olarak işaretlendi.
    - Tarih parse edilemediğinde veya sunucudan tarih gelmediğinde: Güvenli liman olarak yine **Conflict** tercih edildi.
  - Değişiklikleri doğrulamak için `delta.rs` içerisine 3 adet kapsamlı birim test (`test_calculate_delta_empty_previous_local_newer`, `test_calculate_delta_empty_previous_remote_newer`, `test_calculate_delta_empty_previous_close_timestamps`) eklenerek `cargo test` ile tüm testlerin (37 adet test) pürüzsüz geçtiği doğrulandı.
* **Sync Raporunda Dinamik Not Başlığı Gösterimi:**
  - `frontend/src/lib/components/sync/SyncReportModal.svelte` içerisine frontend'deki `notesList` store'u import edildi.
  - Dosya yollarından düzenli ifadeler (regex) yardımıyla 26 karakterli **ULID (Not ID)** değerini ayıklayan `getDisplayName` yardımcı fonksiyonu yazıldı.
  - Bu fonksiyon, ayıklanan ID'yi yerel not listesinde arayarak eşleşen notun gerçek başlığını bulur:
    - Normal notlar için: `01KSE2D8T9VT8F72XQ9HAE98Y1.md (Not Başlığı)`
    - Geçmiş sürümleri için: `.noda/history/01KSE2D8T9VT8F72XQ9HAE98Y1/20260525_004838_967.md (Not Başlığı - Geçmiş)`
    - Bulunamayan notlar veya ek dosyalar (attachments) için ise ham dosya adını bozmadan geri döndürür.
  - Modal üzerindeki tüm işlem listeleri (`uploaded_files`, `downloaded_files`, `conflict_files` vb.) bu yardımcı fonksiyonla ekranda görselleştirildi.

### Sonuç
Önbelleğin temizlenmesi veya ilk kurulum senkronizasyonu durumunda binlerce yapay çakışmanın önüne geçilerek, güncel olan dosyaların otomatik olarak senkronize edilmesi ve sadece kritik zaman dilimindekilerin kullanıcıya sunulması sağlanmıştır. Ayrıca senkronizasyon raporundaki anlamsız dosya adları yerine not başlıklarının parantez içinde dinamik olarak gösterilmesiyle son derece profesyonel, anlaşılır ve premium bir senkronizasyon özeti deneyimi elde edilmiştir.

---

## 10. Versiyon Geçmişi için Snapshot Silme Özelliği, Git Mantığıyla Bağlamsal Diff (Contextual Diff) ve Güvenli Snapshot Geri Yükleme (Safe Restore) Entegrasyonu

### Karşılaşılan Problemler
* **Snapshot Silme Özelliğinin Olmaması:** Versiyon geçmişi panelinde listelenen eski snapshot'ları manuel olarak silebilme seçeneği bulunmuyordu. Bu durum diskte gereksiz sürüm dosyalarının birikmesine yol açıyordu.
* **Tüm Notu Karşılaştıran Hantal Diff Ekranı:** Geçmiş sürümlerin diff paneli (karşılaştırma modalı) tüm notu baştan sona karşılaştırıyordu. Bu durum çok uzun notlarda aradaki ufak tefek değişiklikleri bulmayı zorlaştırıyor ve gereksiz görsel kalabalık yaratıyordu.
* **Geri Yüklemede Zaman Yolculuğu Hatası (`updated_at` & `parent_id`):** Snapshot geri yüklendiğinde eski YAML frontmatter üst bilgisi olduğu gibi kopyalanıyordu. Özellikle eski `updated_at` değerinin notun üzerine aynen yazılması, WebDAV delta sync senkronizasyon motorunun kafasını karıştırıyor ve sunucudaki (WebDAV) dosyaların, geri yüklenen nottan daha yeni sanılarak yereldeki restorasyonun üzerine sessizce yazılmasına (ezilmesine) yol açıyordu. Ayrıca eski `parent_id` değeri notun güncel klasör konumunu ezerek klasör yapısını bozuyordu.
* **Editörün Anlık Güncellenmemesi (Reaktivite Hatası):** Bir snapshot geri yüklendiğinde dosya diskte değişiyor fakat editör alanı (CodeMirror) anlık güncellenmiyordu; içeriğin yansıması için başka nota geçip geri dönmek gerekiyordu.

### Çözüm Yöntemi ve Güncellemeler
* **Snapshot Silme Özelliği (Delete Snapshot):**
  - **Rust Core (`crates/core`)**: `storage.rs` ve `mod.rs` dosyalarında, milisaniye duyarlılığında (`timestamp_millis()`) asenkron bir `delete_snapshot` fonksiyonu yazıldı. Bu işlev Tauri komut katmanına (`history_commands.rs` ve `main.rs`) taşındı.
  - **Frontend Arayüzü**: Arayüze (`Editor.svelte`) üzerine gelindiğinde hafifçe büyüyen ve şık bir HSL kırmızı rengine dönen çöpe atma butonu eklenerek store (`editor.ts`) üzerinden dinamik silme ve liste yenileme sağlandı.
* **Git Mantığıyla Bağlamsal Diff (Contextual Diff):**
  - **Rust Core (`crates/core`)**: Rust tarafında `similar::TextDiff` yapısındaki `.grouped_ops(3)` metodu entegre edildi. Bu sayede değişen kısımların öncesi ve sonrasındaki 3'er satırlık bağlam (context) satırı korunurken, değişmeyen uzun kısımlar atlandı. Atlanan kısımların arasına Rust tarafında özel bir `"Separator"` DiffChunk'ı eklenerek, Svelte tarafında (`DiffViewer.svelte`) bu bölücüler kesikli çizgiler ve yatay ayraçlarla (`...`) Obsidian/GitHub kalitesinde görselleştirildi.
* **Güvenli ve Standartlara Uygun Snapshot Geri Yükleme (Safe Restore & Time Update):**
  - **Tauri Shell (`crates/tauri-shell`)**: `restore_snapshot` Tauri komutu tamamen baştan tasarlandı. Snapshot dosyasından sadece **gövde (body)** ve **başlık (title)** alanları ayrıştırıldı (parsed).
  - **Metaveri Koruma**: Notun şu anki klasörü (`parent_id`), etiketleri (`tags`), rengi (`color`) ve oluşturulma tarihi (`created_at`) korunarak geçmişten gelen gövdeyle birleştirildi.
  - **Sync-Safe Güncelleme**: `updated_at` değeri **`Utc::now()` (şu anki zaman)** olarak güncellenerek senkronizasyon motorunun bu işlemi "en yeni değişiklik" olarak algılaması sağlandı.
* **Editör Reaktivite Sorununun Çözülmesi (Instant Content Sync):**
  - **Editor UI (`Editor.svelte`)**: Svelte reaktif takip bloğu (`$: if (currentNote)`) güncellenerek, not ID'si değişmese dahi içeriğin (body) dışarıdan değiştiği ve notun kirli (dirty) olmadığı durumlar (`editorView.state.doc.toString() !== currentNote.body && !isDirty`) dinlemeye alındı. Bu durumda CodeMirror durumu anında yeniden kurularak editör ekranının restorasyon sonrasında anlık yenilenmesi sağlandı.

### Sonuç
Versiyon geçmişinde gereksiz sürüm birikiminin silinmesi sağlanmış, Obsidian/GitHub kalitesinde son derece okunaklı, sadece değişen kısımlara odaklanan bağlamsal diff yapısı kurulmuştur. En önemlisi, snapshot geri yüklemelerinde klasör ve senkronizasyon yapısını bozmayan asıl endüstri standardı olan güvenli metaveri korumalı geri yükleme modeli Rust çekirdeğine entegre edilerek reaktivite kararlılığıyla mükemmelleştirilmiştir.

---

## 11. Delta Senkronizasyonunda Klasör Yapısı Koruma ve Veritabanı Cache Sağlamlaştırması (Self-Healing)

### Karşılaşılan Problemler
* **Alt Klasör Eşleşmeme Hatası:** `remote_state.json` silindikten veya önbellek temizlendikten sonra yapılan ilk senkronizasyonda, sunucudaki (WebDAV) tüm notlar yerel alt klasör yapıları yerine doğrudan ana (root) dizine çekiliyordu.
  - **Nedeni:** `crates/core/src/sync/delta.rs` içindeki delta hesaplama algoritmasında `local_map` haritalanırken yerel notlar gerçek disk yolları (`n.file_path`) yerine statik `{id}.md` formatı üzerinden eşleştiriliyordu. Bu durum, alt klasördeki notların fark edilemeyip sıfırdan ana dizine indirilmesine sebep oluyordu.
* **Geçersiz ULID Hataları:** Not yükleme (`Upload`) esnasında not ID'si `relative_path.trim_end_matches(".md")` ile ayıklanıyordu. Dosya bir klasör içerisindeyse (Örn: `work/01KS.md`) bu metot `work/01KS` sonucunu veriyor ve geçersiz ULID formatı nedeniyle senkronizasyon motorunun hata vermesine yol açıyordu.
* **Veritabanı Yeniden Yapılandırma Çökmesi:** Disk üzerinde aynı not ID'sine sahip mükerrer dosyalar bulunduğunda, veritabanı önbelleğini sıfırdan oluşturan `Rebuild Cache` işlevi SQLite'ın `UNIQUE constraint failed: notes.id` hatasıyla yarıda kesilip çöküyordu.

### Çözüm Yöntemi ve Güncellemeler
* **Klasör Duyarlı Haritalama:**
  - `delta.rs` dosyasındaki `calculate_delta` fonksiyonunda `local_map` anahtarı ham ID yerine notun gerçek `n.file_path` değeri olacak şekilde güncellendi.
  - `engine.rs` dosyasında dosya yolundan bağımsız olarak sadece dosya adını (ULID) güvenli ayıklamak için `std::path::Path::new(relative_path).file_stem()` metodu entegre edildi.
  - `conflict.rs` üzerinde çakışan notların alt klasör konumlarını korumak için `VaultService::find_note_path` yardımıyla dinamik yol çözümlemesi yapıldı.
* **SQLite Self-Healing (Kendi Kendine İyileşme):**
  - `crates/core/src/database/rebuild.rs` içerisindeki veritabanı önbellek yenileme döngüsünde `insert_note` yerine `upsert_note` kullanıldı. Bu sayede diskte aynı ID'ye sahip birden fazla dosya olsa dahi indeks çökmeden işlem tamamlanır ve sistem kendi kendini onarır.

### Sonuç
Eşitleme önbelleği temizlense dahi notların alt klasör hiyerarşisi %100 korunarak senkronize edilmesi sağlanmış, ULID ayrıştırma hataları giderilmiş ve veritabanı indeks yenileme işlemi mükerrer dosya durumlarında bile çökmeyecek şekilde son derece dayanıklı hale getirilmiştir.

---

## 12. Mükerrer (Duplicate) Not Teşhisi, Önizleme ve Çözüm Paneli

### Karşılaşılan Problemler
* Sürükle-bırak yöntemiyle veya senkronizasyon hatalarıyla Vault klasörleri içerisine aynı Note ID'ye sahip mükerrer dosya kopyaları atıldığında, kullanıcının bunları tespit edip hangisinin güncel veya doğru sürüm olduğunu anlayarak silebileceği bir mekanizma yoktu.

### Çözüm Yöntemi ve Güncellemeler
* **Mükerrer Taraması (Rust Core):**
  - `crates/core/src/diagnostics/mod.rs` dosyasına `DuplicateNoteGroup` ve `DuplicateFileEntry` yapıları eklenerek tüm Vault klasörünü derinlemesine tarayıp aynı Note ID frontmatter'ına sahip mükerrer dosyaları (boyut ve değiştirilme tarihleriyle birlikte) gruplayan `get_duplicate_notes` fonksiyonu yazıldı.
  - Seçilen kopyayı diskten ve veritabanı indeksinden güvenli bir şekilde silen `delete_duplicate_note_file` işlevi kodlandı.
* **Frontend Entegrasyonu ve Önizleme Çekmecesi (Preview Drawer):**
  - `SettingsModal.svelte` içerisindeki **Maintenance** sekmesine "Duplicate Notes Diagnostics" yönetim paneli eklendi.
  - Kullanıcı mükerrer notları taradığında, dosyaların yanındaki yollara tıklayarak glassmorphic bir **Önizleme Çekmecesinde** notun ham gövde içeriğini anlık olarak inceleyebilir (böylece hangi kopyayı sileceğine karar verebilir). Silme butonuna basıldığında diskten kopya silinir ve önbellekler anında güncellenir.
* **Mükerrer Yoksa Boş Durum Kartı (Empty State Card):**
  - Mükerrer dosya bulunmadığında arayüzün bomboş kalması yerine, ek dosyaları temizleme arayüzü ile uyumlu estetik bir başarı kartı (`🎉 Harika! Vault klasöründe hiçbir mükerrer not bulunamadı.`) gösterilmesi sağlandı.

### Sonuç
Vault içerisindeki mükerrer dosyaların taranması, içeriklerinin yan yana güvenle önizlenmesi ve istenmeyen kopyaların tek tıkla sistem kararlılığını bozmadan temizlenmesi sağlanmıştır.

---

## 13. Sahipsiz Versiyon ve Çakışma Kalıntıları (Orphaned Remnants) Teşhisi ve Temizleme

### Karşılaşılan Problemler
* Notlar silindikten sonra diskin derinliklerinde (`.noda/history/` ve `.noda/conflicts/` dizinlerinde) o notlara ait eski geçmiş sürümler (history snapshots) ve çakışma (conflict) dosyaları kalıyordu ve zamanla gereksiz disk alanı işgal ediyordu.

### Çözüm Yöntemi ve Güncellemeler
* **Akıllı Kalıntı Analizi (Rust Core):**
  - `crates/core/src/diagnostics/mod.rs` içerisine `get_orphaned_remnants` ve `delete_orphaned_remnants` fonksiyonları yazıldı. Bu işlevler Vault'taki tüm aktif notların ID'leri ile çöp kutusundaki not ID'lerini tarar, diske yazılmış ama bu ID'ler ile hala eşleşmeyen (sahipsiz kalmış) tüm `.noda/history/{ID}/` ve `.noda/conflicts/{ID}_*.md` dosyalarını asenkron olarak bulur ve temizler.
* **Arayüz ve Yönetim (Frontend):**
  - `SettingsModal.svelte` bakım sekmesi (Maintenance) tamamen premium bir düzene geçirildi. Üç ana temizlik grubu oluşturuldu: "Database Administration", "Vault Diagnostics & Storage Cleanup" ve "Synchronization Self-Healing".
  - "Vault Diagnostics & Storage Cleanup" grubu altına şık ikonlu kartlar olarak **Scan Orphaned Attachments**, **Scan Duplicate Notes** ve **Scan Orphaned Remnants** butonları eklendi.
  - Sahipsiz kalıntılar tarandığında kazanılacak alan, dosya listesi (ikonlarıyla ve değiştirilme tarihleriyle) gösterilmektedir. Kullanıcı tek tek veya toplu olarak bu kalıntı dosyalarını temizleyebilir, yola tıklayarak aynı önizleme çekmecesinde kalıntı dosyasının içeriğini inceleyebilir.

### Sonuç
Vault'ta artık var olmayan notlara ait eski geçmiş ve çakışma dosyalarının arkalarında bıraktığı çöpler güvenle taranıp silinmekte ve yerel depolama alanları maksimum verimlilikle geri kazanılmaktadır.

---

## 14. Detaylı Not Bilgisi (Info Popover), Formatlama Araç Çubuğu ve Gelişmiş Eklenti (Attachment) Entegrasyonları

### Karşılaşılan Problemler
* **Bilgi Eksikliği:** O an açık olan notun boyutu, kelime/karakter sayıları, oluşturulma/düzenlenme tarihleri, bulut yüklenme zamanı (senkronizasyon durumu) ve geçmiş sürüm sayısı gibi meta verilere editör içerisinden hızlıca ulaşılamıyordu.
* **Formatlama Kısayollarının Olmaması:** Markdown biçimlendirmelerini hızlıca uygulamak için bir araç çubuğu bulunmuyordu.
* **Eklenti Yönetimi Zorluğu:** Notun içine eklenti (görsel, PDF vb.) ekleme, sürükleme ve son eklenen dosyaları nota hızlıca yerleştirme süreçleri hantaldı.

### Çözüm Yöntemi ve Güncellemeler
* **Detaylı Bilgi Balonu (Note Info Popover):**
  - Rust tarafında asenkron `get_note_metadata` Tauri komutu kodlanarak nota dair tüm meta verilerin (`NoteMetadataDto`) toplanması sağlandı.
  - `Editor.svelte` üzerinde başlık barına şık bir **Info** butonu eklendi. Buton; başlık, dosya adı, bağıl/mutlak disk yolları, oluşturulma/düzenlenme tarihleri, bulut senkronizasyon zamanı, geçmiş sürüm sayısı, dosya boyutu, kelime/karakter istatistikleri ve not etiketlerini şık bir popover içerisinde gösterir.
* **Formatlama Araç Çubuğu (Formatting Toolbar):**
  - Editör alanının üstüne şık, yarı şeffaf (frosted glass) ve fareyle üzerine gelindiğinde netleşen (fade-in) bir **Formatlama Araç Çubuğu** yerleştirildi. H1, H2, H3 başlıkları, Kalın, İtalik, Madde/Numaralı/Yapılacaklar listeleri, Alıntı, Satır içi kod/Kod bloğu, Link ve Eklenti ekleme butonları sunuldu.
* **Recent Attachments Dropdown & Sürükle-Bırak:**
  - Eklenti ekleme butonunun yanındaki küçük oka tıklandığında açılan popover'da son 20 eklenti görselleriyle (görsel olmayanlar ise doküman ikonuyla) listelenir. Kullanıcı listedeki bir eklentiye tıklayarak veya listeden doğrudan editörün içine sürükleyip bırakarak (drag and drop) eklentiyi anında kürsörün olduğu yere ekleyebilir.
  - Bilgisayardan editöre bir `.md` veya `.txt` dosyası bırakıldığında doğrudan yeni bir not olarak içeri aktarılır. Diğer tüm dosyalar (resim, pdf vb.) otomatik olarak eklenti (attachment) klasörüne kopyalanır ve nota linki basılır. Sürükleme esnasında ekranı kaplayan şık bir animasyonlu overlay ("Dosyaları Buraya Bırakın") tetiklenmektedir.

### Sonuç
Markdown yazım hızı gelişmiş formatlama araç çubuğuyla maksimuma çıkarılmış, son eklentilerin listelenmesi ve not içine sürüklenip bırakılmasıyla Obsidian kalitesinde bir dosya yönetim akışı kurulmuş ve tek tıkla ulaşılan detaylı not bilgisi popover'ı ile notun tüm kimlik kartı kullanıcıya sunulmuştur.

---

## 15. Gelişmiş Dinamik Eklenti Önizleme (Quick Look) Motoru, Sınırsız Eklenti Türleri ve Arayüz Temizliği

### Karşılaşılan Problemler
* **WebView Çökmesi ve Sayfa Hijacking Hatası:** Markdown önizleyicisinde `noda://attachments/...` linkine tıklandığında, Tauri WebView altyapısı bu mutlak URL'yi üst düzey sayfa geçişi olarak yorumluyor ve Svelte uygulamasının üzerine bu dosyayı açarak uygulamayı kilitliyordu. Geri dönmek mümkün olmuyor ve kullanıcının uygulamayı kapatıp açması gerekiyordu.
* **Önizleme Kısıtlamaları:** Eklenti önizleme aracı (Quick Look) sadece `.png`, `.jpg` gibi statik olarak tanımlanmış görselleri gösteriyor; örneğin `.pdf` gibi sık kullanılan belgeleri veya metin/kod dosyalarını önizleyemiyor, hepsine "Non-Previewable Attachment" uyarısı veriyordu.
* **Eklenti Yükleme Sınırları:** Eklenti ekleme (Add Attachment) pencereleri sadece belirli resim ve metin uzantılarını seçmeye izin veriyor, örneğin `.json` veya `.zip` gibi herhangi bir dosyayı eklenti olarak iliştirmeye izin vermiyordu.
* **Bileşen Çiftlenmesi (Duplicate Modal):** macOS Quick Look modal bileşeni hem ana sayfa düzeyinde hem de `NoteList.svelte` içerisinde ayrı ayrı tanımlanmıştı. Bu durum kod çiftlenmesine yol açıyor ve durum senkronizasyonunu zorlaştırıyordu.

### Çözüm Yöntemi ve Güncellemeler
* **Global Link Yakalayıcı ve WebView Koruma (Link Interceptor):**
  - `frontend/src/routes/+page.svelte` dosyasında, pencere düzeyinde capturing (yakalama) fazında (`true`) çalışan global bir `click` dinleyicisi kuruldu. `noda://attachments/...` link tıklamaları WebView'a ulaşmadan yakalandı, `e.preventDefault()` çağrılarak sayfa değişiminin önüne geçildi ve ilgili eklenti adı doğrudan global `triggerQuickLook(name)` işlevine yönlendirildi.
* **Çoklu Format Desteğiyle macOS Quick Look Modal Yükseltmesi:**
  - `QuickLookModal.svelte` içerisinde Svelte 5 reaktif `$derived` yapıları kullanılarak dosyanın `.pdf` mi yoksa metin/kod tabanlı (`.txt`, `.json`, `.md`, `.js`, `.ts` vb.) mı olduğu dinamik olarak analiz edildi.
  - PDF dosyaları için, `noda://` şemasını kullanan asenkron yükleyiciyi çalıştıran estetik bir `iframe` önizleyicisi geliştirildi.
  - Metin ve kod dosyaları için, Svelte 5 `$effect` bloğu ile eklenti içeriği `noda://attachments/...` üzerinden anlık olarak asenkron çekilerek premium, satır kaydırmalı (word-wrap) ve Menlo/Monaco fontlu şık bir `<pre><code>` alanı içerisinde gösterildi.
  - Binary/Bilinmeyen dosyalar için ise macOS tarzı şık bir fallback doküman ikonu ve meta veri kartı korundu.
* **Sınırsız Eklenti Yükleme Desteği:**
  - `Editor.svelte` eklenti ekleme penceresindeki (`handleAddAttachment`) katı uzantı filtreleri kaldırılarak `All Files (*.*)` ve `extensions: ['*']` ayarı yapıldı. Böylece sistem kararlılığını bozmadan her türden dosya eklenebilir hale geldi.
* **Kod Temizliği ve Tekil Modal Mimarisi:**
  - `NoteList.svelte` üzerindeki lokal Quick Look durumu, değişkenler ve mükerrer `<QuickLookModal>` bileşeni tamamen silindi. Eklenti kartına tıklama eylemi doğrudan global store'daki `triggerQuickLook` fonksiyonuna bağlanarak kod kalitesi artırıldı.

### Sonuç
Gelişmiş dinamik önizleme motoru sayesinde PDF'ler doğrudan arayüz içinde okunabilir, metin/kod eklentileri tek tıkla incelenebilir hale getirilmiştir. Global interceptor ile uygulamanın WebView kilitlenme hatası kökten çözülmüş, sınırsız dosya formatı yükleme esnekliği ve tekil modal yapısıyla mimari son derece kararlı hale getirilmiştir.

---

## 16. Sürüm Geçmişi Butonunun Taşınması ve Gelişmiş Rust-First Not Bilgi Paneli (Info Popover) Entegrasyonu

### Karşılaşılan Problemler
* **Buton Yerleşim Tutarsızlığı**: Notun versiyon geçmişini (Version History) gösteren buton toolbar üzerinde (`section-right` altında) bulunuyordu. Bu yerleşim, not düzenleme ve kaydetme eylemlerinin yapıldığı editör alanından uzaktı ve arayüz simetrisini bozuyordu.
* **Not Detaylarına ve Durumuna Ulaşma Zorluğu**: Kullanıcıların notun boyutu, fiziksel dosya adı, diskteki bağıl ve mutlak yolları, kaç adet geçmiş sürümünün saklandığı, en son ne zaman bulut senkronizasyonu (WebDAV) gerçekleştirildiği gibi hayati bilgilere editör ekranından doğrudan ve şık bir şekilde erişebileceği bir arayüz bulunmuyordu.

### Çözüm Yöntemi ve Güncellemeler
* **Buton Grubunun Yeniden Konumlandırılması**:
  - `Toolbar.svelte` içerisindeki sürüm geçmişi butonu, tüm bağımlılıkları ve metotlarıyla birlikte arındırılarak editörün sağ üst köşesindeki `Save` butonunun yanına (`Editor.svelte` note-actions alanına) taşındı.
* **Rust-First Mimariyle Not Bilgisi Çekilmesi (`get_note_metadata`)**:
  - Core mimariyi UI-agnostic tutma prensibine sadık kalınarak, ön yüzün düz bir monitör gibi davranmasını sağlayacak şekilde tüm istatistik ve metaveri hesaplamaları Rust backend katmanına kaydırıldı.
  - **`crates/shared/src/lib.rs`**: Ön yüz ve arka yüz sınırlarını güvenle geçen, not adı, dosya adı, disk yolları, zaman damgaları, etiketler, geçmiş versiyon sayısı, son sync/bulut yükleme zamanı, dosya boyutu ile kelime/karakter sayılarını barındıran `NoteMetadataDto` veri modeli tanımlandı.
  - **`crates/tauri-shell/src/commands/note_commands.rs`**: `get_note_metadata` Tauri IPC komutu yazıldı. Bu komut:
    1. İlgili note_id ile notun diskteki mutlak yolunu çözümler.
    2. `.noda/history/{id}/` klasöründeki geçmiş sürüm dosyalarını tarayarak gerçek snapshot sayısını hesaplar.
    3. `.noda/sync/remote_state.json` dosyasını okuyarak notun en son buluta yüklendiği asıl zamanı (`last_modified`) tespit eder.
    4. Dosya boyutunu (`std::fs::metadata`) ve metin gövdesindeki kelime/karakter istatistiklerini hesaplayarak DTO formatında fırlatır.
  - Komut `main.rs` altındaki Tauri invoke handler'ına kaydedildi ve frontend `ipc.ts` ile `types` modüllerine entegre edildi.
* **Dinamik ve Premium Info Popover Tasarımı**:
  - `Editor.svelte` note-actions alanına "Info" butonu eklendi. Buton; başlık, dosya adı, bağıl/mutlak disk yolları, oluşturulma/düzenlenme tarihleri, bulut senkronizasyon zamanı, geçmiş sürüm sayısı, dosya boyutu, kelime/karakter istatistikleri ve not etiketlerini şık bir popover içerisinde gösterir.
  - Popover açıkken not değiştirilirse veya not üzerinde bir güncelleme yapılırsa veriler arka planda Rust backend'inden otomatik ve asenkron olarak yeniden yüklenir (`loadNoteMetadata`).
  - Kullanıcı deneyimini mükemmelleştirmek amacıyla, popover dışına veya diğer arayüz elemanlarına tıklandığında bilgi balonunun otomatik olarak kapanmasını sağlayan window click dinleyicisi entegre edildi.

### Sonuç
Notun sürüm geçmişi butonu asıl ait olduğu editör alanına taşınarak arayüz bütünlüğü sağlanmıştır. Ayrıca projenin "flat monitor" vizyonuna tam uyumlu, tüm iş mantığı (dosya boyutu, geçmiş sayısı, senkronizasyon zamanı vb.) tamamen Rust katmanında çözülen ve kullanıcının tek tıkla notun tüm kimlik kartını pürüzsüzce inceleyebileceği premium bir bilgi popover paneli elde edilmiştir.

---

## 17. Kalıntı Dosya (Orphaned Remnants) ve Çakışmaların Gelişmiş Teşhisi, Bireysel Silme ve Canlı Önizleme Çekmecesi Entegrasyonu

### Karşılaşılan Problemler
* **Gizli Disk Doluluğu ve Kalıntılar:** Notlar kalıcı olarak silindiğinde bile o nota ait `.noda/history/{note_id}/` dizinindeki geçmiş snapshots dosyaları ve `.noda/conflicts/` altındaki `{note_id}_*.md` çakışma dosyaları diskte sahipsiz (orphaned) kalıyor ve zamanla depolama alanını dolduruyordu.
* **Kimliksiz Kalıntı Listesi:** Sahipsiz dosyalar ilk aşamada sadece ham dosya yolları veya kimliksiz UUID'ler olarak listelendiği için, kullanıcı hangi dosyanın hangi nota ait olduğunu anlayamıyordu.
* **Önizleme ve Bireysel Kontrol Eksikliği:** Kullanıcıların bu kalıntıların içinde ne olduğunu silmeden önce inceleyebileceği bir önizleme mekanizması ve tümünü toplu silmek yerine sadece belirli dosyaları seçip silebileceği bireysel yönetim imkanı yoktu.

### Çözüm Yöntemi ve Güncellemeler
* **Rust Core Katmanında Otomatik Temizlik (Aşama 1):**
  - `crates/core/src/trash/storage.rs` içerisindeki `permanent_delete` fonksiyonu güncellendi. Not çöpten kalıcı olarak silindiğinde, o nota ait `.noda/history/{note_id}/` geçmiş dizini (`tokio::fs::remove_dir_all`) ve `.noda/conflicts/` dizinindeki tüm çakışma dosyaları anında ve otomatik olarak temizlenecek şekilde kodlandı.
* **Akıllı Zengin Teşhis ve Başlık Çözümleme (Aşama 2):**
  - `crates/core/src/diagnostics/mod.rs` dosyasına `OrphanedFile` ve güncellenmiş `OrphanedRemnants` veri modelleri eklendi.
  - `get_orphaned_remnants` fonksiyonu, sahipsiz her geçmiş snapshot ve çakışma Markdown dosyasını asenkron olarak okuyup `gray-matter` ile frontmatter bloklarını parse edecek şekilde geliştirildi. Böylece dosyanın asıl not başlığı (title) dinamik olarak çekildi ve kullanıcıya `Seyahat Planım (Geçmiş: 2026-05-25 00:08:43)` veya `Proje Taslağı (Çakışma: 2026-05-25 00:08:43)` gibi son derece anlaşılır formatlarda sunulması sağlandı.
  - Tekil kalıntı dosyalarını diskten güvenle silen, path traversal güvenlik taraması yapan ve ebeveyn history klasörü boşaldığında o klasörü de otomatik temizleyen `delete_orphaned_file` Tauri IPC komutu kodlandı ve `main.rs` invoke listesine eklendi.
* **Premium Arayüz ve Önizleme Çekmecesi (Preview Drawer) Entegrasyonu:**
  - `SettingsModal.svelte` içerisindeki **Maintenance** sekmesindeki "Vault Diagnostics & Storage Cleanup" grubuna eklenen kalıntı temizleme kartı, mükerrer notların görsel tasarım diliyle (`duplicate-file-item` vb.) birebir eşitlendi.
  - Kalıntı dosyalar boyutları, yolları ve düzenlenme tarihleriyle estetik bir listeye dönüştürüldü.
  - Kullanıcı listedeki bir kalıntı dosyasına tıkladığında, global Quick Look mekanizması ve `get_conflict_note` FFI komutu tetiklenerek dosyanın ham Markdown içeriği anında **Önizleme Çekmecesinde** (Preview Drawer) asenkron olarak yüklenip gösterildi.
  - Her dosyanın yanına yerleştirilen bağımsız "Sil" butonu ile tekil silme ve en alttaki "Tüm Kalıntıları Temizle" butonu ile toplu temizleme imkanları sunuldu.
* **Birim Testleri ile Doğrulama:**
  - `crates/core` altında `test_orphaned_remnants_diagnostics_flow` ve `test_trash_flow` asenkron birim testleri yazılarak kalıntıların asıl başlıklarıyla doğru taranması, bireysel silinmesi, otomatik silme tetikleyicisi ve klasör temizlikleri %100 doğrulandı.

### Sonuç
NodaNotes'un "flat monitor" felsefesine tam uyumlu olarak tüm zengin dosya okuma, başlık çözümleme ve güvenli silme mantığı Rust core katmanında tamamlanmıştır. Kullanıcının sahipsiz kalmış geçmiş sürümleri ve çakışma dosyalarını tek tek önizleyip silebileceği veya tek tıkla tamamen temizleyebileceği, disk alanı yönetimini son derece şeffaf ve güvenli hale getiren premium bir self-healing aracı elde edilmiştir.

## 18. Maintenance & Self-Healing Arayüzünün Tasarım Sistemine Uyumlanması ve Premium Görsel Yenilemesi

### Karşılaşılan Problemler
* **Tasarım Bütünlüğü ve Flat Görünüm Eksikliği:** "Maintenance & Self-Healing" sayfası ilk sürümünde flat, düzensiz ve kaba bir buton yığınından oluşuyordu. Kartların arka planları ve sınır çizgileri, NodaNotes'un koyu renk şeması (`app.css`) ve Apple HIG (Human Interface Guidelines) standartlarındaki elevated (derinlikli) modül tasarımlarıyla uyumsuzdu.
* **Görsel İpucu ve İkon Eksikliği:** İşlemlerin yanında açıklayıcı görsel simgeler bulunmuyor, bu da kullanıcı deneyiminde sayfanın okunabilirliğini ve etkileşim kalitesini düşürüyordu.
* **Gelişigüzel Renk Tanımları:** Hata ve başarı kutuları, buton hover durumları ve paneller tasarım sisteminde önceden tanımlanmış global değişkenler yerine bağımsız (hardcoded) Tailwind benzeri RGB kodlarıyla biçimlendirilmişti. Bu durum genel renk tutarlılığını bozuyordu.

### Çözüm Yöntemi ve Güncellemeler
* **İşlevsel Gruplama ve Temiz Sayfa Düzeni:**
  - `SettingsModal.svelte` içerisindeki Maintenance sekmesinde yer alan tüm teşhis ve onarım işlevleri üç ana başlık altında toplandı:
    1. **Database Administration** (Veritabanı yapısı ve FTS5 indeks sağlığı)
    2. **Vault Diagnostics & Storage Cleanup** (Dosya, kopya ve kalıntı yönetimi)
    3. **Synchronization Self-Healing** (WebDAV senkronizasyon kuyruğu ve izleme önbelleği kurtarma)
* **Gelişmiş İllüstratif SVG İkonları Entegrasyonu:**
  - Her işlem kartının sol tarafına o işleme özel premium, ince çizgili inline SVG ikonları yerleştirildi (Veritabanı silindiri, optimize/süpürge göstergesi, ataş simgesi, mükerrer notlar için üst üste binen sayfalar, geçmiş/kalıntı saat simgesi, tehlikeli senkronizasyon araçları için kırmızı/turuncu uyarı ikonları).
  - Kartların üzerine gelindiğinde (`:hover`) ikonların otomatik olarak birincil tema rengine (`var(--accent)`) bürünmesi sağlandı.
* **Glassmorphic Teşhis Panelleri ve Durum Çizgileri (`.orphaned-box`):**
  - Dosya tarama ve teşhis sonuçlarının gösterildiği kutular yarı saydam cam efekti (`rgba(255, 255, 255, 0.015)`) ve yumuşatılmış köşeler ile premium bir tasarıma kavuşturuldu.
  - Sistemin sağlığını anında hissettirmek adına sol kenara dinamik durum çizgileri yerleştirildi:
    - Tarama temiz çıktığında (**0 dosya**) sol kenarda yeşil bir başarı çizgisi (`var(--color-green)`) belirir.
    - Temizlenecek ve silinecek dosya bulunduğunda sol kenarda mavi bir işlem çizgisi (`var(--accent)`) belirir.
* **Tasarım Sistemi Değişkenleri ve Mikro Animasyonlar:**
  - Tüm harici ve statik renk kodları temizlenerek projenin resmi CSS değişkenleriyle değiştirildi:
    - Kart Arka Planı: `var(--bg-elevated)` (`#2c2c2e`)
    - Hover Arka Planı: `var(--bg-elevated-2)` (`#3a3a3c`)
    - Sınır Çizgileri: `var(--border-normal)`, `var(--border-subtle)` ve aktif durumda `var(--border-strong)`
  - Kullanıcı kartın üzerine geldiğinde kartın yumuşak bir geçişle yukarı doğru hafifçe yükselmesi (`transform: translateY(-1px)`) ve şık bir gölge parlaması (`box-shadow: var(--shadow-sm)`) elde edildi.
  - Alert (hata/başarı) bildirim kutuları global CSS'teki standart `.alert`, `.alert-success` ve `.alert-error` sınıflarına geçirilerek tüm modal genelinde tasarım birliği korundu.
* **Svelte Derleme Uyumluluğu:**
  - Yenilenen Svelte arayüz yapısı `bun run check` komutuyla test edilerek TypeScript ve Svelte 5 kurallarına tam uyumluluğu, sıfır derleme hatasıyla tescillendi.

### Sonuç
Sayfanın tüm onarım, temizleme ve indeksleme fonksiyonları asıllarına sadık kalınarak korunmuş; bununla birlikte arayüz Apple'ın Sonoma tarzı derinlikli kart yapılarına, cam efektli teşhis kutularına ve premium inline SVG vektörlerine kavuşturulmuştur. Projenin modern ve tutarlı tasarım dili Maintenance sekmesinde de en üst seviyeye taşınmıştır.

---

## 19. Dosya Kimliği (ULID), Dosya Adı ve Ek (Attachment) Aramasının Entegre Edilmesi, Unicode Uyumlu ve Güvenli Vurgulama Geliştirmesi

### Karşılaşılan Problemler
* **ID ve Dosya Adı Araması Eksikliği:** Arama modülü sadece not başlığı, gövdesi ve etiketleriyle sınırlıydı; dosya kimlikleri (ULID) ve özel dosya adları ile arama yapılamıyordu.
* **Sınır Dışı Dilimleme Hatası ve Çökmeler (Out of Bounds Panic):** Arama çubuğuna yazılan sorgu aranan hedeften (örneğin 26 karakterlik ULID) daha uzun olduğunda (örneğin 29 karakterlik girdi yapıldığında), Rust tarafındaki karakter dilimleme mantığı sınır dışına çıkıyor ve worker thread panikleyerek uygulamanın o anki iş parçacığını çökertebiliyordu.
* **Alfa-Nümerik Olmayan Karakterler ve Attachment Arama Engeli:** Notların içindeki attachment dosyalarının isimleri (örneğin `xxh3_265b76ac10173dcc.jpg`) aranmak istendiğinde, Rust arama motoru alfa-nümerik olmayan karakterleri (`_`, `.`, `-` vb.) tamamen ayıklayıp kelimeyi birleştiriyordu. SQLite FTS5 veritabanı ise bu karakterleri ayrıcı kabul edip kelimeyi parçalı indekslediği için eşleşme yakalanamıyor ve arama sonuçları boş dönüyordu.

### Çözüm Yöntemi ve Güncellemeler
* **Rust Core Katmanında Doğrudan ID/Path Araması (LIKE):**
  - `crates/core/src/database/search.rs` içindeki `search_notes` fonksiyonu güncellendi. SQLite `LIKE` sorgusu eklenerek, `id` (ULID) ve `file_path` (dosya adı/yolu) sütunlarında doğrudan arama yapılması sağlandı.
  - Bu tür doğrudan eşleşmelere `-1000.0` gibi en öncelikli skor atanarak sonuçlerin her zaman FTS5 sonuçlarının en üstünde çıkması garantilendi.
* **Sıfır-Panik & Unicode-Uyumlu Vurgulama (`highlight_match`):**
  - Arama teriminin karakter uzunluğunun aranan hedeften büyük olması durumunda panikleri önlemek amacıyla anında `None` dönen ilk kontrol eklendi.
  - Orijinal karakter dizisi üzerinde, büyük/küçük harf duyarsızlığını sıfır bellek ayırma yapan `.to_lowercase().eq()` karakter döngüsüyle yöneten ve karakter sınırları içinde güvenli dilimleme yapan `highlight_match` fonksiyonu sıfırdan yazılarak panik riski tamamen ortadan kaldırıldı.
  - Arama modülünde eşleşen dosya adına göre `"File: <b>pasta</b>_recipe.md"` veya dosya ID'sine göre `"ID: <b>01JBY...</b>"` şeklinde şık vurgulamalı önizlemeler üretilmesi sağlandı.
* **Alfa-Nümerik Karakter Parçalama ile Kusursuz Attachment Araması:**
  - FTS5 arama terimi temizleme mantığı, özel karakterleri tamamen silmek yerine arama sorgusunu alfa-nümerik olmayan tüm karakterlerden split (ayıracak) şekilde optimize edildi.
  - `xxh3_265b76ac10173dcc.jpg` araması, SQLite FTS5'in kelime parçalama mantığına tam uyumlu `xxh3* AND 265b76ac10173dcc* AND jpg*` yapısına dönüştürülerek attachment aramaları anında sonuç verecek şekilde iyileştirildi.
* **Birim Testleri ile Kapsamlı Doğrulama:**
  - `test_search_by_id_and_filename` birim testi güncellenerek:
    1. Tam ULID eşleşmesi,
    2. 15 karakterlik benzersiz ULID ön eki araması,
    3. Özel dosya adı eşleşmesi,
    4. 30 karakterlik aşırı uzun sorgu panik testi (gerileme/regression koruması),
    5. Attachment dosya adı arama testi
    başarıyla doğrulandı. Yapılan `cargo test` doğrulamasıyla tüm testlerin hatasız geçtiği tescillendi.

### Sonuç
Noda'nın "dumb monitor" ön yüz vizyonuna sadık kalınarak tüm bu gelişmiş arama ve kelime parçalama mantığı tamamen Rust katmanında çözülmüştür. Sıfır çökme garantili, attachment isimlerini, dosya uzantılarını ve dosya kimliklerini anında en üst sırada görsel olarak vurgulayarak getiren premium bir arama motoru elde edilmiştir.
