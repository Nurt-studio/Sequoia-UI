<script>
	import DocsTopBar from '../components/DocsTopBar.svelte';
	import DocsFooter from '../components/DocsFooter.svelte';
	import { nav } from '../navigation.svelte.js';
	import benchmarkSummary from '../../assets/benchmark-summary.png';

	const BOOK = 'https://nurt-studio.github.io/Sequoia-UI/';

	const posts = [
		{
			num: '02',
			icon: '⚡',
			tag: 'Guide',
			title: 'Custom GPU shaders',
			desc: 'Typed shaders with the shader! macro, validated at compile time. ShaderPass carries blend mode, UV window and textures; ShaderChain sequences passes through offscreen targets.',
			href: `${BOOK}guides/shaders.html`,
			bg: 'linear-gradient(135deg, #7ec8e3, #4a9eb8)'
		},
		{
			num: '03',
			icon: '⌨️',
			tag: 'Guide',
			title: 'Text editing that ships',
			desc: 'IME, accessibility metadata, secure password fields, auto-size and soft wrap. Shaped text is cached, the glyph atlas is compacted, and GPU memory is bounded by budgets.',
			href: `${BOOK}concepts/text.html`,
			bg: 'linear-gradient(135deg, #1a1a1a, #4a4a4a)'
		},
		{
			num: '04',
			icon: '🧩',
			tag: 'Guide',
			title: 'Docking and IDE-style layouts',
			desc: 'DockArea, DockPanel, SplitPanel and floating windows, with dock state that can be encoded and restored. The vision-design example uses the same public API.',
			href: `${BOOK}guides/docking.html`,
			bg: 'linear-gradient(135deg, #98c379, #6b9e52)'
		},
		{
			num: '05',
			icon: '🖼️',
			tag: 'Guide',
			title: 'Headless export and testing',
			desc: 'Render a view tree to a HiDPI PNG without opening a window, and drive it in tests with the sequoia-test harness.',
			href: `${BOOK}guides/export.html`,
			bg: 'linear-gradient(135deg, #c678dd, #8e44ad)'
		},
		{
			num: '06',
			icon: '📄',
			tag: 'Experimental',
			title: 'The .seq format',
			desc: 'An optional declarative format loaded through SeqLoader, with typed host bindings and a preview binary that can watch a file. Still evolving; the Rust API is the supported path.',
			href: `${BOOK}guides/seq.html`,
			bg: 'linear-gradient(135deg, #e5c07b, #c49a3c)'
		},
		{
			num: '07',
			icon: '🧵',
			tag: 'Guide',
			title: 'Async work and animation',
			desc: 'Async tasks bridged back to the UI thread, plus springs, timelines and state-driven transitions. No separate runtime to wire up.',
			href: `${BOOK}concepts/async.html`,
			bg: 'linear-gradient(135deg, #e6391e, #ff6b4a)'
		}
	];

	const rows = [posts.slice(0, 2), posts.slice(2, 4), posts.slice(4, 6)];
</script>

<DocsTopBar />

<div class="blog-hero">
	<h1>Release notes</h1>
	<p>Release highlights and feature guides, taken from the repository and its documentation</p>
</div>

<div class="blog-featured">
	<div class="blog-featured-card">
		<div class="blog-featured-img">
			<span class="big-num">01</span>
			<span class="img-label">Latest release</span>
		</div>
		<div class="blog-featured-content">
			<div class="meta">
				<span class="blog-tag">Release</span>
				<span class="blog-date">November 11, 2026</span>
			</div>
			<h2>SequoiaUI 0.1.5</h2>
			<p>
				Layout and performance release: cached layout, faster flex distribution, view culling,
				stabilized text intrinsic sizing and frame hooks. Requires Rust 1.88 or newer.
			</p>
			<a
				href="https://github.com/Nurt-studio/Sequoia-UI/releases/tag/0.1.5"
				class="blog-read-more"
				target="_blank"
				rel="noreferrer">Read the release notes →</a
			>
		</div>
	</div>
	<div class="blog-featured-card">
		<div class="blog-featured-img has-image">
			<img
				src={benchmarkSummary}
				alt="Cross-framework benchmark: sequoia-ui holds 60 fps in both 3,000-node scenes"
			/>
		</div>
		<div class="blog-featured-content">
			<div class="meta">
				<span class="blog-tag">Benchmark</span>
				<span class="blog-date">November 11, 2026</span>
			</div>
			<h2>sequoia-ui vs gpui, gpui-fast and BUI</h2>
			<p>
				One machine, one session, identical scenes. sequoia-ui holds 60 fps in both 3,000-node
				scenes at 2.26 ms of CPU per frame — 6.1× gpui-fast and 7.7× gpui on the row scene.
			</p>
			<a
				href="/"
				class="blog-read-more"
				onclick={(e) => {
					e.preventDefault();
					nav.page = 'benchmarks';
				}}>See the full benchmark →</a
			>
		</div>
	</div>
</div>

<div class="blog-posts">
	<div class="blog-posts-title">Guides &amp; notes</div>

	{#each rows as row}
		<div class="blog-post-row">
			{#each row as post}
				<a class="blog-post-card" href={post.href} target="_blank" rel="noreferrer">
					<div class="blog-post-thumb" style:background={post.bg}>
						<span class="thumb-num">{post.num}</span>
						<span class="thumb-icon">{post.icon}</span>
					</div>
					<div class="blog-post-body">
						<div class="meta">
							<span class="blog-tag">{post.tag}</span>
						</div>
						<h3>{post.title}</h3>
						<p>{post.desc}</p>
						<span class="blog-card-link">Read in the book →</span>
					</div>
				</a>
			{/each}
		</div>
	{/each}
</div>

<DocsFooter />

<style>
	.blog-hero {
		max-width: 900px;
		margin: 0 auto;
		padding: var(--section-py) var(--space-6) var(--space-7);
		text-align: center;
	}
	.blog-hero h1 {
		font-size: var(--text-5xl);
		font-weight: 800;
		letter-spacing: var(--tracking-tighter);
		line-height: var(--leading-tight);
		margin-bottom: var(--space-4);
		text-wrap: balance;
	}
	.blog-hero p {
		font-size: var(--text-lg);
		color: #666;
		max-width: 500px;
		margin: 0 auto;
		line-height: var(--leading-normal);
	}

	.blog-featured {
		max-width: 900px;
		margin: 0 auto var(--space-7);
		padding: 0 var(--space-6);
	}
	.blog-featured-card {
		display: grid;
		grid-template-columns: 1fr 1fr;
		background: #fff;
		border-radius: 16px;
		overflow: hidden;
		border: 1px solid #e5e5e7;
		transition: all 0.3s;
		cursor: pointer;
	}
	.blog-featured-card:hover {
		box-shadow: 0 12px 40px rgba(0, 0, 0, 0.08);
		transform: translateY(-2px);
	}
	.blog-featured-card + .blog-featured-card {
		margin-top: var(--space-5);
	}
	.blog-featured-img {
		background: linear-gradient(135deg, #e6391e 0%, #ff6b4a 50%, #ff9a76 100%);
		min-height: 280px;
		display: flex;
		align-items: flex-end;
		padding: var(--space-6);
		position: relative;
	}
	.blog-featured-img .big-num {
		font-size: 120px;
		font-weight: 800;
		color: rgba(255, 255, 255, 0.15);
		position: absolute;
		top: 20px;
		left: 32px;
		line-height: 1;
	}
	.blog-featured-img .img-label {
		color: #fff;
		font-size: var(--text-sm);
		font-weight: 600;
		background: rgba(0, 0, 0, 0.2);
		backdrop-filter: blur(8px);
		padding: var(--space-2) var(--space-3);
		border-radius: 20px;
	}
	.blog-featured-img.has-image {
		padding: 0;
		background: #fff;
	}
	.blog-featured-img.has-image img {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: cover;
		object-position: left center;
	}
	.blog-featured-content {
		padding: var(--space-7);
		display: flex;
		flex-direction: column;
		justify-content: center;
	}
	.blog-featured-content .meta {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		margin-bottom: var(--space-4);
	}
	.blog-tag {
		font-size: var(--text-xs);
		font-weight: 700;
		color: var(--accent);
		background: rgba(230, 57, 30, 0.08);
		padding: var(--space-1) var(--space-3);
		border-radius: 20px;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.blog-date {
		font-size: var(--text-xs);
		color: #999;
		font-weight: 500;
	}
	.blog-featured-content h2 {
		font-size: var(--text-2xl);
		font-weight: 800;
		letter-spacing: var(--tracking-tight);
		margin-bottom: var(--space-3);
		line-height: var(--leading-snug);
	}
	.blog-featured-content p {
		font-size: var(--text-base);
		color: #666;
		line-height: var(--leading-normal);
		margin-bottom: var(--space-5);
	}
	.blog-read-more {
		font-size: var(--text-sm);
		font-weight: 700;
		color: var(--accent);
		text-decoration: none;
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		transition: gap 0.2s;
	}
	.blog-read-more:hover {
		gap: 10px;
	}

	.blog-posts {
		max-width: 900px;
		margin: 0 auto;
		padding: 0 var(--space-6) var(--space-8);
	}
	.blog-posts-title {
		font-size: var(--text-sm);
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: var(--tracking-wide);
		color: #999;
		margin-bottom: var(--space-5);
	}
	.blog-post-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-5);
		margin-bottom: var(--space-5);
	}
	.blog-post-card {
		display: block;
		color: inherit;
		text-decoration: none;
		background: #fff;
		border-radius: 12px;
		overflow: hidden;
		border: 1px solid #e5e5e7;
		transition: all 0.3s;
		cursor: pointer;
	}
	.blog-post-card:hover {
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.06);
		transform: translateY(-2px);
	}
	.blog-post-thumb {
		height: 160px;
		display: flex;
		align-items: center;
		justify-content: center;
		position: relative;
	}
	.blog-post-thumb .thumb-num {
		font-size: 64px;
		font-weight: 800;
		color: rgba(255, 255, 255, 0.2);
	}
	.blog-post-thumb .thumb-icon {
		position: absolute;
		font-size: 32px;
	}
	.blog-post-body {
		padding: var(--space-5);
	}
	.blog-post-body .meta {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-bottom: var(--space-2);
	}
	.blog-post-body h3 {
		font-size: var(--text-lg);
		font-weight: 700;
		margin-bottom: var(--space-2);
		letter-spacing: -0.01em;
		line-height: var(--leading-snug);
	}
	.blog-post-body p {
		font-size: var(--text-sm);
		color: #777;
		line-height: var(--leading-normal);
	}
	.blog-card-link {
		display: inline-block;
		margin-top: var(--space-3);
		font-size: var(--text-sm);
		font-weight: 700;
		color: var(--accent);
	}

	@media (max-width: 900px) {
		.blog-featured-card {
			grid-template-columns: 1fr;
		}
		.blog-post-row {
			grid-template-columns: 1fr;
		}
	}
	@media (max-width: 768px) {
		.blog-hero h1 {
			font-size: var(--text-3xl);
		}
	}
</style>