<script>
	import { nav } from '../navigation.svelte.js';

	const DOCS = 'https://nurt-studio.github.io/Sequoia-UI/';
	const REPO = 'https://github.com/Nurt-studio/Sequoia-UI';

	const links = [
		{ label: 'Docs', href: '/', page: 'docs', onClick: () => (nav.page = 'docs') },
		{ label: 'Components', href: `${DOCS}guides/widgets.html` },
		{ label: 'Examples', href: `${DOCS}getting-started/examples.html` },
		{ label: 'Benchmarks', href: '/', page: 'benchmarks', onClick: () => (nav.page = 'benchmarks') },
		{ label: 'Blog', href: '/', page: 'blog', onClick: () => (nav.page = 'blog') }
	];

	let open = $state(false);

	function toggle() {
		open = !open;
	}
	function close() {
		open = false;
	}
	/** @param {{ label: string, href?: string, page?: string, onClick?: () => void }} link */
	function go(link) {
		close();
		link.onClick?.();
	}
	/** @param {{ page?: string }} link */
	function isCurrent(link) {
		return link.page === nav.page;
	}
	/** @param {KeyboardEvent} e */
	function onKeydown(e) {
		if (e.key === 'Escape') close();
	}

	$effect(() => {
		const mq = window.matchMedia('(max-width: 768px)');
		const onChange = () => {
			if (!mq.matches) close();
		};
		onChange();
		mq.addEventListener('change', onChange);
		return () => mq.removeEventListener('change', onChange);
	});

	$effect(() => {
		document.body.style.overflow = open ? 'hidden' : '';
		return () => {
			document.body.style.overflow = '';
		};
	});
</script>

<svelte:window onkeydown={onKeydown} />

<nav class:open>
	<a
		href="/"
		class="logo"
		aria-label="SequoiaUI home"
		onclick={(e) => {
			e.preventDefault();
			close();
			nav.page = 'home';
		}}
	>
		Sequoia<span class="logo-ui">UI</span>
	</a>

	<div class="nav-actions">
		<ul class="nav-links">
			{#each links as link}
				<li>
					<a
						href={link.href ?? '#'}
						class:active={isCurrent(link)}
						aria-current={isCurrent(link) ? 'page' : undefined}
						target={link.href?.startsWith('http') ? '_blank' : undefined}
						rel={link.href?.startsWith('http') ? 'noreferrer' : undefined}
						onclick={(e) => {
							if (link.onClick) e.preventDefault();
							go(link);
						}}>{link.label}</a
					>
				</li>
			{/each}
		</ul>

		<a
			class="github-link"
			href={REPO}
			target="_blank"
			rel="noreferrer"
			aria-label="SequoiaUI on GitHub"
		>
			<svg viewBox="0 0 16 16" aria-hidden="true" fill="currentColor">
				<path
					d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.57-.16 1.18-.24 1.8-.24s1.23.08 1.8.24c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z"
				/>
			</svg>
			<span>GitHub</span>
		</a>
	</div>

	<button
		class="toggle"
		class:open
		type="button"
		aria-label={open ? 'Close menu' : 'Open menu'}
		aria-expanded={open}
		aria-controls="mobile-panel"
		onclick={toggle}
	>
		<span></span>
		<span></span>
	</button>
</nav>

<div id="mobile-panel" class="panel" class:open inert={!open}>
	<ul class="panel-links">
		{#each links as link, i}
			<li style:--i={i}>
				<a
					href={link.href ?? '#'}
					class:active={isCurrent(link)}
					aria-current={isCurrent(link) ? 'page' : undefined}
					target={link.href?.startsWith('http') ? '_blank' : undefined}
					rel={link.href?.startsWith('http') ? 'noreferrer' : undefined}
					onclick={(e) => {
						if (link.onClick) e.preventDefault();
						go(link);
					}}
				>
					<span class="num">0{i + 1}</span>
					<span class="label">{link.label}</span>
				</a>
			</li>
		{/each}
	</ul>

	<div class="panel-footer">
		<span>© 2026 SequoiaUI</span>
		<a href={REPO} target="_blank" rel="noreferrer">GitHub</a>
	</div>
</div>

<style>
	nav {
		position: relative;
		z-index: 100;
		display: flex;
		justify-content: space-between;
		align-items: center;
		max-width: 1200px;
		margin: 0 auto;
		padding: var(--space-5) var(--section-px);
	}
	.logo {
		font-size: var(--text-lg);
		font-weight: 700;
		color: var(--fg);
		text-decoration: none;
		letter-spacing: var(--tracking-tight);
		transition: color 0.3s ease;
	}
	.logo-ui {
		color: var(--accent);
		transition: color 0.3s ease;
	}
	.logo:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 4px;
		border-radius: 4px;
	}
	.nav-actions {
		display: flex;
		align-items: center;
		gap: var(--space-5);
	}
	.nav-links {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.nav-links a {
		display: block;
		padding: var(--space-2) var(--space-3);
		border-radius: 999px;
		color: #555;
		text-decoration: none;
		font-size: var(--text-sm);
		font-weight: 500;
		line-height: 1.2;
		transition:
			color 0.2s ease,
			background-color 0.2s ease;
	}
	.nav-links a:hover {
		color: var(--fg);
		background: rgba(26, 26, 26, 0.05);
	}
	.nav-links a.active {
		color: var(--fg);
		background: rgba(26, 26, 26, 0.07);
		font-weight: 600;
	}
	.nav-links a:focus-visible,
	.github-link:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.github-link {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--border);
		border-radius: 999px;
		color: var(--fg);
		text-decoration: none;
		font-size: var(--text-sm);
		font-weight: 600;
		line-height: 1.2;
		transition:
			color 0.2s ease,
			background-color 0.2s ease,
			border-color 0.2s ease;
	}
	.github-link svg {
		width: 16px;
		height: 16px;
	}
	.github-link:hover {
		background: var(--fg);
		border-color: var(--fg);
		color: var(--bg);
	}
	.toggle {
		display: none;
	}
	.panel {
		position: fixed;
		inset: 0;
		z-index: 90;
		display: flex;
		flex-direction: column;
		justify-content: center;
		overflow-y: auto;
		padding: var(--space-9) var(--section-px) var(--space-6);
		background: var(--accent, #e6391e);
		clip-path: circle(0% at calc(100% - 46px) 38px);
		transition: clip-path 0.7s cubic-bezier(0.76, 0, 0.24, 1);
		pointer-events: none;
	}
	.panel.open {
		clip-path: circle(150% at calc(100% - 46px) 38px);
		pointer-events: auto;
	}
	.panel-links {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.panel-links li {
		opacity: 0;
		transform: translateY(32px);
		transition:
			opacity 0.35s ease,
			transform 0.55s cubic-bezier(0.16, 1, 0.3, 1);
		transition-delay: 0s;
	}
	.panel.open .panel-links li {
		opacity: 1;
		transform: translateY(0);
		transition-delay: calc(0.18s + var(--i) * 70ms);
	}
	.panel-links a {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
		padding: var(--space-3) 0;
		color: #fff;
		text-decoration: none;
		font-size: clamp(1.375rem, 6.5vw, 2rem);
		font-weight: 700;
		letter-spacing: var(--tracking-tight);
		line-height: var(--leading-tight);
		border-bottom: 1px solid rgba(255, 255, 255, 0.2);
		transition:
			color 0.25s ease,
			transform 0.35s cubic-bezier(0.16, 1, 0.3, 1);
	}
	.panel-links a:hover,
	.panel-links a:focus-visible,
	.panel-links a.active {
		color: #1a1a1a;
		transform: translateX(8px);
		outline: none;
	}
	.num {
		font-size: var(--text-2xs);
		font-weight: 700;
		letter-spacing: var(--tracking-wide);
		opacity: 0.7;
		font-variant-numeric: tabular-nums;
	}
	.panel-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: var(--space-6);
		font-size: var(--text-xs);
		font-weight: 600;
		color: rgba(255, 255, 255, 0.8);
		opacity: 0;
		transition: opacity 0.3s ease;
	}
	.panel.open .panel-footer {
		opacity: 1;
		transition-delay: 0.5s;
	}
	.panel-footer a {
		color: #fff;
		text-decoration: none;
	}
	.panel-footer a:hover {
		color: #1a1a1a;
	}
	@media (min-width: 769px) {
		.panel {
			display: none;
		}
	}
	@media (max-width: 768px) {
		nav {
			padding: var(--space-4) var(--section-px);
		}
		nav.open .logo,
		nav.open .logo-ui {
			color: #fff;
		}
		.nav-actions {
			display: none;
		}
		.logo {
			font-size: var(--text-base);
		}
		.toggle {
			position: relative;
			z-index: 101;
			display: block;
			width: 40px;
			height: 40px;
			padding: 0;
			border: 0;
			background: transparent;
			cursor: pointer;
			-webkit-tap-highlight-color: transparent;
		}
		.toggle:focus-visible {
			outline: 2px solid #fff;
			outline-offset: 4px;
			border-radius: 8px;
		}
		.toggle span {
			position: absolute;
			left: 8px;
			width: 24px;
			height: 2px;
			border-radius: 2px;
			background: #1a1a1a;
			transition:
				top 0.45s cubic-bezier(0.76, 0, 0.24, 1),
				transform 0.45s cubic-bezier(0.76, 0, 0.24, 1),
				background-color 0.3s ease;
		}
		.toggle span:nth-child(1) {
			top: 16px;
		}
		.toggle span:nth-child(2) {
			top: 22px;
		}
		.toggle.open span:nth-child(1) {
			top: 19px;
			transform: rotate(45deg);
			background: #fff;
		}
		.toggle.open span:nth-child(2) {
			top: 19px;
			transform: rotate(-45deg);
			background: #fff;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.panel,
		.panel-links li,
		.panel-links a,
		.toggle span,
		.logo {
			transition-duration: 0.01ms !important;
			transition-delay: 0s !important;
		}
	}
</style>