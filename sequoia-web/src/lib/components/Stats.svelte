<script>
	const stats = [
		{ number: '0.1.4', accent: '', label: 'Current crates.io release' },
		{ number: '25', accent: '', label: 'Framework crates' },
		{ number: '1', accent: '', label: 'Crate you add to Cargo.toml' },
		{ number: 'MIT', accent: '', label: 'License' }
	];
</script>

<section class="section">
	<div>
		<span class="section-number">06</span>
		<p class="section-label">By the numbers</p>
		<h2 class="section-title">Where the project stands</h2>
	</div>
	<div class="stats-grid">
		{#each stats as stat}
			<div class="stat-item">
				<div class="stat-number">
					{stat.number}<span class="accent">{stat.accent}</span>
				</div>
				<div class="stat-label">{stat.label}</div>
			</div>
		{/each}
	</div>
	<p class="stats-note">
		SequoiaUI is pre-1.0. The version above is the current crates.io release and the book targets 0.1.x;
		APIs may still change between minor releases.
	</p>
</section>

<style>
	.stats-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: var(--space-6);
		margin-top: var(--section-gap);
	}

	.stats-note {
		margin-top: var(--space-5);
		font-size: var(--text-sm);
		line-height: var(--leading-normal);
		color: var(--muted);
		max-width: 640px;
	}

	.stat-item {
		text-align: left;
	}

	.stat-number {
		font-size: var(--text-5xl);
		font-weight: 800;
		color: var(--fg);
		letter-spacing: var(--tracking-tighter);
		line-height: 1;
	}
	.stat-number .accent {
		color: var(--accent);
	}

	.stat-label {
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--muted);
		margin-top: var(--space-2);
	}

	@media (max-width: 900px) {
		.stats-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}
</style>