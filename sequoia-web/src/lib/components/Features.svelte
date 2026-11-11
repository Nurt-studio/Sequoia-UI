<script>
    import { softReveal } from "../actions/softReveal";
	const features = [
		{
			num: '01',
			title: 'Native rendering',
			desc: 'Views paint through wgpu on Metal, Vulkan, DX12 or OpenGL. Your UI never runs inside a browser engine or a system web view.'
		},
		{
			num: '02',
			title: 'Memory safe',
			desc: 'Rust ownership removes whole classes of bugs at compile time, and the workspace sets unsafe_code = "forbid". There is no garbage collector.'
		},
		{
			num: '03',
			title: 'Themeable',
			desc: 'A theme token system plus typed Style presets for every widget. Override a single property with a builder, or define a complete look.'
		},
		{
			num: '04',
			title: 'Reactive State',
			desc: 'Signals update only the views that read them. There is no diff step, no virtual DOM and no separate store to synchronize.'
		},
		{
			num: '05',
			title: 'Desktop details',
			desc: 'IME, accessibility metadata, hotkeys, async work, multi-window support and headless PNG export are part of the framework, not add-ons.'
		},
		{
			num: '06',
			title: 'Extensible',
			desc: 'Implement the View trait for custom widgets, add custom layouts, shaders and assets. The runtime treats them like built-ins.'
		}
	];
</script>

<section class="section" use:softReveal>
	<div>
		<span class="section-number">03</span>
		<p class="section-label">Features</p>
		<h2 class="section-title">A native Rust framework, without a browser or a diff step</h2>
		<p class="section-description">
			Retained views painted through wgpu, with signals that update only what changed
		</p>
	</div>
	<div class="features-list">
		{#each features as f}
			<div class="feature-row">
				<div class="feature-num">{f.num}</div>
				<div class="feature-title">{f.title}</div>
				<div class="feature-desc">{f.desc}</div>
			</div>
		{/each}
	</div>
</section>

<style>
	.features-list {
		margin-top: var(--section-gap);
		display: flex;
		flex-direction: column;
	}

	.feature-row {
		display: grid;
		grid-template-columns: 80px 1fr 2fr;
		gap: var(--space-6);
		align-items: start;
		padding: var(--space-6) 0;
		border-bottom: 1px solid var(--border);
	}
	.feature-row:first-child {
		border-top: 1px solid var(--border);
	}
	.feature-row:hover .feature-num,
	.feature-row:hover .feature-title {
		color: var(--accent);
	}

	.feature-num {
		font-size: var(--text-sm);
		font-weight: 700;
		color: var(--muted);
		transition: color 0.3s ease;
		font-variant-numeric: tabular-nums;
	}

	.feature-title {
		font-size: var(--text-xl);
		font-weight: 700;
		letter-spacing: var(--tracking-tight);
		transition: color 0.3s ease;
	}

	.feature-desc {
		font-size: var(--text-base);
		font-weight: 400;
		line-height: var(--leading-normal);
		color: #555;
	}

	@media (max-width: 900px) {
		.feature-row {
			grid-template-columns: 1fr;
			gap: var(--space-2);
		}
		.feature-num {
			font-size: var(--text-xs);
		}
	}
</style>