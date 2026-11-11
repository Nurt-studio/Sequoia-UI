<script>
	import { nav } from '../navigation.svelte.js';

	/** @param {string} page */
	function goto(page) {
		nav.page = page;
	}
</script>

<div class="top-bar">
	<div class="top-bar-left">
		<a
			href="/"
			class="top-bar-logo"
			onclick={(e) => {
				e.preventDefault();
				goto('home');
			}}>Sequoia<span class="logo-ui">UI</span></a
		>
		<div class="top-bar-divider"></div>
		<span class="top-bar-version">v0.1.4</span>
	</div>
	<div class="top-bar-nav">
		<a
			href="/"
			class:active={nav.page === 'docs'}
			onclick={(e) => {
				e.preventDefault();
				goto('docs');
			}}>Docs</a
		>
		<a
			href="/"
			class:active={nav.page === 'blog'}
			onclick={(e) => {
				e.preventDefault();
				goto('blog');
			}}
			>Blog</a
		>
		<a
			href="/"
			class:active={nav.page === 'benchmarks'}
			onclick={(e) => {
				e.preventDefault();
				goto('benchmarks');
			}}>Benchmarks</a
		>
		<a href="https://nurt-studio.github.io/Sequoia-UI/guides/widgets.html" target="_blank" rel="noreferrer">Components</a>
		<a href="https://nurt-studio.github.io/Sequoia-UI/getting-started/examples.html" target="_blank" rel="noreferrer">Examples</a>
	</div>
	<div class="top-bar-right">
		<a href="https://docs.rs/sequoia-ui" class="search-box" target="_blank" rel="noreferrer">
			<span>API reference</span>
		</a>
		<a href="https://github.com/Nurt-studio/Sequoia-UI" class="github-btn" aria-label="GitHub repository" target="_blank" rel="noreferrer">
			<svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"
				><path
					d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"
				/></svg
			>
		</a>
	</div>
</div>

<style>
	.top-bar {
		position: sticky;
		top: 0;
		z-index: 100;
		background: rgba(247, 247, 247, 0.85);
		backdrop-filter: blur(12px);
		-webkit-backdrop-filter: blur(12px);
		border-bottom: 1px solid #e5e5e7;
		padding: 0 var(--space-6);
		height: 56px;
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.top-bar-left {
		display: flex;
		align-items: center;
		gap: var(--space-5);
	}
	.top-bar-logo {
		font-size: var(--text-base);
		font-weight: 700;
		color: var(--fg);
		text-decoration: none;
		letter-spacing: var(--tracking-tight);
	}
	.logo-ui {
		color: var(--accent);
	}
	.top-bar-divider {
		width: 1px;
		height: 20px;
		background: var(--border);
	}
	.top-bar-version {
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--accent);
		background: rgba(230, 57, 30, 0.08);
		padding: var(--space-1) var(--space-2);
		border-radius: 20px;
	}
	.top-bar-nav {
		display: flex;
		gap: var(--space-2);
	}
	.top-bar-nav a {
		text-decoration: none;
		color: #555;
		font-size: var(--text-sm);
		font-weight: 500;
		padding: var(--space-2) var(--space-3);
		border-radius: 8px;
		transition: all 0.2s;
	}
	.top-bar-nav a:hover {
		color: #1a1a1a;
		background: #eee;
	}
	.top-bar-nav a.active {
		color: var(--accent);
		background: rgba(230, 57, 30, 0.06);
	}
	.top-bar-right {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}
	.search-box {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 8px;
		padding: var(--space-2) var(--space-3);
		cursor: pointer;
		transition: border-color 0.2s;
	}
	.search-box:hover {
		border-color: #ccc;
	}
	.search-box span {
		font-size: var(--text-sm);
		color: #999;
	}
	.github-btn {
		width: 36px;
		height: 36px;
		border-radius: 8px;
		border: 1px solid #e5e5e7;
		background: #fff;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition: all 0.2s;
		text-decoration: none;
		color: #1a1a1a;
	}
	.github-btn:hover {
		border-color: #ccc;
		background: #f5f5f5;
	}
	@media (max-width: 768px) {
		.top-bar {
			padding: 0 var(--space-4);
		}
		.top-bar-nav {
			display: none;
		}
		.search-box {
			padding: var(--space-2);
		}
	}
</style>