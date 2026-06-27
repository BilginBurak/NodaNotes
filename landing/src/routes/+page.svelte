<script lang="ts">
	import { onMount } from 'svelte';

	let activeSection = $state('sync');
	let scrolled = $state(false);

	const syncCode = `// crates/core/src/sync/delta.rs
let remote_sync_etag = client.propfind_depth_0(url).await?;
if local_etag == remote_sync_etag \x7b
    return Ok(SyncResult::Clean);
\x7d`;

	const keyringCode = `// crates/core/src/auth/keychain.rs
let keyring = Keyring::new("NodaNotes", username)?;
keyring.set_password(secure_password)?;`;

	const filesystemCode = `---
id: "01JXYZ..."
title: "NodaNotes Architecture"
updated_at: "2026-06-23T03:30:00Z"
tags: ["rust", "architecture"]
---`;

	const historyCode = `.noda/history/[NoteID]_[YYYYMMDD-HHMMSS]_[reason].md`;

	const technicalSpecs = [
		{
			id: 'sync',
			title: '0.1s Fast-Check Sync',
			description: 'Checks remote ETags using WebDAV PROPFIND (Depth: 1). If the signatures of the empty sync files match, it exits in 100 milliseconds without executing a full scan.',
			code: syncCode
		},
		{
			id: 'filesystem',
			title: 'Local-First Markdown Store',
			description: 'The disk is the single source of truth. Notes are plain-text .md files using structured YAML frontmatter. If the SQLite index is deleted, Noda parses the vault and reconstructs the search index automatically.',
			code: filesystemCode
		},
		{
			id: 'encryption',
			title: 'OS-Level Keychain integration',
			description: 'Plain-text credentials are never saved to disk. macOS Keyring and Android EncryptedSharedPreferences store WebDAV passwords using native hardware APIs.',
			code: keyringCode
		},
		{
			id: 'history',
			title: 'Flat History Snapshots',
			description: 'Version control operates directly in the history directory without database overhead. Snapshots are written as flat files and mapped directly to visualize inline code diffs.',
			code: historyCode
		}
	];

	let activeScreenshotTab = $state('desktop');

	onMount(() => {
		const handleScroll = () => {
			scrolled = window.scrollY > 40;
		};
		window.addEventListener('scroll', handleScroll);
		return () => window.removeEventListener('scroll', handleScroll);
	});
</script>

<svelte:head>
	<title>NodaNotes — High Performance Local-First Markdown Note Application</title>
	<meta name="description" content="NodaNotes is a secure, local-first markdown note-taking application powered by a standalone Rust core engine. Synced via WebDAV in 0.1s with hardware-level encryption." />
	<meta name="keywords" content="NodaNotes, markdown notes, local first note app, WebDAV sync, Rust notes app, Tauri notes application, private markdown editor, offline notes tool" />
	
	<!-- Canonical Link -->
	<link rel="canonical" href="https://nodanotes.netlify.app/" />

	<!-- Open Graph / Facebook -->
	<meta property="og:type" content="website" />
	<meta property="og:url" content="https://nodanotes.netlify.app/" />
	<meta property="og:title" content="NodaNotes — High Performance Local-First Markdown Note Application" />
	<meta property="og:description" content="Secure, local-first markdown note-taking powered by a standalone Rust core engine. Synced via WebDAV in 0.1s with hardware-level encryption." />
	<meta property="og:image" content="https://nodanotes.netlify.app/logo.png" />

	<!-- Twitter -->
	<meta property="twitter:card" content="summary_large_image" />
	<meta property="twitter:url" content="https://nodanotes.netlify.app/" />
	<meta property="twitter:title" content="NodaNotes — High Performance Local-First Markdown Note Application" />
	<meta property="twitter:description" content="Secure, local-first markdown note-taking powered by a standalone Rust core engine. Synced via WebDAV in 0.1s with hardware-level encryption." />
	<meta property="twitter:image" content="https://nodanotes.netlify.app/logo.png" />

	<meta name="robots" content="index, follow" />
</svelte:head>

<!-- Premium Editorial Navbar -->
<header class="navbar" class:scrolled>
	<div class="nav-container">
		<a href="#" class="logo-link">
			<img src="/logo.png" alt="NodaNotes" class="logo-img" />
			<span class="logo-text">Noda<span>Notes</span></span>
		</a>
		<nav class="nav-links">
			<a href="#features">Features</a>
			<a href="#architecture">Architecture</a>
			<a href="#specs">Specifications</a>
			<a href="#download" class="nav-cta">Download</a>
		</nav>
	</div>
</header>

<!-- Hero Section: Editorial Typography and Layout -->
<section class="hero-section">
	<div class="hero-grid">
		<div class="hero-text-block">
			<div class="version-tag">Version 1.0 — Pure Rust Core</div>
			<h1>Local markdown notes, synced in 0.1 seconds.</h1>
			<p class="hero-description">
				A filesystem-first markdown note-taking environment. Built with a standalone Rust core, Tauri, and native Kotlin. Secure, offline-first, and completely owned by you.
			</p>
			<div class="hero-ctas">
				<a href="#download" class="btn-main">Get NodaNotes</a>
				<a href="#architecture" class="btn-secondary">Technical Specs</a>
			</div>
		</div>

		<div class="hero-preview-block">
			<div class="terminal-mockup">
				<div class="terminal-header">
					<span class="terminal-dot"></span>
					<span class="terminal-dot"></span>
					<span class="terminal-dot"></span>
					<span class="terminal-title">crates/core/src/sync/engine.rs</span>
				</div>
				<div class="terminal-body">
					<pre><code><span class="line-num">1</span> <span class="t-keyword">pub async fn</span> <span class="t-func">sync_vault</span>(state: &amp;VaultState) -> <span class="t-type">Result</span>&lt;SyncReport&gt; &#123;
<span class="line-num">2</span>     <span class="t-keyword">let</span> start = Instant::now();
<span class="line-num">3</span>     <span class="t-keyword">let</span> is_dirty = state.db.is_dirty().await?;
<span class="line-num">4</span>     
<span class="line-num">5</span>     <span class="t-keyword">if</span> !is_dirty &#123;
<span class="line-num">6</span>         <span class="t-keyword">let</span> fast_check = sync::fast_check(&amp;state.config).await?;
<span class="line-num">7</span>         <span class="t-keyword">if</span> fast_check == FastCheck::Match &#123;
<span class="line-num">8</span>             log::info!("Sync completed in &#123;:.2?&#125;", start.elapsed());
<span class="line-num">9</span>             <span class="t-keyword">return</span> <span class="t-val">Ok</span>(SyncReport::Unchanged);
<span class="line-num">10</span>        &#125;
<span class="line-num">11</span>    &#125;
<span class="line-num">12</span>    <span class="t-func">run_delta_sync</span>(state).await
<span class="line-num">13</span> &#125;</code></pre>
				</div>
			</div>
		</div>
	</div>
</section>

<!-- Product Value Architecture (No hype, exact metrics) -->
<section id="features" class="section-container border-top">
	<div class="editorial-row">
		<div class="col-title">
			<span class="section-label">01 / CAPABILITIES</span>
			<h2>No servers. No vendor lock-in.</h2>
		</div>
		<div class="col-content">
			<p class="large-para">
				NodaNotes writes files directly to your local drive. Your notes exist outside the application in standard directories.
			</p>
		</div>
	</div>

	<div class="grid-three-col">
		<div class="grid-card">
			<span class="card-num">01.</span>
			<h3>Fast-Check Engine</h3>
			<p>Evaluates local database changes and queries remote WebDAV signatures. Exits in 0.1 seconds when no modifications are present on either side.</p>
		</div>
		<div class="grid-card">
			<span class="card-num">02.</span>
			<h3>Zero-Metadata Database</h3>
			<p>SQLite acts exclusively as a search and listing cache. Deleting the cache database does not cause data loss; the system rebuilds on next launch.</p>
		</div>
		<div class="grid-card">
			<span class="card-num">03.</span>
			<h3>Structured Version Control</h3>
			<p>Every note change, editor blur, and manual save writes a flat snapshot to the local history directory. View visual diffs instantly.</p>
		</div>
	</div>
</section>

<!-- Dual-Layer Architecture Section -->
<section id="architecture" class="section-container border-top bg-dark">
	<div class="editorial-row">
		<div class="col-title">
			<span class="section-label">02 / CORE DESIGN</span>
			<h2>One Rust Core. Native interfaces.</h2>
		</div>
		<div class="col-content">
			<p>
				Unlike web-based editors wrapped in heavy runtimes, NodaNotes runs a single compiled Rust binary as its engine. macOS interacts via Tauri IPC commands, while Android loads the engine via the Java Native Interface (JNI).
			</p>
		</div>
	</div>

	<div class="interactive-architecture-block">
		<div class="spec-tabs">
			{#each technicalSpecs as spec}
				<button 
					class="spec-tab-btn" 
					class:active={activeSection === spec.id}
					onclick={() => activeSection = spec.id}
				>
					<span class="spec-tab-title">{spec.title}</span>
				</button>
			{/each}
		</div>

		<div class="spec-viewer">
			{#each technicalSpecs as spec}
				{#if activeSection === spec.id}
					<div class="spec-info-layout">
						<div class="spec-text-details">
							<h3>{spec.title}</h3>
							<p>{spec.description}</p>
						</div>
						<div class="spec-code-details">
							<pre><code>{spec.code}</code></pre>
						</div>
					</div>
				{/if}
			{/each}
		</div>
	</div>
</section>

<!-- Interactive Screenshot Showcases Section -->
<section id="specs" class="section-container border-top">
	<div class="editorial-row">
		<div class="col-title">
			<span class="section-label">03 / INTERFACE SHOWCASES</span>
			<h2>Visualizing NodaNotes</h2>
		</div>
		<div class="col-content">
			<p>Explore the premium, highly-polished user interface of NodaNotes across desktop, mobile, and browser extensions.</p>
		</div>
	</div>

	<div class="screenshot-gallery-container">
		<div class="gallery-tabs">
			<button class="gallery-tab-btn" class:active={activeScreenshotTab === 'desktop'} onclick={() => activeScreenshotTab = 'desktop'}>macOS Desktop</button>
			<button class="gallery-tab-btn" class:active={activeScreenshotTab === 'mobile'} onclick={() => activeScreenshotTab = 'mobile'}>Android Mobile</button>
			<button class="gallery-tab-btn" class:active={activeScreenshotTab === 'clipper'} onclick={() => activeScreenshotTab = 'clipper'}>Web Clipper</button>
		</div>

		<div class="gallery-content">
			{#if activeScreenshotTab === 'desktop'}
				<div class="desktop-gallery-grid">
					<div class="gallery-item large-item">
						<img src="/assets/screenshots/desktop/desktop_main.png" alt="Desktop Main Interface" />
						<div class="gallery-item-info">
							<h4>Dual-Panel Workspace</h4>
							<p>Clean layout containing files sidebar, note lists, and the core distraction-free editor.</p>
						</div>
					</div>
					<div class="gallery-item">
						<img src="/assets/screenshots/desktop/desktop_livePreviewEditor.png" alt="Live Preview Markdown Editor" />
						<div class="gallery-item-info">
							<h4>CodeMirror 6 Editor</h4>
							<p>Rich syntax highlighting alongside live side-by-side preview rendering.</p>
						</div>
					</div>
					<div class="gallery-item">
						<img src="/assets/screenshots/desktop/desktop_versionHistoryDiff.png" alt="Visual Diff History" />
						<div class="gallery-item-info">
							<h4>Git-Style Diff Viewer</h4>
							<p>Visually compare line changes between different snapshots in note history.</p>
						</div>
					</div>
					<div class="gallery-item">
						<img src="/assets/screenshots/desktop/desktop_syncSettings.png" alt="WebDAV Sync Settings" />
						<div class="gallery-item-info">
							<h4>Cloud Synchronization</h4>
							<p>Fast WebDAV connection setup using hardware-level keychain tokens.</p>
						</div>
					</div>
					<div class="gallery-item">
						<img src="/assets/screenshots/desktop/desktop_readingMode.png" alt="Reading Mode Preview" />
						<div class="gallery-item-info">
							<h4>Pure Reading Mode</h4>
							<p>Hides editing interfaces entirely for a premium publication-like viewing experience.</p>
						</div>
					</div>
				</div>
			{:else if activeScreenshotTab === 'mobile'}
				<div class="mobile-gallery-grid">
					<div class="gallery-item mobile-item">
						<img src="/assets/screenshots/mobile/mobile_Main.jpg" alt="Mobile Main Notes List" />
						<div class="gallery-item-info">
							<h4>Main Workspace</h4>
							<p>Clean dashboard with instant search, pinning, and folder hierarchy navigation.</p>
						</div>
					</div>
					<div class="gallery-item mobile-item">
						<img src="/assets/screenshots/mobile/mobile_writeMode.jpg" alt="Jetpack Compose Editor" />
						<div class="gallery-item-info">
							<h4>Distraction-Free Editor</h4>
							<p>Writing mode with system-integrated Monet primary color themes.</p>
						</div>
					</div>
					<div class="gallery-item mobile-item">
						<img src="/assets/screenshots/mobile/mobile_calendar.jpg" alt="Interactive Calendar Dialogue" />
						<div class="gallery-item-info">
							<h4>Interactive Calendar</h4>
							<p>View daily logs and reflections directly mapped onto a calendar interface.</p>
						</div>
					</div>
					<div class="gallery-item mobile-item">
						<img src="/assets/screenshots/mobile/mobile_syncReport.jpg" alt="Sync Diagnostics" />
						<div class="gallery-item-info">
							<h4>Sync Diagnostics</h4>
							<p>Detailed performance report of zero-byte WebDAV sync operations.</p>
						</div>
					</div>
					<div class="gallery-item mobile-item">
						<img src="/assets/screenshots/mobile/mobile_versionHistory.jpg" alt="Mobile Version History" />
						<div class="gallery-item-info">
							<h4>Flat History Logs</h4>
							<p>Review and restore from physical snapshots saved in the local history folder.</p>
						</div>
					</div>
					<div class="gallery-item mobile-item">
						<img src="/assets/screenshots/mobile/mobile_sidebar_detailed.jpg" alt="Mobile Sidebar" />
						<div class="gallery-item-info">
							<h4>Sidebar Navigation</h4>
							<p>Quick access to tags, folders, trash bin, and settings pages.</p>
						</div>
					</div>
				</div>
			{:else if activeScreenshotTab === 'clipper'}
				<div class="clipper-gallery-grid">
					<div class="gallery-item large-item">
						<img src="/assets/screenshots/clipper/clipper_clippingAllPage.png" alt="Safari Web Clipper Parsing" />
						<div class="gallery-item-info">
							<h4>Full DOM Parsing</h4>
							<p>Select, filter, and extract whole web pages into optimized markdown content.</p>
						</div>
					</div>
					<div class="gallery-item large-item">
						<img src="/assets/screenshots/clipper/clipper_clippedNote.png" alt="Parsed Markdown Preview" />
						<div class="gallery-item-info">
							<h4>Formatted Markdown Output</h4>
							<p>Clean rendering preview of parsed links, images, tables, and frontmatter metadata.</p>
						</div>
					</div>
				</div>
			{/if}
		</div>
	</div>
</section>

<!-- Performance Benchmarks & Metrics (Editorial Technical Table) -->
<section class="section-container border-top">
	<div class="editorial-row">
		<div class="col-title">
			<span class="section-label">04 / PERFORMANCE METRICS</span>
			<h2>Benchmarked metrics on actual vaults.</h2>
		</div>
		<div class="col-content">
			<p>
				Tested on standard hardware configurations using a vault containing 5,000 active markdown files (average 2.4KB size per file).
			</p>
		</div>
	</div>

	<div class="benchmark-table-wrapper">
		<table class="benchmark-table">
			<thead>
				<tr>
					<th>Metric Description</th>
					<th>Target Latency</th>
					<th>Actual Performance</th>
					<th>Bottleneck Area</th>
				</tr>
			</thead>
			<tbody>
				<tr>
					<td>Cold Startup & SQLite Scan Rebuild</td>
					<td>&lt; 2000ms</td>
					<td>450ms</td>
					<td>Disk I/O Bound</td>
				</tr>
				<tr>
					<td>FTS5 Full-Text Match Search Latency</td>
					<td>&lt; 100ms</td>
					<td>12ms</td>
					<td>CPU Bound</td>
				</tr>
				<tr>
					<td>WebDAV Local Signature Sync Evaluation</td>
					<td>&lt; 200ms</td>
					<td>100ms</td>
					<td>Network Bound</td>
				</tr>
				<tr>
					<td>Active Typing Latency (CodeMirror 6)</td>
					<td>Imperceptible</td>
					<td>&lt; 4ms</td>
					<td>GPU Bound</td>
				</tr>
			</tbody>
		</table>
	</div>
</section>

<!-- Sync Lifecycle & Self Healing Sequence Flow -->
<section class="section-container border-top bg-dark">
	<div class="editorial-row">
		<div class="col-title">
			<span class="section-label">05 / SELF-HEALING PROTOCOLS</span>
			<h2>WebDAV Sync Lifecycle and Conflict Resolution</h2>
		</div>
		<div class="col-content">
			<p>
				NodaNotes implements a structured decision matrix during sensor ticks to guarantee remote state integrity without creating infinite bandwidth loops.
			</p>
		</div>
	</div>

	<div class="flow-layout">
		<div class="flow-step">
			<span class="flow-step-num">Step 01</span>
			<h4>State Evaluation</h4>
			<p>The client reads the local transaction queue from queue.json. If clean, it initiates a WebDAV PROPFIND query with Depth: 1 targeting the remote signatures.</p>
		</div>
		<div class="flow-step">
			<span class="flow-step-num">Step 02</span>
			<h4>Zero-Byte Evaluation</h4>
			<p>If the remote ETag signature matches local remote_state.json cache, the thread shuts down in 100ms. No database transaction is triggered.</p>
		</div>
		<div class="flow-step">
			<span class="flow-step-num">Step 03</span>
			<h4>Delta Synchronization</h4>
			<p>If mismatch exists, the engine maps file paths. In conflicts where both client and server files modified, Noda writes local copy, redirects server copy to conflicts folder, and logs diagnostic states.</p>
		</div>
	</div>
</section>

<!-- Vault Diagnostics & Storage Management -->
<section class="section-container border-top">
	<div class="editorial-row">
		<div class="col-title">
			<span class="section-label">06 / VAULT DIAGNOSTICS</span>
			<h2>Self-healing storage architecture.</h2>
		</div>
		<div class="col-content">
			<p class="large-para">
				NodaNotes is engineered for long-term vault durability. The Rust core actively monitors filesystem integrity and provides embedded diagnostics tools.
			</p>
		</div>
	</div>

	<div class="grid-three-col">
		<div class="grid-card">
			<span class="card-num">01.</span>
			<h3>Orphaned Assets Cleaner</h3>
			<p>Scans the .noda/attachments/ folder for binary media assets that are no longer linked within any active markdown document, allowing one-click storage reclamation.</p>
		</div>
		<div class="grid-card">
			<span class="card-num">02.</span>
			<h3>Duplicate ID Resolver</h3>
			<p>Scans note YAML metadata headers recursively. If multiple markdown files share the same ULID identifier, Noda groups them, enabling safe content comparison and physical path deduplication.</p>
		</div>
		<div class="grid-card">
			<span class="card-num">03.</span>
			<h3>Orphaned History & Conflicts</h3>
			<p>Cleans up remnant history snapshot files and unresolved conflict notes belonging to documents that have been permanently deleted from the trash directory.</p>
		</div>
	</div>
</section>

<!-- Security & Threat Model Specification -->
<section class="section-container border-top bg-dark">
	<div class="editorial-row">
		<div class="col-title">
			<span class="section-label">07 / THREAT MODEL & SECURITY</span>
			<h2>No telemetry. Zero data transit footprint.</h2>
		</div>
		<div class="col-content">
			<p class="large-para">
				Our security architecture is structured around local-first data protection. NodaNotes does not run external metrics services or crash collection threads.
			</p>
		</div>
	</div>

	<div class="flow-layout">
		<div class="flow-step">
			<span class="flow-step-num">Sandbox Bounds</span>
			<h4>App-Level Separation</h4>
			<p>The Tauri shell enforces strict CSP parameters. Custom protocol handlers registered to load attachments validate relative paths to prevent path traversal directory attacks.</p>
		</div>
		<div class="flow-step">
			<span class="flow-step-num">Credential Isolation</span>
			<h4>Hardware Keyring</h4>
			<p>WebDAV password tokens are stored in the OS Keychain using the Rust security-framework crate, preventing storage in standard plain-text configs.</p>
		</div>
		<div class="flow-step">
			<span class="flow-step-num">Zero Cloud Metadata</span>
			<h4>Offline Resilience</h4>
			<p>Notes are never cached on Noda infrastructure. The synchronization runs point-to-point between your client and your personal WebDAV server.</p>
		</div>
	</div>
</section>

<!-- Download Section -->
<section id="download" class="section-container border-top bg-dark download-block">
	<h2>Build from source or download binaries.</h2>
	<p>Open-source, local-first note environment.</p>
	
	<div class="download-grid">
		<a href="https://github.com" class="download-link-card">
			<span class="download-platform">macOS App Bundle</span>
			<span class="download-meta">Requires macOS 14+ / Intel or Apple Silicon</span>
		</a>
		<a href="https://github.com" class="download-link-card">
			<span class="download-platform">Android APK Bundle</span>
			<span class="download-meta">Requires Android API 36+ (arm64-v8a target)</span>
		</a>
	</div>
</section>

<footer class="footer">
	<div class="footer-container">
		<span class="footer-logo">NodaNotes</span>
		<span class="footer-copyright">© 2026 NodaNotes. Built with Rust.</span>
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
		transition: var(--transition-minimal);
	}
	.navbar.scrolled {
		background: #0a0a0a;
		border-bottom: 1px solid var(--border-color);
		height: 64px;
	}
	.nav-container {
		width: 90%;
		max-width: 1000px; /* Reduced from 1200px to align with page width */
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
	.nav-links {
		display: flex;
		align-items: center;
		gap: 32px;
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
	.nav-links .nav-cta {
		padding: 6px 14px;
		background: #1f1f1f;
		color: #fff;
		border-radius: 4px;
		border: 1px solid var(--border-color);
	}
	.nav-links .nav-cta:hover {
		background: #2a2a2a;
		border-color: var(--border-focus);
	}

	/* Hero Section */
	.hero-section {
		min-height: 90vh; /* Large viewpoint presence */
		display: flex;
		align-items: center;
		padding: 180px 0 100px 0; /* Substantial padding to give space and look imposing */
	}
	.hero-grid {
		width: 90%;
		max-width: 1000px;
		margin: 0 auto;
		display: grid;
		grid-template-columns: 1.1fr 0.9fr; /* Slightly larger text-block layout */
		gap: 60px;
		align-items: center;
	}
	.hero-text-block {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
	}
	.version-tag {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		color: var(--accent-blue);
		margin-bottom: 24px;
		border: 1px solid rgba(10, 132, 255, 0.3);
		padding: 4px 10px;
		border-radius: 4px;
	}
	.hero-description {
		margin-top: 30px;
		font-size: 1.2rem;
		line-height: 1.6;
		max-width: 520px;
	}
	.hero-ctas {
		display: flex;
		gap: 16px;
		margin-top: 40px;
	}
	.btn-main {
		padding: 12px 24px;
		background: var(--text-primary);
		color: var(--bg-base);
		font-weight: 600;
		text-decoration: none;
		border-radius: 4px;
		transition: var(--transition-minimal);
	}
	.btn-main:hover {
		background: #e5e5e5;
	}
	.btn-secondary {
		padding: 12px 24px;
		background: transparent;
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		font-weight: 500;
		text-decoration: none;
		border-radius: 4px;
		transition: var(--transition-minimal);
	}
	.btn-secondary:hover {
		border-color: var(--border-focus);
	}

	/* Terminal Preview */
	.terminal-mockup {
		background: var(--bg-surface);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		overflow: hidden;
		box-shadow: 0 30px 60px rgba(0,0,0,0.5);
	}
	.terminal-header {
		height: 40px;
		background: #181818;
		border-bottom: 1px solid var(--border-color);
		display: flex;
		align-items: center;
		padding: 0 16px;
		gap: 6px;
	}
	.terminal-dot {
		width: 8px;
		height: 8px;
		background: #333;
		border-radius: 50%;
	}
	.terminal-title {
		margin-left: 12px;
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--text-secondary);
	}
	.terminal-body {
		padding: 24px;
		font-family: var(--font-mono);
		font-size: 0.8rem;
		line-height: 1.6;
		overflow-x: auto;
	}
	.line-num {
		color: var(--text-muted);
		margin-right: 12px;
		display: inline-block;
		width: 14px;
		text-align: right;
		user-select: none;
	}
	.t-keyword { color: var(--accent-orange); }
	.t-func { color: var(--accent-blue); }
	.t-type { color: var(--accent-green); }
	.t-str { color: var(--text-primary); }
	.t-val { color: var(--accent-orange); }

	/* General Layout: Editorial Layouts */
	.section-container {
		width: 90%;
		max-width: 1000px; /* Reduced from 1200px for a more compact read */
		margin: 0 auto;
		padding: 60px 0;   /* Reduced padding from 100px to 60px */
	}
	.border-top {
		border-top: 1px solid var(--border-color);
	}
	.bg-dark {
		background: #111112; 
		max-width: 100%;
		width: 100%;
		padding-left: 5%;
		padding-right: 5%;
	}
	.bg-dark > .section-container {
		padding: 60px 0;   /* Aligned with standard section padding */
	}
	.editorial-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 40px;         /* Reduced from 80px */
		margin-bottom: 40px; /* Reduced from 60px */
		align-items: flex-start;
	}
	.section-label {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		color: var(--text-secondary);
		display: block;
		margin-bottom: 16px;
	}
	.large-para {
		font-size: 1.35rem;
		line-height: 1.5;
		color: var(--text-primary);
	}

	/* Grid Layouts */
	.grid-three-col {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 40px;
	}
	.grid-card {
		border-left: 1px solid var(--border-color);
		padding-left: 24px;
	}
	.card-num {
		font-family: var(--font-mono);
		font-size: 0.85rem;
		color: var(--accent-orange);
		display: block;
		margin-bottom: 20px;
	}
	.grid-card h3 {
		margin-bottom: 12px;
	}
	.grid-card p {
		font-size: 0.95rem;
		line-height: 1.6;
	}

	/* Technical Spec Interactive Blocks */
	.interactive-architecture-block {
		margin-top: 40px;
		border: 1px solid var(--border-color);
		border-radius: 8px;
		overflow: hidden;
		background: var(--bg-base);
	}
	.spec-tabs {
		display: flex;
		border-bottom: 1px solid var(--border-color);
		background: #0c0c0c;
	}
	.spec-tab-btn {
		flex: 1;
		padding: 16px;
		background: transparent;
		border: none;
		border-right: 1px solid var(--border-color);
		color: var(--text-secondary);
		font-family: inherit;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		text-align: center;
		transition: var(--transition-minimal);
	}
	.spec-tab-btn:last-child {
		border-right: none;
	}
	.spec-tab-btn.active {
		background: var(--bg-surface);
		color: var(--text-primary);
		font-weight: 600;
	}
	.spec-viewer {
		padding: 40px;
		background: var(--bg-surface);
	}
	.spec-info-layout {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 60px;
		align-items: center;
	}
	.spec-text-details h3 {
		font-size: 1.5rem;
		margin-bottom: 16px;
	}
	.spec-text-details p {
		line-height: 1.6;
	}
	.spec-code-details pre {
		background: var(--bg-base);
		border: 1px solid var(--border-color);
		padding: 24px;
		border-radius: 6px;
		font-family: var(--font-mono);
		font-size: 0.8rem;
		color: var(--text-primary);
		overflow-x: auto;
		white-space: pre;
	}

	/* Screenshot Gallery */
	.screenshot-gallery-container {
		display: flex;
		flex-direction: column;
		gap: 30px;
		margin-top: 40px;
	}
	.gallery-tabs {
		display: flex;
		gap: 12px;
		border-bottom: 1px solid var(--border-color);
		padding-bottom: 16px;
	}
	.gallery-tab-btn {
		background: transparent;
		border: 1px solid var(--border-color);
		color: var(--text-secondary);
		padding: 10px 20px;
		border-radius: 4px;
		font-weight: 500;
		font-size: 0.9rem;
		cursor: pointer;
		transition: var(--transition-minimal);
	}
	.gallery-tab-btn:hover {
		color: var(--text-primary);
		border-color: var(--border-focus);
	}
	.gallery-tab-btn.active {
		background: var(--text-primary);
		color: var(--bg-base);
		border-color: var(--text-primary);
		font-weight: 600;
	}
	.gallery-content {
		width: 100%;
	}
	.desktop-gallery-grid, .clipper-gallery-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 30px;
	}
	.mobile-gallery-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 30px;
	}
	.gallery-item.large-item {
		grid-column: span 2;
	}
	.gallery-item {
		background: var(--bg-surface);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		overflow: hidden;
		transition: var(--transition-minimal);
		display: flex;
		flex-direction: column;
		height: 100%;
	}
	.gallery-item:hover {
		border-color: var(--border-focus);
		transform: translateY(-2px);
		box-shadow: 0 12px 30px rgba(0, 0, 0, 0.4);
	}
	.gallery-item img {
		width: 100%;
		height: auto;
		object-fit: cover;
		border-bottom: 1px solid var(--border-color);
		background: #000;
	}
	/* Limit portrait mobile images from being excessively tall */
	.mobile-gallery-grid .gallery-item img {
		max-height: 520px;
		object-position: top;
	}
	.gallery-item-info {
		padding: 20px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.gallery-item-info h4 {
		font-size: 1.1rem;
		font-weight: 700;
		color: var(--text-primary);
		margin: 0;
	}
	.gallery-item-info p {
		font-size: 0.85rem;
		color: var(--text-secondary);
		line-height: 1.5;
		margin: 0;
	}

	/* Download Block */
	.download-block {
		text-align: center;
	}
	.download-block p {
		margin-bottom: 50px;
	}
	.download-grid {
		display: flex;
		justify-content: center;
		gap: 24px;
		flex-wrap: wrap;
	}
	.download-link-card {
		display: flex;
		flex-direction: column;
		background: var(--bg-surface);
		border: 1px solid var(--border-color);
		padding: 24px 40px;
		border-radius: 6px;
		text-decoration: none;
		text-align: left;
		min-width: 320px;
		transition: var(--transition-minimal);
	}
	.download-link-card:hover {
		border-color: var(--border-focus);
		transform: translateY(-2px);
	}
	.download-platform {
		font-weight: 700;
		font-size: 1.1rem;
		color: var(--text-primary);
		margin-bottom: 6px;
	}
	.download-meta {
		font-size: 0.8rem;
		color: var(--text-secondary);
	}

	/* Footer */
	.footer {
		border-top: 1px solid var(--border-color);
		padding: 40px 0;
	}
	.footer-container {
		width: 90%;
		max-width: 1000px; /* Reduced from 1200px to align with page width */
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

	/* Benchmark Table Styles */
	.benchmark-table-wrapper {
		margin-top: 40px;
		border: 1px solid var(--border-color);
		border-radius: 8px;
		overflow: hidden;
	}
	.benchmark-table {
		width: 100%;
		border-collapse: collapse;
		text-align: left;
		font-size: 0.95rem;
	}
	.benchmark-table th, .benchmark-table td {
		padding: 16px 24px;
		border-bottom: 1px solid var(--border-color);
	}
	.benchmark-table th {
		background: #0f0f0f;
		font-family: var(--font-mono);
		font-size: 0.8rem;
		color: var(--text-secondary);
		font-weight: 500;
	}
	.benchmark-table tbody tr:last-child td {
		border-bottom: none;
	}
	.benchmark-table td:nth-child(3) {
		font-family: var(--font-mono);
		color: var(--noda-teal);
		font-weight: 600;
	}

	/* Flow steps layout */
	.flow-layout {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 40px;
		margin-top: 40px;
	}
	.flow-step {
		background: var(--bg-surface);
		border: 1px solid var(--border-color);
		padding: 30px;
		border-radius: 6px;
		position: relative;
	}
	.flow-step-num {
		font-family: var(--font-mono);
		font-size: 0.8rem;
		color: var(--noda-orange);
		display: block;
		margin-bottom: 16px;
	}
	.flow-step h4 {
		font-size: 1.15rem;
		margin-bottom: 12px;
	}
	.flow-step p {
		font-size: 0.9rem;
		line-height: 1.5;
	}

	/* Responsive design adjustments */
	@media (max-width: 968px) {
		.hero-grid, .editorial-row, .grid-three-col, .spec-info-layout, .specs-grid, .flow-layout {
			grid-template-columns: 1fr !important;
			gap: 30px !important;
		}
		.navbar .nav-links {
			display: none;
		}
		.hero-section {
			padding-top: 100px;
			padding-bottom: 60px;
			min-height: auto;
		}
		.hero-grid {
			text-align: center;
			display: flex;
			flex-direction: column;
			gap: 40px;
		}
		.hero-text-block {
			align-items: center;
			text-align: center;
		}
		.hero-description {
			max-width: 100%;
		}
		.hero-ctas {
			justify-content: center;
			width: 100%;
		}
		.terminal-mockup {
			width: 100%;
			max-width: 100%;
			box-sizing: border-box;
		}
		.terminal-body {
			padding: 16px;
			font-size: 0.75rem;
		}
		.spec-tabs {
			flex-wrap: wrap; /* Prevent tabs from overflowing horizontally */
		}
		.spec-tab-btn {
			flex: 1 1 50%; /* Make tabs stack into a grid of 2x2 on mobile */
			border-bottom: 1px solid var(--border-color);
			border-right: 1px solid var(--border-color);
			padding: 12px;
			font-size: 0.85rem;
		}
		.spec-tab-btn:nth-child(2n) {
			border-right: none;
		}
		.spec-viewer {
			padding: 20px;
		}
		.spec-info-layout {
			display: flex;
			flex-direction: column;
			gap: 20px;
		}
		.spec-code-details {
			width: 100%;
		}
		.spec-code-details pre {
			padding: 16px;
			font-size: 0.75rem;
			max-width: 100%;
			overflow-x: auto;
			white-space: pre-wrap; /* Wrap lines to prevent code block overflow */
		}
		.benchmark-table-wrapper {
			overflow-x: auto;
			width: 100%;
		}
		.download-link-card {
			min-width: 100%;
		}
	}
	@media (max-width: 480px) {
		.hero-ctas {
			flex-direction: column;
			width: 100%;
		}
		.btn-main, .btn-secondary {
			width: 100%;
			text-align: center;
		}
		.terminal-body {
			padding: 12px;
			font-size: 0.7rem;
		}
		.spec-tab-btn {
			flex: 1 1 100%; /* Stack tabs vertically on tiny mobile screens */
			border-right: none;
		}
		.flow-step, .spec-item {
			padding: 20px;
		}
	}
</style>
