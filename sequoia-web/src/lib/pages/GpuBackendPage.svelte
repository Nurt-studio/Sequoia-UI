<script>
	import DocsTopBar from '../components/DocsTopBar.svelte';
	import DocsFooter from '../components/DocsFooter.svelte';
	import { nav } from '../navigation.svelte.js';
	import {
		meta,
		highlights,
		lightScene,
		heavyScene,
		general,
		facts,
		caveats,
		delta,
		winner,
		num
	} from '../backend-compare-data.js';
	import lightImage from '../../assets/backend-export-light.png';
	import heavyImage from '../../assets/backend-export-heavy.png';

	const scenes = [
		{
			image: lightImage,
			label: 'Light scene',
			detail: 'One sphere, one plane, 56 raymarch steps. Vsync-capped on both backends.'
		},
		{
			image: heavyImage,
			label: 'Heavy scene',
			detail: 'Four SDF objects, soft shadows, AO and a reflection bounce. Clearly GPU-bound.'
		}
	];

	const groups = [
		{ id: 'light', title: 'Light scene', note: 'Vsync-capped at 60 fps on both backends.', rows: lightScene },
		{ id: 'heavy', title: 'Heavy scene', note: 'GPU-bound below vsync — the throughput difference is visible.', rows: heavyScene },
		{ id: 'general', title: 'General / UI showcase', note: 'Same application, no embedded 3D surface.', rows: general }
	];
</script>

<DocsTopBar version={meta.version} />

<div class="bench-page">
	<header class="bench-hero">
		<a
			class="back"
			href="/"
			onclick={(e) => {
				e.preventDefault();
				nav.page = 'benchmarks';
			}}>&larr; Benchmarks</a
		>

		<div class="eyebrow">
			<div>
				<span class="section-number">01</span>
				<p class="section-label">Experimental backend</p>
			</div>
			<span class="eyebrow-date">{meta.date}</span>
		</div>

		<h1>Ash: <span class="accent">pure Vulkan</span>, beside wgpu.</h1>

		<p class="lede">
			An experimental renderer that talks to Vulkan directly through <code>ash</code>, living
			alongside the stable wgpu backend. Same render list, same glyph atlas, same WGSL source —
			compiled to SPIR-V for the Vulkan path. On the heavy 3D scene it reaches
			<span class="accent">21.8 fps</span> against 16.3, with roughly
			<span class="accent">a third of the memory</span>.
		</p>

		<div class="hero-meta">
			<span>Intel HD 620</span>
			<span>1200 × 800</span>
			<span>Debug build</span>
			<span>540 frames after warm-up</span>
		</div>

		<div class="hero-actions">
			<a href="#scenes" class="btn btn-outline">See the scenes</a>
			<a href="#results" class="hero-link">Benchmark results &rarr;</a>
			<a href="#method" class="hero-link">How it was measured &rarr;</a>
		</div>

		<div class="stat-grid">
			{#each highlights as item}
				<div class="stat">
					<strong>{item.value}</strong>
					<span class="stat-label">{item.label}</span>
					<span class="stat-detail">{item.detail}</span>
				</div>
			{/each}
		</div>
	</header>

	<section class="bench-section" aria-labelledby="design-title">
		<div class="section-head">
			<div>
				<span class="section-number">02</span>
				<p class="section-label">Design</p>
				<h2 id="design-title" class="section-title">A second backend without a second UI</h2>
			</div>
			<p class="section-note">
				Both backends consume the same <code>RenderList</code> and share the built-in WGSL module
				so coverage maths and colour handling cannot drift apart.
			</p>
		</div>

		<div class="fact-grid">
			{#each facts as fact}
				<article class="fact">
					<h3>{fact.title}</h3>
					<p>{fact.text}</p>
				</article>
			{/each}
		</div>
	</section>

	<section class="bench-section" id="scenes" aria-labelledby="scenes-title">
		<div class="section-head">
			<div>
				<span class="section-number">03</span>
				<p class="section-label">Scenes</p>
				<h2 id="scenes-title" class="section-title">Rendered headlessly with sequoia-export</h2>
			</div>
			<p class="section-note">
				Sequoia's CPU exporter produces these images from the same view tree — no window, no GPU
				adapter and no screen capture.
			</p>
		</div>

		<div class="scene-grid">
			{#each scenes as scene}
				<figure class="shot">
					<img src={scene.image} alt="{scene.label}: raymarched 3D surface" />
					<figcaption>
						<strong>{scene.label}</strong>
						<span>{scene.detail}</span>
					</figcaption>
				</figure>
			{/each}
		</div>

		<p class="table-hint">
			<code>surface_3d</code> draws the same scene on the GPU through either
			<code>WgpuSurface</code> or <code>AshSurface</code>; <code>backend_scene</code> reproduces it
			as a CPU shader for the exporter. The numbers below come from the GPU example.
		</p>
	</section>

	<section class="bench-section" id="results" aria-labelledby="results-title">
		<div class="section-head">
			<div>
				<span class="section-number">04</span>
				<p class="section-label">Measurements</p>
				<h2 id="results-title" class="section-title">wgpu vs ash, same scene</h2>
			</div>
			<p class="section-note">
				Lower is better for CPU, memory and frame cost; higher is better for FPS.
			</p>
		</div>

		{#each groups as group}
			<div class="snapshot-table" id={group.id === 'general' ? 'method' : undefined}>
				<table>
					<thead>
						<tr>
							<th scope="col">Metric</th>
							<th scope="col">wgpu (Vulkan)</th>
							<th scope="col">ash (Vulkan)</th>
							<th scope="col">Difference</th>
						</tr>
					</thead>
					<tbody>
						{#each group.rows as row}
							<tr>
								<td><span class="metric">{row.metric}</span></td>
								<td class:best={winner(row) === 'wgpu'}>{num(row.wgpu)} <span class="unit">{row.unit}</span></td>
								<td class:best={winner(row) === 'ash'}>{num(row.ash)} <span class="unit">{row.unit}</span></td>
								<td class="value">
									<span class="change good">{delta(row)}</span>
									<span class="hint">{winner(row)}</span>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
				<p class="table-hint">{group.title} — {group.note}</p>
			</div>
		{/each}
	</section>

	<section class="bench-section" aria-labelledby="status-title">
		<div class="section-head">
			<div>
				<span class="section-number">05</span>
				<p class="section-label">Status</p>
				<h2 id="status-title" class="section-title">Experimental, with known limits</h2>
			</div>
			<p class="section-note">Backdrops, filters, isolated opacity groups and custom WGSL user shaders are not implemented by the Vulkan renderer yet.</p>
		</div>

		<ul class="caveat-list">
			{#each caveats as item}
				<li>{item}</li>
			{/each}
		</ul>

		<div class="hero-actions">
			<a href="https://github.com/Nurt-studio/Sequoia-UI" target="_blank" rel="noreferrer" class="btn btn-outline">
				Try the examples on GitHub
			</a>
		</div>
	</section>
</div>

<DocsFooter />

<style>
	.bench-page {
		max-width: 1200px;
		margin: 0 auto;
		padding: 0 var(--section-px) var(--space-8);
	}

	.bench-hero {
		padding: var(--section-py) 0 var(--space-7);
		border-bottom: 1px solid var(--border, #d5d5d8);
	}
	.back {
		display: inline-block;
		margin-bottom: var(--space-5);
		font-size: var(--text-sm);
		font-weight: 600;
		color: #777;
		text-decoration: none;
	}
	.back:hover {
		color: var(--accent, #e6391e);
	}
	.eyebrow {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-5);
	}
	.section-number {
		font-size: var(--text-xl);
		line-height: 1;
	}
	.eyebrow .section-label {
		margin-bottom: 0;
	}
	.eyebrow-date {
		padding-top: var(--space-1);
		font-family: var(--font-mono, monospace);
		font-size: var(--text-2xs);
		letter-spacing: var(--tracking-wide);
		text-transform: uppercase;
		color: #8a8a8e;
	}
	.bench-hero h1 {
		margin-top: var(--space-5);
		max-width: 900px;
		font-size: clamp(2.125rem, 5vw, 3.25rem);
		font-weight: 800;
		line-height: var(--leading-tight);
		letter-spacing: var(--tracking-tighter);
		text-wrap: balance;
	}
	.accent {
		color: var(--accent, #e6391e);
	}
	.lede {
		margin-top: var(--space-5);
		max-width: 820px;
		font-size: var(--text-lg);
		font-weight: 500;
		line-height: var(--leading-normal);
		color: #3d3d3f;
	}
	code {
		font-family: var(--font-mono, monospace);
		font-size: 0.88em;
		color: var(--accent, #e6391e);
	}
	.hero-meta {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2) var(--space-5);
		margin-top: var(--space-5);
		font-family: var(--font-mono, monospace);
		font-size: var(--text-2xs);
		letter-spacing: var(--tracking-wide);
		text-transform: uppercase;
		color: #8a8a8e;
	}
	.hero-meta span {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
	}
	.hero-meta span + span::before {
		content: '';
		width: 3px;
		height: 3px;
		border-radius: 50%;
		background: #c5c5c9;
	}
	.hero-actions {
		display: flex;
		align-items: center;
		gap: var(--space-5);
		flex-wrap: wrap;
		margin-top: var(--space-6);
	}
	.hero-link {
		font-size: var(--text-sm);
		font-weight: 600;
		color: #555;
		text-decoration: none;
	}
	.hero-link:hover {
		color: var(--accent, #e6391e);
	}

	.stat-grid {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: var(--space-3);
		margin-top: var(--space-7);
	}
	.stat {
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 16px;
		padding: var(--space-5);
	}
	.stat strong {
		display: block;
		font-size: clamp(1.5rem, 2.6vw, 2rem);
		line-height: 1;
		letter-spacing: var(--tracking-tighter);
		color: var(--accent, #e6391e);
	}
	.stat-label {
		display: block;
		margin-top: var(--space-3);
		font-size: var(--text-sm);
		font-weight: 700;
	}
	.stat-detail {
		display: block;
		margin-top: var(--space-1);
		font-size: var(--text-xs);
		color: #8a8a8e;
	}

	.bench-section {
		padding: var(--space-8) 0;
		border-bottom: 1px solid var(--border, #d5d5d8);
		scroll-margin-top: 72px;
	}
	.bench-section:last-of-type {
		border-bottom: 0;
	}
	.section-head {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: var(--space-6);
		margin-bottom: var(--space-6);
	}
	.section-head .section-title {
		margin: 0;
	}
	.section-note {
		max-width: 360px;
		font-size: var(--text-xs);
		line-height: var(--leading-normal);
		color: #777;
	}

	.fact-grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: var(--space-3);
	}
	.fact {
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 16px;
		padding: var(--space-5);
	}
	.fact h3 {
		margin: 0 0 var(--space-2);
		font-size: var(--text-base);
	}
	.fact p {
		margin: 0;
		color: #666;
		font-size: var(--text-sm);
		line-height: var(--leading-normal);
	}

	.scene-grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: var(--space-4);
	}
	.shot {
		margin: 0;
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 16px;
		overflow: hidden;
	}
	.shot img {
		display: block;
		width: 100%;
		height: auto;
		background: #0b0d12;
	}
	.shot figcaption {
		border-top: 1px solid #e5e5e7;
		padding: var(--space-3) var(--space-4);
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.shot figcaption strong {
		font-size: var(--text-sm);
	}
	.shot figcaption span {
		font-size: var(--text-xs);
		line-height: var(--leading-normal);
		color: #777;
	}

	.snapshot-table {
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 16px;
		padding: var(--space-2) var(--space-3) var(--space-3);
		margin-bottom: var(--space-4);
		scroll-margin-top: 72px;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-variant-numeric: tabular-nums;
	}
	thead th {
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid #1a1a1a;
		text-align: right;
		vertical-align: bottom;
		font-size: var(--text-2xs);
		font-weight: 700;
		letter-spacing: var(--tracking-wide);
		text-transform: uppercase;
	}
	thead th:first-child {
		text-align: left;
	}
	tbody td {
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid #ededee;
		text-align: right;
	}
	tbody tr:last-child td {
		border-bottom: 0;
	}
	tbody td:first-child {
		text-align: left;
	}
	.metric {
		font-weight: 600;
		font-size: var(--text-sm);
	}
	.unit {
		font-size: var(--text-xs);
		color: #8a8a8e;
	}
	td.best {
		font-weight: 700;
		color: var(--accent, #e6391e);
	}
	.change {
		font-weight: 700;
	}
	.change.good {
		color: #18864b;
	}
	.hint {
		margin-left: 8px;
		font-size: var(--text-xs);
		color: #8a8a8e;
	}
	.table-hint {
		margin: var(--space-3) var(--space-4) 0;
		font-size: var(--text-xs);
		line-height: var(--leading-normal);
		color: #777;
	}

	.caveat-list {
		margin: 0;
		padding-left: 20px;
		color: #666;
		font-size: var(--text-sm);
		line-height: var(--leading-normal);
	}
	.caveat-list li {
		margin-bottom: var(--space-2);
	}

	@media (max-width: 1000px) {
		.stat-grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
	@media (max-width: 768px) {
		.fact-grid,
		.scene-grid {
			grid-template-columns: 1fr;
		}
		.section-head {
			align-items: flex-start;
			flex-direction: column;
			gap: var(--space-3);
		}
		.section-note {
			max-width: none;
		}
	}
</style>
