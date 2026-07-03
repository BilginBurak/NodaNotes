# NodaNotes İşletim ve Yaşam Döngüsü Kılavuzu (Operations Manual)

Bu kılavuz, NodaNotes projesinin derlenmesi, çalıştırılması ve depolama alanının güvenli bir şekilde temizlenmesi için doğrulanmış komut gruplarını içerir.

---

## Part 1: Cold Boot Recovery Suite (Kurtarma ve Ayaklandırma Komutları)

Yeni bir klonlamadan veya sıfırlamadan sonra tüm sistem bileşenlerini ayağa kaldırmak için aşağıdaki adımları sırasıyla uygulayın.

### 1. Frontend Bağımlılıklarının Yüklenmesi
Web arayüzü katmanı için gerekli paketlerin Bun paket yöneticisi kullanılarak yüklenmesini sağlar.
- **Dizin:** `frontend/`
- **Komut:**
  ```bash
  bun install
  ```

### 2. Masaüstü Uygulaması Derleme Kanalları (Tauri)
Masaüstü istemcisini yerel geliştirme modunda çalıştırmak veya optimize edilmiş üretim sürümlerini derlemek için kullanılır.
- **Dizin:** `crates/tauri-shell/`
- **Geliştirme Modu (Dev):**
  ```bash
  cargo tauri dev
  ```
- **Üretim Sürümü Derleme (Production Build):**
  ```bash
  cargo tauri build
  ```

### 3. Mobil Uygulama Derleme Kanalları (Android)
Android uygulamasını derlemek ve cihazlara yüklemek için kullanılan Gradle komutlarıdır. Rust köprüsü (`android-bridge`) otomatik olarak derleme zincirine dahil edilir.
- **Dizin:** `android/`
- **Hata Ayıklama Paketi Yükleme (Debug Build):**
  ```bash
  JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home" ./gradlew installDebug
  ```
- **Hata Ayıklama Paketi Derleme (Alternative Debug APK Output):**
  ```bash
  JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home" ./gradlew assembleDebug
  ```
- **Optimize Edilmiş Nihai Mobil Paket Derleme (Release APK):**
  ```bash
  JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home" ./gradlew assembleRelease
  ```

### 4. Eklenti Derleme Kanalları (Clipper Suite)
Web Clipper eklenti paketini derlemek ve hazır hale getirmek için kullanılır.
- **Dizin:** Proje Kök Dizini (`/`)
- **Komut:**
  ```bash
  bun .clipper/package.js
  ```

### 5. Landing Sayfası Derleme Ayarları (Netlify Context)
Landing sayfasının Netlify üzerindeki dağıtımı (deployment) doğrudan `landing/netlify.toml` yapılandırma dosyası üzerinden yönetilir. Referans olması açısından geçerli parametreler şunlardır:
```toml
[build]
  command = "bun run build"
  publish = "build"
```

### 6. Otomatik Yayınlama ve Dağıtım Kanalları (Automated Publishing & Deployment Tracks)
Uygulama ve tarayıcı eklentisi sürümlerini otomatik olarak derleyip yayınlamak için kullanılır (Bu betikler özel alt modülümüz içerisinde korunur).
- **Dizin:** Proje Kök Dizini (`/`)
- **Clipper Eklentisi WebDAV Yayınlama (Clipper Publisher):**
  ```bash
  bun .scripts/publish_clipper.js
  ```
- **Android Sürüm Güncellemesi Yayınlama (Android Publisher):**
  ```bash
  bun .scripts/publish_android.js
  ```

---

## Part 2: Storage Clean & Pruning Suite (Temizleme ve Disk Boşaltma Komutları)

Sistemde biriken geçici dosyaları ve derleme önbelleklerini verileri bozmadan güvenle temizlemek için aşağıdaki komutları kullanın.

### 1. Rust Derleme Önbelleği Temizliği (Maksimum Disk Kazancı)
Rust derleme adımlarında oluşturulan ve diskte büyük yer kaplayan `target/` önbellek dizinini güvenli bir şekilde siler.
- **Dizin:** Proje Kök Dizini (`/`)
- **Komut:**
  ```bash
  cargo clean
  ```

### 2. Frontend Önbellek ve Modül Temizliği
SvelteKit önbellek dizinlerini, `node_modules` bağımlılık paketlerini ve derlenmiş web arayüzü çıktılarını tamamen temizler.
- **Dizin:** `frontend/`
- **Komut:**
  ```bash
  rm -rf node_modules .svelte-kit build
  ```

### 3. Android Derleme Önbelleği Temizliği
Android tarafındaki Gradle ara nesne derlemelerini ve önbellek çıktılarını güvenle sıfırlar.
- **Dizin:** `android/`
- **Komut:**
  ```bash
  ./gradlew clean
  ```

> [!WARNING]
> Yerel bağımlılıkların (hardlinked local dependencies) bozulmasını önlemek için, `android/` dizini altında bulunan `app/src/main/jniLibs` klasörünü kesinlikle manuel olarak `rm -rf` ile **SİLMEYİNİZ**.
