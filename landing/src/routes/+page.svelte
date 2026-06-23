<script lang="ts">
	import { onMount } from 'svelte';

	// Animation states
	let activeTab = $state('sync');
	let scrolled = $state(false);

	// Carousel screenshots
	const features = [
		{
			id: 'sync',
			title: '0.1s Fast-Check Sync',
			description: 'WebDAV Empty Marker Protocol (Zero-Byte marker) prevents duplicate syncs and reduces overhead from 20s to 0.1s.',
			icon: '⚡'
		},
		{
			id: 'filesystem',
			title: 'Filesystem-First Structure',
			description: 'Your folders are the database. Notes are physical standard .md files with YAML frontmatter. If SQLite fails, Noda reconstructs everything.',
			icon: '📂'
		},
		{
			id: 'encryption',
			title: 'Secure Local Storage',
			description: 'Keytar / Android EncryptedSharedPreferences security wrappers ensure your credentials and WebDAV tokens are securely stored.',
			icon: '🔒'
		},
		{
			id: 'history',
			title: 'Flat File-Based Snapshots',
			description: 'Full version history stored directly as flat files. View diff comparisons easily with instant rollback.',
			icon: '⏳'
		},
		{
			id: 'calendar',
			title: 'Daily Notes & Calendar Dialog',
			description: 'Create and navigate daily notes using a custom interactive calendar UI. Stay organized day by day.',
			icon: '📅'
		}
	];

	// Handle navbar transparent to solid transition on scroll
	onMount(() => {
		const handleScroll = () => {
			scrolled = window.scrollY > 50;
		};
		window.addEventListener('scroll', handleScroll);
		return () => window.removeEventListener('scroll', handleScroll);
	});
</script>

<svelte:head>
	<title>NodaNotes — Pure Rust Markdown Notes Application</title>
	<meta name="description" content="Cross-platform markdown note-taking app with high performance WebDAV synchronization, flat file history, and local-first encryption." />
</svelte:head>

<!-- Header / Navigation -->
<header class="navbar" class:scrolled>
	<div class="nav-container">
		<a href="#" class="logo-area">
			<img src="/logo.png" alt="NodaNotes Logo" class="nav-logo" />
			<span class="logo-text">Noda<span>Notes</span></span>
		</a>
		<nav class="nav-links">
			<a href="#features">Özellikler</a>
			<a href="#architecture">Mimari</a>
			<a href="#screenshots">Ekran Görüntüleri</a>
			<a href="#download" class="btn-primary">İndir</a>
		</nav>
	</div>
</header>

<!-- Hero Section -->
<section class="hero">
	<div class="hero-bg-glow"></div>
	<div class="hero-container">
		<div class="hero-content">
			<div class="badge">🚀 YENİ NESİL NOT ALMA DENEYİMİ</div>
			<h1>Göz Alıcı Tasarım,<br><span class="gradient-text">Saf Rust Performansı</span></h1>
			<p class="hero-subtitle">
				Tauri & Kotlin ile güçlendirilmiş; WebDAV destekli, yerel-öncelikli (local-first) şifreleme sunan, tamamen standart Markdown formatında çalışan, eşsiz bir not defteri.
			</p>
			
			<div class="hero-actions">
				<a href="#download" class="btn-hero-primary">Hemen İndir</a>
				<a href="#features" class="btn-hero-secondary">Keşfet</a>
			</div>

			<div class="platforms-supported">
				<span>Desteklenen Platformlar:</span>
				<div class="platform-icons">
					<span class="p-icon" title="macOS">🍎 macOS</span>
					<span class="p-icon" title="Android">🤖 Android</span>
					<span class="p-icon" title="Windows/Linux">💻 Windows & Linux</span>
				</div>
			</div>
		</div>

		<div class="hero-mockup-wrapper">
			<div class="hero-mockup-glow"></div>
			<div class="mockup-frame">
				<div class="mockup-header">
					<div class="dots"><span class="dot-r"></span><span class="dot-y"></span><span class="dot-g"></span></div>
					<div class="address-bar">NodaNotes — Developer Vault</div>
				</div>
				<div class="mockup-content">
					<!-- Fake Editor UI -->
					<div class="fake-app">
						<aside class="fake-sidebar">
							<div class="app-logo-small"><img src="/logo.png" alt="" /> Noda</div>
							<div class="folder-section">
								<div class="sec-title">WORKSPACE</div>
								<div class="f-item active">📝 Notes</div>
								<div class="f-item">📅 Daily Notes</div>
								<div class="f-item">📁 Archives</div>
								<div class="f-item">🗑️ Trash</div>
							</div>
							<div class="tag-section">
								<div class="sec-title">TAGS</div>
								<div class="t-badge">#rust</div>
								<div class="t-badge">#tauri</div>
								<div class="t-badge">#kotlin</div>
							</div>
						</aside>
						<section class="fake-note-list">
							<div class="search-bar">🔍 Ara...</div>
							<div class="n-card active">
								<div class="n-title">NodaSync Engine Spec</div>
								<div class="n-desc">Zero-byte signature and fast-check algorithm overview...</div>
								<div class="n-tags"><span>#rust</span><span>#sync</span></div>
							</div>
							<div class="n-card">
								<div class="n-title">Daily Note: 2026-06-23</div>
								<div class="n-desc">Today I completed the new high-performance landing page...</div>
								<div class="n-tags"><span>#daily</span></div>
							</div>
						</section>
						<main class="fake-editor">
							<div class="editor-header">
								<div class="note-info">
									<span class="note-icon">📝</span>
									<h3>NodaSync Engine Spec</h3>
								</div>
								<div class="sync-badge">✓ Synced (0.1s)</div>
							</div>
							<div class="editor-body">
								<pre><code><span class="cm-yaml">---
id: "01JXYZ..."
title: "NodaSync Engine Spec"
tags: ["rust", "sync"]
pinned: true
---</span>

<span class="cm-h1"># Noda Sync Architecture</span>

The synchronizer uses a <span class="cm-bold">Zero-Byte Marker Protocol</span> 
for ultra-fast remote state evaluations.

<span class="cm-h2">## Core Logic</span>
1. Local changes evaluated first
2. Remote ETags queried via Depth: 1
3. 0.1s Fast-Check completes if hashes match</code></pre>
							</div>
						</main>
					</div>
				</div>
			</div>
		</div>
	</div>
</section>

<!-- Features Grid Section -->
<section id="features" class="features-section">
	<div class="section-header">
		<h2>Neden NodaNotes?</h2>
		<p>Güçlü bir altyapı ve kusursuz tasarım prensipleri üzerine inşa edildi.</p>
	</div>
	<div class="features-grid">
		<div class="feature-card">
			<div class="feat-icon">⚡</div>
			<h3>0.1s Hızında WebDAV Sync</h3>
			<p>Gelişmiş "Fast-Check" mimarisi sayesinde hiçbir değişiklik yoksa 0.1 saniyede senkronizasyonu tamamlar, şebeke ve pil tüketmez.</p>
		</div>
		<div class="feature-card">
			<div class="feat-icon">🔒</div>
			<h3>Yerel-Öncelikli Şifreleme</h3>
			<p>Hassas verileriniz ve WebDAV parolalarınız macOS Keychain ve Android EncryptedSharedPreferences ile cihazınızda şifreli tutulur.</p>
		</div>
		<div class="feature-card">
			<div class="feat-icon">📂</div>
			<h3>Dosya Sistemi Bağımsızlığı</h3>
			<p>Notlarınız sunucularda değil, tamamen standart .md formatında kendi disklerinizdedir. Uygulama veritabanı silinse dahi kendini diskten otomatik yeniden inşa eder.</p>
		</div>
		<div class="feature-card">
			<div class="feat-icon">⏳</div>
			<h3>Flat File Snapshot Geçmişi</h3>
			<p>Her not kaydetme veya çıkış esnasında otomatik olarak hafif sürüm kopyaları alınır. Geriye dönük değişimleri visual diff (satır farkı) ile izleyin.</p>
		</div>
		<div class="feature-card">
			<div class="feat-icon">📅</div>
			<h3>Gelişmiş Günlük Notlar</h3>
			<p>TopAppBar'daki entegre takvim düğmesi veya kısayollar sayesinde gün bazlı notlarınızı bir dokunuşla oluşturun, takvim üzerinde görün.</p>
		</div>
		<div class="feature-card">
			<div class="feat-icon">🚀</div>
			<h3>Ultra Hafif ve Hızlı</h3>
			<p>Tüm ağır iş mantığı (arama, şifreleme, disk işlemleri) saf Rust (RustCore) ile yürütülür. Arayüz sadece "Dumb Monitor" olarak çalışır.</p>
		</div>
	</div>
</section>

<!-- Interactive Architecture Section -->
<section id="architecture" class="architecture-section">
	<div class="arch-container">
		<div class="arch-text">
			<h2>Benzersiz Çift Katmanlı Mimari</h2>
			<p>NodaNotes, kullanıcı deneyimini maksimuma çıkarmak için Tauri ve Kotlin platformlarını tek bir saf Rust çekirdeği ile besler.</p>
			
			<div class="arch-selector">
				{#each features as feat}
					<button 
						class="arch-btn" 
						class:active={activeTab === feat.id}
						onclick={() => activeTab = feat.id}
					>
						<span class="btn-icon">{feat.icon}</span>
						<div class="btn-text">
							<h4>{feat.title}</h4>
						</div>
					</button>
				{/each}
			</div>
		</div>

		<div class="arch-visual">
			<div class="visual-card">
				<div class="visual-header">
					<span class="tag">RUST CORE SPECIFICATION</span>
				</div>
				<div class="visual-body">
					{#if activeTab === 'sync'}
						<h3>WebDAV Fast-Check Protocol</h3>
						<p>Yerel veritabanı temiz durumdaysa, uzaktaki .sync imzaları sorgulanır. Herhangi bir değişiklik yoksa eşitleme döngüsü 0.1 saniye içerisinde sonlandırılır.</p>
						<div class="code-box">
							<span class="code-comment">// zero-byte signature calculation</span>
							<span class="code-keyword">let</span> remote_sync_etag = client.propfind_depth_0(url).await?;
							<span class="code-keyword">if</span> local_etag == remote_sync_etag {'{'}
							    log::info!(<span class="code-str">"Zero modifications. Fast exit."</span>);
							    <span class="code-keyword">return</span> <span class="code-val">Ok</span>(SyncResult::Clean);
							{'}'}
						</div>
					{:else if activeTab === 'filesystem'}
						<h3>Filesystem-First Law</h3>
						<p>Kullanıcının klasörü tek gerçek kaynaktır. SQLite sadece hızlı FTS5 araması ve önbellek için kullanılır. Tüm notlar YAML frontmatter ile standart diskte yer alır.</p>
						<div class="code-box">
							<span class="code-comment">// YAML frontmatter block example</span>
							---
							id: <span class="code-str">"01JXYZ..."</span>
							title: <span class="code-str">"NodaNotes Architecture"</span>
							updated_at: <span class="code-str">"2026-06-23T03:30:00Z"</span>
							tags: [<span class="code-str">"rust"</span>, <span class="code-str">"architecture"</span>]
							---
						</div>
					{:else if activeTab === 'encryption'}
						<h3>Keychain & Cryptography Wrappers</h3>
						<p>WebDAV parolanız, token'lar ve hassas veriler yerel diskte asla düz metin olarak saklanmaz. Sistem OS-level Keychain API'leri ile sıkıca sarmalanmıştır.</p>
						<div class="code-box">
							<span class="code-keyword">use</span> security_framework::keychain::SecKeychain;
							<span class="code-comment">// Safely resolve keychain access</span>
							<span class="code-keyword">let</span> password = SecKeychain::default()?
							    .find_generic_password(<span class="code-str">"NodaNotes"</span>, username)?;
						</div>
					{:else if activeTab === 'history'}
						<h3>Flat File History</h3>
						<p>Versiyon geçmişleri karmaşık veritabanı tablolarında değil, doğrudan `.noda/history/` altında flat dosyalar halinde kaydedilir. Disk I/O ve sync yükü minimumdur.</p>
						<div class="code-box">
							<span class="code-comment">// flat history path naming pattern</span>
							.noda/history/[NoteID]_[YYYYMMDD-HHMMSS]_[reason].md
						</div>
					{:else if activeTab === 'calendar'}
						<h3>Dynamic Calendar & Daily Notes</h3>
						<p>JNI köprüsü ve RustCore üzerinden dilediğiniz güne ait günlük notu (Daily Note) tek tuşla tetikleyin, diskte organize edin.</p>
						<div class="code-box">
							<span class="code-comment">// Kotlin JNI call representation</span>
							val params = Json.encodeToString(TriggerDailyParams(date = <span class="code-str">"2026-06-23"</span>))
							RustCore.triggerDailyNote(params)
						</div>
					{/if}
				</div>
			</div>
		</div>
	</div>
</section>

<!-- Screenhots Showcase / Required Screenshots list -->
<section id="screenshots" class="screenshots-section">
	<div class="section-header">
		<h2>Ekran Görüntüleri Mockup Listesi</h2>
		<p>Uygulamanın gücünü görmek için yerleştirmen gereken ekran görüntüleri rehberi.</p>
	</div>
	<div class="screen-grid">
		<div class="screen-card">
			<div class="screen-placeholder">
				<span class="icon">💻</span>
				<h4>1. macOS Editör & Markdown Görünümü</h4>
				<p>CodeMirror 6 ile güçlendirilmiş, syntax vurgulamalı ve temiz üç panelli arayüzün ekran görüntüsü.</p>
			</div>
		</div>
		<div class="screen-card">
			<div class="screen-placeholder">
				<span class="icon">🤖</span>
				<h4>2. Android Jetpack Compose Arayüzü</h4>
				<p>Material 3 Dynamic Color (Monet) destekli, akıcı geçişlere sahip mobil not listesi ve editör ekran görüntüsü.</p>
			</div>
		</div>
		<div class="screen-card">
			<div class="screen-placeholder">
				<span class="icon">📅</span>
				<h4>3. Dinamik Takvim & Daily Notes</h4>
				<p>Kullanıcının geçmiş günlük notlarını kolayca seçebildiği şık takvim diyaloğu görünümü.</p>
			</div>
		</div>
		<div class="screen-card">
			<div class="screen-placeholder">
				<span class="icon">⏳</span>
				<h4>4. Visual Diff Sürüm Karşılaştırması</h4>
				<p>Bir notun geçmiş sürümleri arasındaki satır farklarını gösteren visual diff panel ekran görüntüsü.</p>
			</div>
		</div>
	</div>
</section>

<!-- Call to action / Download Section -->
<section id="download" class="download-section">
	<div class="download-container">
		<h2>Verilerinizin Kontrolünü Eline Alın</h2>
		<p>Hemen bugün NodaNotes'u indirin ve yüksek performanslı, şifreli ve yerel öncelikli not almanın keyfini çıkarın.</p>
		
		<div class="download-buttons">
			<a href="#" class="btn-download">
				<span class="d-icon">🍏</span>
				<div class="d-text">
					<span class="d-sub">Download for</span>
					<span class="d-main">macOS (Tauri)</span>
				</div>
			</a>
			<a href="#" class="btn-download">
				<span class="d-icon">🤖</span>
				<div class="d-text">
					<span class="d-sub">Download for</span>
					<span class="d-main">Android (Kotlin)</span>
				</div>
			</a>
		</div>
	</div>
</section>

<!-- Footer -->
<footer>
	<div class="footer-container">
		<div class="footer-brand">
			<img src="/logo.png" alt="" class="footer-logo" />
			<span>NodaNotes</span>
		</div>
		<p>© 2026 NodaNotes. All rights reserved. Built with Rust, Tauri, Svelte & Kotlin.</p>
	</div>
</footer>

<style>
	/* Header Navigation */
	.navbar {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 80px;
		display: flex;
		align-items: center;
		z-index: 100;
		border-bottom: 1px solid transparent;
		transition: var(--transition-smooth);
	}
	.navbar.scrolled {
		background: rgba(9, 9, 11, 0.85);
		backdrop-filter: blur(12px);
		border-bottom: 1px solid var(--border-color);
		height: 70px;
	}
	.nav-container {
		width: 90%;
		max-width: 1200px;
		margin: 0 auto;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.logo-area {
		display: flex;
		align-items: center;
		gap: 10px;
		text-decoration: none;
		color: var(--text-primary);
	}
	.nav-logo {
		height: 38px;
		width: auto;
	}
	.logo-text {
		font-size: 1.4rem;
		font-weight: 700;
		letter-spacing: -0.5px;
	}
	.logo-text span {
		color: var(--color-primary);
	}
	.nav-links {
		display: flex;
		align-items: center;
		gap: 30px;
	}
	.nav-links a {
		text-decoration: none;
		color: var(--text-secondary);
		font-weight: 500;
		font-size: 0.95rem;
		transition: var(--transition-smooth);
	}
	.nav-links a:hover {
		color: var(--text-primary);
	}
	.nav-links .btn-primary {
		padding: 8px 18px;
		background: var(--color-primary);
		color: #fff;
		border-radius: 8px;
		font-weight: 600;
	}
	.nav-links .btn-primary:hover {
		background: var(--color-primary-hover);
		transform: translateY(-2px);
	}

	/* Hero Section */
	.hero {
		position: relative;
		min-height: 100vh;
		display: flex;
		align-items: center;
		padding-top: 120px;
		padding-bottom: 80px;
	}
	.hero-bg-glow {
		position: absolute;
		top: -10%;
		left: 50%;
		transform: translateX(-50%);
		width: 80vw;
		height: 80vh;
		background: radial-gradient(circle, rgba(139, 92, 246, 0.15) 0%, rgba(6, 182, 212, 0.05) 50%, transparent 100%);
		pointer-events: none;
		z-index: 1;
	}
	.hero-container {
		width: 90%;
		max-width: 1200px;
		margin: 0 auto;
		display: grid;
		grid-template-columns: 1.1fr 0.9fr;
		gap: 60px;
		align-items: center;
		z-index: 2;
	}
	.hero-content {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
	}
	.badge {
		padding: 6px 14px;
		background: var(--color-primary-light);
		color: var(--color-primary-hover);
		border-radius: 99px;
		font-size: 0.8rem;
		font-weight: 700;
		letter-spacing: 0.5px;
		margin-bottom: 25px;
		border: 1px solid rgba(139, 92, 246, 0.3);
	}
	.hero h1 {
		font-size: 3.8rem;
		font-weight: 800;
		line-height: 1.1;
		letter-spacing: -2px;
		margin-bottom: 25px;
	}
	.gradient-text {
		background: var(--gradient-hero);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
	}
	.hero-subtitle {
		font-size: 1.15rem;
		line-height: 1.6;
		color: var(--text-secondary);
		margin-bottom: 35px;
	}
	.hero-actions {
		display: flex;
		gap: 20px;
		margin-bottom: 40px;
	}
	.btn-hero-primary {
		padding: 14px 28px;
		background: var(--gradient-hero);
		color: white;
		border-radius: 10px;
		font-weight: 600;
		text-decoration: none;
		transition: var(--transition-bounce);
		box-shadow: var(--glow-primary);
	}
	.btn-hero-primary:hover {
		transform: scale(1.05);
	}
	.btn-hero-secondary {
		padding: 14px 28px;
		background: var(--bg-surface);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		border-radius: 10px;
		font-weight: 600;
		text-decoration: none;
		transition: var(--transition-smooth);
	}
	.btn-hero-secondary:hover {
		background: var(--bg-surface-hover);
		border-color: var(--border-focus);
	}
	.platforms-supported {
		font-size: 0.85rem;
		color: var(--text-muted);
	}
	.platform-icons {
		display: flex;
		gap: 15px;
		margin-top: 10px;
	}
	.p-icon {
		background: var(--bg-surface);
		padding: 5px 12px;
		border-radius: 6px;
		border: 1px solid var(--border-color);
		color: var(--text-secondary);
	}

	/* Mockup Styling */
	.hero-mockup-wrapper {
		position: relative;
	}
	.hero-mockup-glow {
		position: absolute;
		width: 100%;
		height: 100%;
		background: radial-gradient(circle, rgba(6, 182, 212, 0.2) 0%, transparent 70%);
		z-index: 1;
		filter: blur(40px);
	}
	.mockup-frame {
		position: relative;
		z-index: 2;
		background: var(--bg-surface);
		border: 1px solid var(--border-color);
		border-radius: 12px;
		overflow: hidden;
		box-shadow: var(--shadow-card);
		aspect-ratio: 16/10;
		display: flex;
		flex-direction: column;
	}
	.mockup-header {
		height: 36px;
		background: #18181b;
		border-bottom: 1px solid var(--border-color);
		display: flex;
		align-items: center;
		padding: 0 15px;
		position: relative;
	}
	.dots {
		display: flex;
		gap: 6px;
	}
	.dots span {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		display: inline-block;
	}
	.dot-r { background: #ef4444; }
	.dot-y { background: #f59e0b; }
	.dot-g { background: #10b981; }
	.address-bar {
		position: absolute;
		left: 50%;
		transform: translateX(-50%);
		font-size: 0.75rem;
		color: var(--text-muted);
		background: var(--bg-base);
		padding: 3px 20px;
		border-radius: 4px;
		border: 1px solid var(--border-color);
	}

	/* Fake App inside Hero Mockup */
	.fake-app {
		display: flex;
		flex: 1;
		font-size: 0.75rem;
	}
	.fake-sidebar {
		width: 130px;
		background: #0e0e11;
		border-right: 1px solid var(--border-color);
		padding: 15px 10px;
	}
	.app-logo-small {
		display: flex;
		align-items: center;
		gap: 5px;
		font-weight: 700;
		margin-bottom: 20px;
		font-size: 0.85rem;
	}
	.app-logo-small img {
		height: 16px;
	}
	.sec-title {
		font-weight: 600;
		color: var(--text-muted);
		margin-bottom: 8px;
		letter-spacing: 0.5px;
	}
	.folder-section, .tag-section {
		margin-bottom: 20px;
	}
	.f-item {
		padding: 6px 8px;
		border-radius: 4px;
		color: var(--text-secondary);
		cursor: pointer;
		margin-bottom: 3px;
	}
	.f-item.active {
		background: var(--color-primary-light);
		color: var(--color-primary-hover);
		font-weight: 600;
	}
	.t-badge {
		display: inline-block;
		background: var(--bg-surface-hover);
		padding: 3px 6px;
		border-radius: 4px;
		margin-right: 4px;
		margin-bottom: 4px;
		color: var(--text-secondary);
	}
	
	.fake-note-list {
		width: 150px;
		background: #111115;
		border-right: 1px solid var(--border-color);
		padding: 10px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.search-bar {
		background: var(--bg-base);
		border: 1px solid var(--border-color);
		padding: 5px;
		border-radius: 4px;
		color: var(--text-muted);
	}
	.n-card {
		padding: 8px;
		border-radius: 6px;
		background: var(--bg-surface);
		border: 1px solid var(--border-color);
	}
	.n-card.active {
		border-color: var(--color-primary);
		background: var(--bg-surface-hover);
	}
	.n-title {
		font-weight: 600;
		margin-bottom: 3px;
	}
	.n-desc {
		color: var(--text-muted);
		font-size: 0.65rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin-bottom: 5px;
	}
	.n-tags span {
		background: var(--bg-base);
		padding: 2px 4px;
		border-radius: 3px;
		margin-right: 3px;
		color: var(--color-primary-hover);
	}

	.fake-editor {
		flex: 1;
		background: var(--bg-surface);
		display: flex;
		flex-direction: column;
	}
	.editor-header {
		padding: 10px 15px;
		border-bottom: 1px solid var(--border-color);
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.note-info {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.sync-badge {
		color: var(--color-success);
		background: rgba(16, 185, 129, 0.1);
		padding: 2px 8px;
		border-radius: 99px;
		font-size: 0.65rem;
	}
	.editor-body {
		flex: 1;
		padding: 15px;
		font-family: var(--font-mono);
		overflow: hidden;
	}
	.editor-body pre {
		white-space: pre-wrap;
	}
	.cm-yaml { color: var(--text-muted); }
	.cm-h1 { color: var(--color-primary-hover); font-weight: bold; }
	.cm-h2 { color: var(--color-secondary-hover); font-weight: bold; }
	.cm-bold { color: #fff; font-weight: bold; }

	/* Features Section */
	.features-section {
		padding: 100px 0;
		width: 90%;
		max-width: 1200px;
		margin: 0 auto;
	}
	.section-header {
		text-align: center;
		margin-bottom: 60px;
	}
	.section-header h2 {
		font-size: 2.5rem;
		font-weight: 800;
		letter-spacing: -1px;
		margin-bottom: 15px;
	}
	.section-header p {
		color: var(--text-secondary);
		font-size: 1.1rem;
	}
	.features-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
		gap: 30px;
	}
	.feature-card {
		background: var(--bg-surface);
		border: 1px solid var(--border-color);
		border-radius: 12px;
		padding: 30px;
		transition: var(--transition-smooth);
	}
	.feature-card:hover {
		transform: translateY(-5px);
		border-color: var(--color-primary);
		box-shadow: var(--shadow-card);
	}
	.feat-icon {
		font-size: 2rem;
		margin-bottom: 20px;
	}
	.feature-card h3 {
		font-size: 1.25rem;
		font-weight: 700;
		margin-bottom: 10px;
	}
	.feature-card p {
		color: var(--text-secondary);
		line-height: 1.6;
		font-size: 0.95rem;
	}

	/* Interactive Architecture Section */
	.architecture-section {
		padding: 100px 0;
		background: #0c0c0e;
		border-y: 1px solid var(--border-color);
	}
	.arch-container {
		width: 90%;
		max-width: 1200px;
		margin: 0 auto;
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 60px;
		align-items: center;
	}
	.arch-text h2 {
		font-size: 2.5rem;
		font-weight: 800;
		letter-spacing: -1px;
		margin-bottom: 20px;
	}
	.arch-text p {
		color: var(--text-secondary);
		margin-bottom: 40px;
		line-height: 1.6;
	}
	.arch-selector {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}
	.arch-btn {
		display: flex;
		align-items: center;
		gap: 15px;
		background: var(--bg-surface);
		border: 1px solid var(--border-color);
		padding: 12px 20px;
		border-radius: 10px;
		color: var(--text-secondary);
		text-align: left;
		cursor: pointer;
		font-family: inherit;
		transition: var(--transition-smooth);
	}
	.arch-btn:hover {
		background: var(--bg-surface-hover);
		color: var(--text-primary);
	}
	.arch-btn.active {
		border-color: var(--color-primary);
		background: var(--color-primary-light);
		color: var(--text-primary);
	}
	.btn-icon {
		font-size: 1.5rem;
	}
	.btn-text h4 {
		font-weight: 600;
		margin-bottom: 2px;
	}
	.arch-visual {
		display: flex;
		justify-content: center;
	}
	.visual-card {
		background: var(--bg-surface);
		border: 1px solid var(--border-color);
		border-radius: 12px;
		width: 100%;
		max-width: 500px;
		padding: 30px;
		box-shadow: var(--shadow-card);
	}
	.visual-header {
		margin-bottom: 20px;
	}
	.visual-header .tag {
		font-size: 0.7rem;
		font-weight: 700;
		letter-spacing: 1px;
		color: var(--color-secondary-hover);
	}
	.visual-body h3 {
		font-size: 1.5rem;
		font-weight: 700;
		margin-bottom: 15px;
	}
	.visual-body p {
		color: var(--text-secondary);
		margin-bottom: 25px;
		line-height: 1.6;
		font-size: 0.95rem;
	}
	.code-box {
		background: var(--bg-base);
		border: 1px solid var(--border-color);
		padding: 20px;
		border-radius: 8px;
		font-family: var(--font-mono);
		font-size: 0.8rem;
		line-height: 1.5;
		white-space: pre-wrap;
		color: #e4e4e7;
	}
	.code-comment { color: var(--text-muted); }
	.code-keyword { color: #f43f5e; }
	.code-str { color: var(--color-success); }
	.code-val { color: var(--color-secondary-hover); }

	/* Screenshots section */
	.screenshots-section {
		padding: 100px 0;
		width: 90%;
		max-width: 1200px;
		margin: 0 auto;
	}
	.screen-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
		gap: 30px;
	}
	.screen-placeholder {
		background: var(--bg-surface);
		border: 2px dashed var(--border-color);
		border-radius: 12px;
		padding: 40px 25px;
		text-align: center;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 15px;
		min-height: 240px;
		justify-content: center;
		transition: var(--transition-smooth);
	}
	.screen-card:hover .screen-placeholder {
		border-color: var(--color-primary-hover);
		background: var(--bg-surface-hover);
	}
	.screen-placeholder .icon {
		font-size: 2.5rem;
	}
	.screen-placeholder h4 {
		font-size: 1.1rem;
		font-weight: 700;
	}
	.screen-placeholder p {
		font-size: 0.85rem;
		color: var(--text-secondary);
		line-height: 1.5;
	}

	/* Download Section */
	.download-section {
		padding: 100px 0;
		background: radial-gradient(circle at bottom, rgba(139, 92, 246, 0.1) 0%, transparent 60%);
	}
	.download-container {
		width: 90%;
		max-width: 800px;
		margin: 0 auto;
		text-align: center;
	}
	.download-container h2 {
		font-size: 2.8rem;
		font-weight: 800;
		letter-spacing: -1.5px;
		margin-bottom: 20px;
	}
	.download-container p {
		color: var(--text-secondary);
		font-size: 1.15rem;
		margin-bottom: 40px;
	}
	.download-buttons {
		display: flex;
		justify-content: center;
		gap: 20px;
		flex-wrap: wrap;
	}
	.btn-download {
		display: flex;
		align-items: center;
		gap: 15px;
		background: var(--bg-surface);
		border: 1px solid var(--border-color);
		padding: 14px 28px;
		border-radius: 12px;
		color: white;
		text-decoration: none;
		text-align: left;
		transition: var(--transition-bounce);
	}
	.btn-download:hover {
		transform: scale(1.05);
		border-color: var(--color-primary);
		box-shadow: var(--glow-primary);
	}
	.d-icon {
		font-size: 2rem;
	}
	.d-sub {
		display: block;
		font-size: 0.75rem;
		color: var(--text-muted);
	}
	.d-main {
		display: block;
		font-size: 1.1rem;
		font-weight: 700;
	}

	/* Footer */
	footer {
		border-top: 1px solid var(--border-color);
		padding: 40px 0;
	}
	.footer-container {
		width: 90%;
		max-width: 1200px;
		margin: 0 auto;
		display: flex;
		justify-content: space-between;
		align-items: center;
		flex-wrap: wrap;
		gap: 20px;
	}
	.footer-brand {
		display: flex;
		align-items: center;
		gap: 10px;
		font-weight: 700;
	}
	.footer-logo {
		height: 24px;
	}
	footer p {
		font-size: 0.85rem;
		color: var(--text-muted);
	}

	/* Responsive Adjustment */
	@media (max-width: 968px) {
		.hero-container {
			grid-template-columns: 1fr;
			text-align: center;
			gap: 40px;
		}
		.hero-content {
			align-items: center;
		}
		.hero h1 {
			font-size: 2.8rem;
		}
		.arch-container {
			grid-template-columns: 1fr;
		}
		.nav-links {
			display: none; /* simple burger implementation could go here, or just keep it simple */
		}
	}
</style>
