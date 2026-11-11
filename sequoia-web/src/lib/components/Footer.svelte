<script>
	import { nav } from '../navigation.svelte.js';

	const DOCS = 'https://nurt-studio.github.io/Sequoia-UI/';
	const REPO = 'https://github.com/Nurt-studio/Sequoia-UI';

	const columns = [
		{
			title: 'Product',
			links: [
				{ label: 'Documentation', href: '/', onClick: () => (nav.page = 'docs') },
				{ label: 'Benchmarks', href: '/', onClick: () => (nav.page = 'benchmarks') },
				{ label: 'Full book', href: DOCS },
				{ label: 'API reference', href: 'https://docs.rs/sequoia-ui' },
				{ label: 'Repository', href: REPO },
				{ label: 'Release notes', href: `${REPO}/releases` }
			]
		},
		{
			title: 'Resources',
			links: [
				{ label: 'Installation', href: `${DOCS}getting-started/installation.html` },
				{ label: 'Examples', href: `${DOCS}getting-started/examples.html` },
				{ label: 'Widgets', href: `${DOCS}guides/widgets.html` },
				{ label: '.seq format', href: `${DOCS}guides/seq.html` }
			]
		},
		{
			title: 'Community',
			links: [
				{ label: 'GitHub Issues', href: `${REPO}/issues` },
				{ label: 'crates.io', href: 'https://crates.io/crates/sequoia-ui' },
				{ label: 'MIT license', href: `${REPO}/blob/main/LICENSE-MIT` }
			]
		}
	];

	const socials = [
		{ label: 'GitHub', href: REPO },
		{ label: 'docs.rs', href: 'https://docs.rs/sequoia-ui' },
		{ label: 'crates.io', href: 'https://crates.io/crates/sequoia-ui' }
	];
</script>

<footer>
	<div class="footer-content">
		<div class="footer-brand">
			<h3>SequoiaUI</h3>
			<p>
				A native, reactive GUI framework for Rust built on wgpu. Retained view tree,
				fine-grained signals and a typed widget library. MIT licensed, no web view.
			</p>
		</div>
		{#each columns as col}
			<div class="footer-column">
				<h4>{col.title}</h4>
				<ul>
					{#each col.links as link}
						<li>
							<a
								href={link.href}
								target={link.href.startsWith('http') ? '_blank' : undefined}
								rel={link.href.startsWith('http') ? 'noreferrer' : undefined}
								onclick={(e) => {
									if (!link.onClick) return;
									e.preventDefault();
									link.onClick();
								}}>{link.label}</a
							>
						</li>
					{/each}
				</ul>
			</div>
		{/each}
	</div>
	<div class="footer-bottom">
		<div>© 2026 SequoiaUI. MIT licensed.</div>
		<div class="social-links">
			{#each socials as s}
				<a href={s.href} target="_blank" rel="noreferrer">{s.label}</a>
			{/each}
		</div>
	</div>
</footer>

<style>
	footer {
		background: #1a1a1a;
		border-top: 1px solid #2a2a2a;
		padding: var(--space-8) var(--section-px) var(--space-6);
		color: #a0a0a0;
	}

	.footer-content {
		max-width: 1200px;
		margin: 0 auto var(--space-7);
		display: grid;
		grid-template-columns: 2fr 1fr 1fr 1fr;
		gap: var(--space-7);
	}

	.footer-brand h3 {
		color: #ffffff;
		font-size: var(--text-lg);
		font-weight: 700;
		margin-bottom: var(--space-4);
		letter-spacing: var(--tracking-tight);
	}

	.footer-brand p {
		font-size: var(--text-sm);
		line-height: var(--leading-normal);
		max-width: 300px;
	}

	.footer-column h4 {
		color: #ffffff;
		font-size: var(--text-sm);
		font-weight: 700;
		margin-bottom: var(--space-5);
		text-transform: uppercase;
		letter-spacing: var(--tracking-wide);
	}

	.footer-column ul {
		list-style: none;
	}

	.footer-column li {
		margin-bottom: var(--space-3);
	}

	.footer-column a {
		color: #a0a0a0;
		text-decoration: none;
		font-size: var(--text-sm);
		font-weight: 500;
		transition: color 0.2s;
	}
	.footer-column a:hover {
		color: var(--accent);
	}

	.footer-bottom {
		max-width: 1200px;
		margin: 0 auto;
		padding-top: var(--space-6);
		border-top: 1px solid #2a2a2a;
		display: flex;
		justify-content: space-between;
		align-items: center;
		font-size: var(--text-xs);
	}

	.social-links {
		display: flex;
		gap: var(--space-4);
	}
	.social-links a {
		color: #a0a0a0;
		text-decoration: none;
		font-weight: 500;
		transition: color 0.2s;
	}
	.social-links a:hover {
		color: var(--accent);
	}

	@media (max-width: 900px) {
		.footer-content {
			grid-template-columns: 1fr 1fr;
		}
	}

	@media (max-width: 768px) {
		footer {
			padding: var(--space-7) var(--section-px) var(--space-5);
		}
		.footer-content {
			grid-template-columns: 1fr;
			gap: var(--space-6);
		}
		.footer-bottom {
			flex-direction: column;
			gap: var(--space-4);
			text-align: center;
		}
	}
</style>