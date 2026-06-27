<script lang="ts">
	let selectedCategory = $state('all');
	let activeLightboxImage = $state<string | null>(null);
	let activeLightboxTitle = $state<string>('');

	const images = [
		// DESKTOP
		{ src: '/assets/screenshots/desktop/desktop_main.webp', category: 'desktop', title: 'Main Dashboard', desc: 'Central workspace featuring dual sidebar lists and distraction-free workspace.' },
		{ src: '/assets/screenshots/desktop/desktop_livePreviewEditor.webp', category: 'desktop', title: 'Live Preview Editor', desc: 'Dual pane showing raw CodeMirror 6 markdown alongside real-time HTML rendering.' },
		{ src: '/assets/screenshots/desktop/desktop_versionHistoryDiff.webp', category: 'desktop', title: 'Version Diff History', desc: 'Detailed visual comparison highlighting deletions (red) and insertions (green) between saved snapshots.' },
		{ src: '/assets/screenshots/desktop/desktop_syncSettings.webp', category: 'desktop', title: 'Sync Configuration', desc: 'Secure WebDAV sync settings page utilizing local hardware keychain integration.' },
		{ src: '/assets/screenshots/desktop/desktop_readingMode.webp', category: 'desktop', title: 'Reading Mode', desc: 'Hides all editing utilities and provides a sleek typography-first publication view.' },
		{ src: '/assets/screenshots/desktop/desktop_encryptedNote.webp', category: 'desktop', title: 'Vault Encryption State', desc: 'Locked vault interface demonstrating AES-GCM secure passphrase validation.' },
		{ src: '/assets/screenshots/desktop/desktop_noteInfo.webp', category: 'desktop', title: 'Note Metadata Panel', desc: 'Inspect note properties, tags, word counts, and system dates.' },
		{ src: '/assets/screenshots/desktop/desktop_syncing.webp', category: 'desktop', title: 'Sync Status Indicators', desc: 'Real-time synchronization logs indicating server handshake status.' },
		{ src: '/assets/screenshots/desktop/desktop_syncReport.webp', category: 'desktop', title: 'Sync Report Dashboard', desc: 'Detailed transaction logs indicating mutated files, etags, and elapsed delta millisecond benchmarks.' },
		{ src: '/assets/screenshots/desktop/desktop_versionHistoryList.webp', category: 'desktop', title: 'History Snapshots List', desc: 'Browsing past physical snapshot entries preserved in the history directory.' },
		{ src: '/assets/screenshots/desktop/desktop_emptyVault.webp', category: 'desktop', title: 'Empty Workspace Onboarding', desc: 'Minimal dashboard layout prompting to import folders or create initial documents.' },
		{ src: '/assets/screenshots/desktop/desktop_localBrowserAccess.webp', category: 'desktop', title: 'Local Axum Server Access', desc: 'Access notes securely from external browser clients via a local authenticated port.' },
		{ src: '/assets/screenshots/desktop/desktop_mainSidebarDetailed.webp', category: 'desktop', title: 'Detailed Sidebar Explorer', desc: 'Deep nested tree explorer showing parent-child directory structures.' },
		{ src: '/assets/screenshots/desktop/desktop_webclipper&webDevicesManagement.webp', category: 'desktop', title: 'Browser Integration Panel', desc: 'Control active web clipper extensions, authorized tokens, and remote web access states.' },

		// MOBILE
		{ src: '/assets/screenshots/mobile/mobile_Main.webp', category: 'mobile', title: 'Mobile Main Interface', desc: 'Native Jetpack Compose dashboard presenting modern note list cards and pinning.' },
		{ src: '/assets/screenshots/mobile/mobile_writeMode.webp', category: 'mobile', title: 'Compose Editor Mode', desc: 'Clean, distraction-free markdown editing interface.' },
		{ src: '/assets/screenshots/mobile/mobile_calendar.webp', category: 'mobile', title: 'Calendar Interface', desc: 'Interactive monthly calendar mapping and launching daily notes.' },
		{ src: '/assets/screenshots/mobile/mobile_syncReport.webp', category: 'mobile', title: 'Sync Diagnostics', desc: 'Detailed performance audit logs of recent sync runs.' },
		{ src: '/assets/screenshots/mobile/mobile_versionHistory.webp', category: 'mobile', title: 'History Logs', desc: 'Inspect and rollback changes directly from the mobile interface.' },
		{ src: '/assets/screenshots/mobile/mobile_sidebar_detailed.webp', category: 'mobile', title: 'Navigation Drawer', desc: 'Quick-access sidebar featuring hierarchical tags, directories, and trash bin.' },
		{ src: '/assets/screenshots/mobile/mobile_syncSettings.webp', category: 'mobile', title: 'Mobile WebDAV Sync', desc: 'Configure sync servers and interval checks directly on the go.' },
		{ src: '/assets/screenshots/mobile/mobile_maintenance.webp', category: 'mobile', title: 'System Diagnostics', desc: 'Self-healing diagnostic utilities to purge orphan attachments and resolve duplicate keys.' },
		{ src: '/assets/screenshots/mobile/mobile_encrytptedNote.webp', category: 'mobile', title: 'Mobile Encryption Shield', desc: 'Passphrase dialogue prompt locking specific note vaults.' },
		{ src: '/assets/screenshots/mobile/mobile_encryptedNotesList.webp', category: 'mobile', title: 'Encrypted Note Previews', desc: 'Indicates locked notes with visual lock icons and disabled subtitle previews.' },
		{ src: '/assets/screenshots/mobile/mobile_sidebar.webp', category: 'mobile', title: 'Compact Mobile Sidebar', desc: 'Sleek navigation menu optimized for one-handed reachability.' },
		{ src: '/assets/screenshots/mobile/mobile_sidebarClean.webp', category: 'mobile', title: 'Clean Navigation Menu', desc: 'Minimalist drawer view showing only the main root vaults.' },
		{ src: '/assets/screenshots/mobile/mobile_readMode.webp', category: 'mobile', title: 'Reader View', desc: 'Formatted markdown render optimized for reading on mobile screens.' },
		{ src: '/assets/screenshots/mobile/mobile_emptyVault.webp', category: 'mobile', title: 'Empty Workspace Feed', desc: 'Onboarding screen when opening an empty local vault.' },
		{ src: '/assets/screenshots/mobile/mobile_noteInfo.webp', category: 'mobile', title: 'Mobile Note Info', desc: 'Displaying word metrics and system dates.' },
		{ src: '/assets/screenshots/mobile/mobile_securitySettings.webp', category: 'mobile', title: 'Security Settings', desc: 'Configure biometric locks and master security keys.' },
		{ src: '/assets/screenshots/mobile/mobile_updates.webp', category: 'mobile', title: 'Zen Update Manager', desc: 'Dynamic version checking and multi-ABI package updates.' },

		// CLIPPER
		{ src: '/assets/screenshots/clipper/clipper_clippingAllPage.webp', category: 'clipper', title: 'Full DOM Capture', desc: 'Extension layout parsing and extracting full web pages.' },
		{ src: '/assets/screenshots/clipper/clipper_clippedNote.webp', category: 'clipper', title: 'Markdown Note Output', desc: 'Preview parsed links, headers, and media before saving.' },
		{ src: '/assets/screenshots/clipper/clipper_clippedNoteFooter.webp', category: 'clipper', title: 'Footer Meta Injector', desc: 'Appends source URLs and clipping date metadata.' },
		{ src: '/assets/screenshots/clipper/clipper_noteClip.webp', category: 'clipper', title: 'Selected Text Clipping', desc: 'Instantly save highlighted article blocks instead of full web pages.' },
		{ src: '/assets/screenshots/clipper/clipper_tokenAuth.webp', category: 'clipper', title: 'Extension Token Authentication', desc: 'Approve browser access requests using cryptographically signed authorization tokens.' }
	];

	const filteredImages = $derived(
		selectedCategory === 'all' 
			? images 
			: images.filter(img => img.category === selectedCategory)
	);

	function openLightbox(src: string, title: string) {
		activeLightboxImage = src;
		activeLightboxTitle = title;
	}

	function closeLightbox() {
		activeLightboxImage = null;
	}
</script>

<svelte:head>
	<title>NodaNotes — Complete Showcases & Interface Gallery</title>
	<meta name="description" content="Browse all 36 detailed screenshots of NodaNotes Desktop, Mobile, and Safari Web Clipper interfaces." />
</svelte:head>

<!-- Header -->
<header class="navbar scrolled">
	<div class="nav-container">
		<a href="/" class="logo-link">
			<img src="/logo.png" alt="NodaNotes" class="logo-img" />
			<span class="logo-text">Noda<span>Notes</span></span>
		</a>
		<nav class="nav-links">
			<a href="/">Back to Home</a>
		</nav>
	</div>
</header>

<!-- Main Gallery View -->
<main class="gallery-page">
	<div class="gallery-header-section">
		<span class="section-label">VISUAL WALKTHROUGH</span>
		<h1>Complete Interface Gallery</h1>
		<p class="subtitle-text">Detailed screenshots illustrating database synchronisation, note editors, diagnostics, and security configurations.</p>
	</div>

	<!-- Category Filters -->
	<div class="category-filters-container">
		<div class="category-tabs">
			<button class="filter-btn" class:active={selectedCategory === 'all'} onclick={() => selectedCategory = 'all'}>All ({images.length})</button>
			<button class="filter-btn" class:active={selectedCategory === 'desktop'} onclick={() => selectedCategory = 'desktop'}>macOS Desktop ({images.filter(i => i.category === 'desktop').length})</button>
			<button class="filter-btn" class:active={selectedCategory === 'mobile'} onclick={() => selectedCategory = 'mobile'}>Android Mobile ({images.filter(i => i.category === 'mobile').length})</button>
			<button class="filter-btn" class:active={selectedCategory === 'clipper'} onclick={() => selectedCategory = 'clipper'}>Web Clipper ({images.filter(i => i.category === 'clipper').length})</button>
		</div>
	</div>

	<!-- Masonry / Responsive Grid -->
	<div class="gallery-showcase-grid">
		{#each filteredImages as img}
			<div class="showcase-card" onclick={() => openLightbox(img.src, img.title)}>
				<div class="img-wrapper">
					<img src={img.src} alt={img.title} loading="lazy" />
					<div class="img-overlay">
						<span>Click to Inspect 🔍</span>
					</div>
				</div>
				<div class="card-details">
					<div class="card-tag" class:tag-desktop={img.category === 'desktop'} class:tag-mobile={img.category === 'mobile'} class:tag-clipper={img.category === 'clipper'}>
						{img.category.toUpperCase()}
					</div>
					<h3>{img.title}</h3>
					<p>{img.desc}</p>
				</div>
			</div>
		{/each}
	</div>
</main>

<!-- Lightbox Overlay -->
{#if activeLightboxImage}
	<div class="lightbox-overlay" onclick={closeLightbox} transition:fade={{ duration: 150 }}>
		<div class="lightbox-modal" onclick={(e) => e.stopPropagation()}>
			<button class="close-btn" onclick={closeLightbox}>×</button>
			<img src={activeLightboxImage} alt={activeLightboxTitle} />
			<div class="lightbox-footer">
				<h3>{activeLightboxTitle}</h3>
			</div>
		</div>
	</div>
{/if}

<footer class="footer">
	<div class="footer-container">
		<span class="footer-logo">NodaNotes</span>
		<span class="footer-copyright">© 2026 NodaNotes. Built with Svelte.</span>
	</div>
</footer>

<style>
	/* Navbar Layout */
	.navbar {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 64px;
		display: flex;
		align-items: center;
		z-index: 100;
		background: #0a0a0a;
		border-bottom: 1px solid var(--border-color);
	}
	.nav-container {
		width: 90%;
		max-width: 1000px;
		margin: 0 auto;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.logo-link {
		display: flex;
		align-items: center;
		gap: 12px;
		text-decoration: none;
		color: var(--text-primary);
	}
	.logo-img {
		height: 32px;
		width: auto;
	}
	.logo-text {
		font-size: 1.25rem;
		font-weight: 700;
		letter-spacing: -0.03em;
	}
	.logo-text span {
		color: var(--accent-orange);
	}
	.nav-links a {
		text-decoration: none;
		color: var(--text-secondary);
		font-weight: 500;
		font-size: 0.9rem;
		transition: var(--transition-minimal);
	}
	.nav-links a:hover {
		color: var(--text-primary);
	}

	/* Main Page Structure */
	.gallery-page {
		width: 90%;
		max-width: 1000px;
		margin: 0 auto;
		padding: 120px 0 60px 0;
	}
	.gallery-header-section {
		text-align: center;
		margin-bottom: 50px;
	}
	.gallery-header-section h1 {
		font-size: 2.5rem;
		font-weight: 800;
		margin-top: 10px;
		letter-spacing: -0.03em;
	}
	.subtitle-text {
		font-size: 1.1rem;
		color: var(--text-secondary);
		max-width: 600px;
		margin: 16px auto 0 auto;
		line-height: 1.6;
	}
	.section-label {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		color: var(--accent-orange);
		letter-spacing: 0.05em;
	}

	/* Category Tabs */
	.category-filters-container {
		display: flex;
		justify-content: center;
		margin-bottom: 40px;
	}
	.category-tabs {
		display: flex;
		gap: 10px;
		background: var(--bg-surface);
		padding: 6px;
		border-radius: 8px;
		border: 1px solid var(--border-color);
		flex-wrap: wrap;
	}
	.filter-btn {
		background: transparent;
		border: none;
		color: var(--text-secondary);
		padding: 8px 16px;
		border-radius: 6px;
		cursor: pointer;
		font-weight: 500;
		font-size: 0.9rem;
		transition: var(--transition-minimal);
	}
	.filter-btn:hover {
		color: var(--text-primary);
	}
	.filter-btn.active {
		background: #1f1f1f;
		color: var(--text-primary);
		border: 1px solid rgba(255,255,255,0.08);
	}

	/* Responsive Cards Grid */
	.gallery-showcase-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: 30px;
	}
	.showcase-card {
		background: var(--bg-surface);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		overflow: hidden;
		cursor: pointer;
		transition: var(--transition-minimal);
		display: flex;
		flex-direction: column;
		height: 100%;
	}
	.showcase-card:hover {
		border-color: var(--border-focus);
		transform: translateY(-4px);
		box-shadow: 0 12px 30px rgba(0, 0, 0, 0.4);
	}
	.img-wrapper {
		position: relative;
		overflow: hidden;
		background: #000;
		border-bottom: 1px solid var(--border-color);
		display: flex;
		align-items: flex-start;
	}
	.showcase-card img {
		width: 100%;
		height: auto;
		object-fit: cover;
		transition: transform 0.3s ease;
	}
	.showcase-card:hover img {
		transform: scale(1.02);
	}
	.img-overlay {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background: rgba(0,0,0,0.5);
		display: flex;
		justify-content: center;
		align-items: center;
		opacity: 0;
		transition: opacity 0.2s ease;
	}
	.showcase-card:hover .img-overlay {
		opacity: 1;
	}
	.img-overlay span {
		background: #18181a;
		border: 1px solid var(--border-color);
		padding: 8px 16px;
		border-radius: 4px;
		font-size: 0.85rem;
		font-weight: 500;
		color: var(--text-primary);
	}

	.card-details {
		padding: 20px;
		display: flex;
		flex-direction: column;
		gap: 10px;
		flex-grow: 1;
	}
	.card-tag {
		align-self: flex-start;
		font-family: var(--font-mono);
		font-size: 0.7rem;
		padding: 2px 6px;
		border-radius: 4px;
		font-weight: 600;
	}
	.tag-desktop {
		background: rgba(10, 132, 255, 0.1);
		color: var(--accent-blue);
		border: 1px solid rgba(10, 132, 255, 0.2);
	}
	.tag-mobile {
		background: rgba(48, 209, 88, 0.1);
		color: var(--accent-green);
		border: 1px solid rgba(48, 209, 88, 0.2);
	}
	.tag-clipper {
		background: rgba(255, 159, 10, 0.1);
		color: var(--accent-orange);
		border: 1px solid rgba(255, 159, 10, 0.2);
	}
	.card-details h3 {
		font-size: 1.15rem;
		font-weight: 700;
		margin: 0;
	}
	.card-details p {
		font-size: 0.85rem;
		color: var(--text-secondary);
		line-height: 1.5;
		margin: 0;
	}

	/* Lightbox System */
	.lightbox-overlay {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background: rgba(0,0,0,0.9);
		z-index: 1000;
		display: flex;
		justify-content: center;
		align-items: center;
		padding: 40px;
	}
	.lightbox-modal {
		position: relative;
		max-width: 90%;
		max-height: 90%;
		display: flex;
		flex-direction: column;
		align-items: center;
		background: #0f0f10;
		border: 1px solid var(--border-color);
		border-radius: 8px;
		overflow: hidden;
	}
	.lightbox-modal img {
		max-width: 100%;
		max-height: 75vh;
		object-fit: contain;
	}
	.close-btn {
		position: absolute;
		top: 16px;
		right: 16px;
		background: #1c1c1e;
		border: 1px solid var(--border-color);
		color: #fff;
		font-size: 1.5rem;
		width: 36px;
		height: 36px;
		border-radius: 50%;
		cursor: pointer;
		display: flex;
		justify-content: center;
		align-items: center;
		z-index: 10;
		transition: var(--transition-minimal);
	}
	.close-btn:hover {
		background: #2c2c2e;
	}
	.lightbox-footer {
		padding: 20px;
		width: 100%;
		border-top: 1px solid var(--border-color);
		text-align: center;
	}
	.lightbox-footer h3 {
		margin: 0;
		font-size: 1.2rem;
		color: var(--text-primary);
	}

	/* Footer styling matching design system */
	.footer {
		border-top: 1px solid var(--border-color);
		padding: 40px 0;
		margin-top: 80px;
	}
	.footer-container {
		width: 90%;
		max-width: 1000px;
		margin: 0 auto;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.footer-logo {
		font-weight: 700;
		font-size: 1rem;
	}
	.footer-copyright {
		font-size: 0.8rem;
		color: var(--text-muted);
	}

	/* Mobile responsive adjustments */
	@media (max-width: 768px) {
		.gallery-page {
			padding-top: 100px;
		}
		.gallery-header-section h1 {
			font-size: 2rem;
		}
		.category-tabs {
			width: 100%;
			justify-content: center;
		}
		.filter-btn {
			flex: 1 1 40%;
			text-align: center;
		}
		.lightbox-overlay {
			padding: 20px;
		}
	}
</style>
