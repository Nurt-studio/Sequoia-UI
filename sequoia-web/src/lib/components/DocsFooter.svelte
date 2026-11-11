<script>
	import { nav } from '../navigation.svelte.js';
</script>

<footer>
	<div class="footer-inner">
		<div class="footer-logo">SequoiaUI</div>
		<div class="footer-links">
			<a
				href="/"
				onclick={(e) => {
					e.preventDefault();
					nav.page = 'docs';
				}}>Documentation</a
			>
			<a
				href="/"
				onclick={(e) => {
					e.preventDefault();
					nav.page = 'blog';
				}}				>Release notes</a
			>
			<a
				href="/"
				onclick={(e) => {
					e.preventDefault();
					nav.page = 'benchmarks';
				}}>Benchmarks</a
			>
			<a href="https://github.com/Nurt-studio/Sequoia-UI" target="_blank" rel="noreferrer">GitHub</a>
			<a href="https://crates.io/crates/sequoia-ui" target="_blank" rel="noreferrer">crates.io</a>
		</div>
	</div>
	<div class="footer-bottom-text">© 2026 SequoiaUI. MIT licensed.</div>
</footer>

<style>
	footer {
		background: #1a1a1a;
		padding: var(--space-7) var(--space-6) var(--space-5);
		color: #a0a0a0;
	}
	.footer-inner {
		max-width: 1400px;
		margin: 0 auto var(--space-5);
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding-bottom: var(--space-5);
		border-bottom: 1px solid #2a2a2a;
	}
	.footer-logo {
		font-size: var(--text-base);
		font-weight: 700;
		color: #fff;
	}
	.footer-links {
		display: flex;
		gap: var(--space-5);
	}
	.footer-links a {
		color: #a0a0a0;
		text-decoration: none;
		font-size: var(--text-sm);
		font-weight: 500;
		transition: color 0.2s;
	}
	.footer-links a:hover {
		color: var(--accent);
	}
	.footer-bottom-text {
		max-width: 1400px;
		margin: 0 auto;
		font-size: var(--text-xs);
		color: #666;
	}
	@media (max-width: 768px) {
		footer {
			padding: var(--space-6) var(--space-4) var(--space-4);
		}
		.footer-inner {
			flex-direction: column;
			gap: 16px;
			align-items: flex-start;
		}
		.footer-links {
			flex-wrap: wrap;
			gap: var(--space-4);
		}
	}
</style>