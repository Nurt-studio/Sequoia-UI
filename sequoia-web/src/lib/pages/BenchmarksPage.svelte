<script>
	import DocsTopBar from '../components/DocsTopBar.svelte';
	import DocsFooter from '../components/DocsFooter.svelte';
	import { nav } from '../navigation.svelte.js';
	import { meta, runs } from '../benchmark-data.js';

	const frameworks = [
		{ id: 'sequoia', name: 'sequoia-ui', version: '0.1.5' },
		{ id: 'gpui-fast', name: 'gpui-fast', version: '0.1.14' },
		{ id: 'bui', name: 'bui', version: 'blenderui 0.5.0' },
		{ id: 'gpui', name: 'gpui', version: '0.2.2' }
	];

	const rank = new Map(frameworks.map((f, i) => [f.id, i]));
	/** @type {Record<string, number>} */
	const modeRank = { anim: 0, one: 1, idle: 2 };

	/**
	 * @param {string} framework
	 * @param {string} scenario
	 * @param {string} mode
	 * @param {number|null} [cells]
	 */
	function find(framework, scenario, mode, cells = null) {
		const run = runs.find(
			(r) =>
				r.framework === framework &&
				r.scenario === scenario &&
				r.mode === mode &&
				r.cells === cells
		);
		if (!run) throw new Error(`Missing benchmark run: ${framework}/${scenario}/${mode}/${cells}`);
		return run;
	}

	const summary = frameworks.map((f) => {
		const cells = find(f.id, 'cells', 'anim', 3000);
		const rows = find(f.id, 'rows', 'anim', 3000);
		return {
			...f,
			cellsFps: cells.fps,
			cpu: cells.cpu_ms_per_frame,
			rowsFps: rows.fps,
			rebuild: rows.rebuild_p50_us
		};
	});

	const best = {
		cellsFps: Math.max(...summary.map((s) => s.cellsFps ?? 0)),
		cpu: Math.min(...summary.map((s) => s.cpu ?? Infinity)),
		rowsFps: Math.max(...summary.map((s) => s.rowsFps ?? 0)),
		rebuild: Math.min(...summary.map((s) => s.rebuild ?? Infinity))
	};

	const seqCells = find('sequoia', 'cells', 'anim', 3000);
	const seqRows = find('sequoia', 'rows', 'anim', 3000);
	const gpuiFastCells = find('gpui-fast', 'cells', 'anim', 3000);
	const gpuiCells = find('gpui', 'cells', 'anim', 3000);
	const buiCells = find('bui', 'cells', 'anim', 3000);
	const gpuiFastRows = find('gpui-fast', 'rows', 'anim', 3000);
	const gpuiRows = find('gpui', 'rows', 'anim', 3000);
	const buiRows = find('bui', 'rows', 'anim', 3000);

	/**
	 * @param {number|null|undefined} a
	 * @param {number|null|undefined} b
	 */
	const ratio = (a, b) => (a == null || b == null ? '—' : (a / b).toFixed(1));

	const highlights = [
		{
			value: `${num(seqCells.fps, 1)} fps`,
			label: '3,000 animated cells',
			desc: `gpui-fast ${num(gpuiFastCells.fps, 1)} · bui ${num(buiCells.fps, 1)} · gpui ${num(gpuiCells.fps, 1)}`
		},
		{
			value: `${num(seqRows.fps, 1)} fps`,
			label: '3,000 animated rows',
			desc: `${ratio(seqRows.fps, gpuiFastRows.fps)}× gpui-fast · ${ratio(seqRows.fps, buiRows.fps)}× bui · ${ratio(seqRows.fps, gpuiRows.fps)}× gpui`
		},
		{
			value: `${num(seqCells.cpu_ms_per_frame, 2)} ms`,
			label: 'CPU per frame, 3,000 cells',
			desc: `${ratio(gpuiFastCells.cpu_ms_per_frame, seqCells.cpu_ms_per_frame)}× less than gpui-fast · ${ratio(gpuiCells.cpu_ms_per_frame, seqCells.cpu_ms_per_frame)}× less than gpui`
		},
		{
			value: `${num(seqCells.ready_wall_ms, 0)} ms`,
			label: 'to first presented frame',
			desc: `gpui ${num(gpuiCells.ready_wall_ms, 0)} ms · gpui-fast ${num(gpuiFastCells.ready_wall_ms, 0)} ms · bui ${num(buiCells.ready_wall_ms, 0)} ms`
		}
	];

	const scenarios = [
		{ id: 'cells', label: 'Cells', hint: 'Grid of animated cells at 100, 1,000 and 3,000 nodes.' },
		{ id: 'rows', label: 'Rows', hint: '3,000-node row list. Rows are virtualized in sequoia-ui by design.' },
		{ id: 'settings', label: 'Settings', hint: 'Startup only: one presented frame, then idle.' }
	];

	const cellFilters = [
		{ id: 'all', label: 'All' },
		{ id: '100', label: '100' },
		{ id: '1000', label: '1,000' },
		{ id: '3000', label: '3,000' }
	];

	let scenario = $state('cells');
	let cellsFilter = $state('all');

	const scenarioHint = $derived(scenarios.find((s) => s.id === scenario)?.hint ?? '');

	const visibleRuns = $derived(
		runs
			.filter((r) => r.scenario === scenario)
			.filter(
				(r) =>
					scenario !== 'cells' || cellsFilter === 'all' || r.cells === Number(cellsFilter)
			)
			.sort(
				(a, b) =>
					(rank.get(a.framework) ?? 0) - (rank.get(b.framework) ?? 0) ||
					(a.cells ?? 0) - (b.cells ?? 0) ||
					(modeRank[a.mode] ?? 0) - (modeRank[b.mode] ?? 0)
			)
	);

	const facts = [
		{ label: 'Machine', value: meta.machine },
		{
			label: 'Runner',
			value: `${meta.seconds} s window after ${meta.warmup} s warm-up, median of ${meta.reps} reps`
		},
		{ label: 'Build', value: 'Release · opt-level 3 · LTO fat · one back-to-back session' },
		{ label: 'Ready', value: 'Spawn → first presented frame; includes process start-up' },
		{ label: 'CPU / frame', value: 'Process CPU time ÷ measured frames; excludes GPU time' },
		{
			label: 'Rebuild p50',
			value: 'Render body (gpui family) / reactive probe (sequoia-ui); not directly comparable'
		}
	];

	const caveats = [
		'FPS saturates at the 60 Hz display refresh. For headroom, compare CPU ms/frame rather than FPS.',
		'Ready includes roughly 5–15 ms of process spawn. The app-self value is kept in the raw JSON.',
		'CPU/frame excludes GPU time, so read it together with the GPU 3D % column.',
		'RAM columns are working set and private bytes. VRAM is not included.',
		'Rows are virtualized in sequoia-ui by design; the scene still contains 3,000 nodes.',
		'Rebuild p50 measures different work per framework. It is not an apples-to-apples row.',
		'GPU counters are skipped when the driver does not expose them, so some cells stay empty.',
		'Numbers come from one machine and one session. Treat them as a reproducible reference, not a universal ranking.'
	];

	/**
	 * @param {number|null|undefined} value
	 * @param {number} [digits]
	 */
	function num(value, digits = 2) {
		if (value == null) return '—';
		const factor = 10 ** digits;
		return String(Math.round(value * factor) / factor);
	}

	/** @param {string} id */
	function fwName(id) {
		return frameworks.find((f) => f.id === id)?.name ?? id;
	}

	/** @param {string} id */
	function fwVersion(id) {
		return frameworks.find((f) => f.id === id)?.version ?? '';
	}
</script>

<DocsTopBar />

<div class="bench-page">
	<header class="bench-hero">
		<div class="eyebrow">
			<div>
				<span class="section-number">01</span>
				<p class="section-label">Benchmark</p>
			</div>
			<span class="eyebrow-date">{meta.when.slice(0, 10)}</span>
		</div>

		<h1>3,000 animated nodes. Still <span class="accent">60 fps</span>.</h1>

		<p class="lede">
			One machine, one session, identical scenes. sequoia-ui holds 60 fps in both 3,000-node
			scenes — <span class="accent">6.1×</span> gpui-fast and
			<span class="accent">7.7×</span> gpui on the row scene, at
			<span class="accent">2.26 ms</span> of CPU per frame on the cell scene.
		</p>

		<div class="hero-meta">
			<span>Windows 11</span>
			<span>Release · LTO fat</span>
			<span>10 s window</span>
			<span>Median of 3</span>
		</div>

		<div class="hero-actions">
			<a href="#results" class="btn btn-outline">See the full results</a>
			<a
				href="/"
				class="hero-link"
				onclick={(e) => {
					e.preventDefault();
					nav.page = 'compare-0.1.6';
				}}>0.1.6 vs 0.1.5 benchmark comparison &rarr;</a
			>
			<a href="#method" class="hero-link">How it was measured &rarr;</a>
		</div>
	</header>

	<section class="bench-section" aria-labelledby="snapshot-title">
		<div class="section-head">
			<div>
				<span class="section-number">02</span>
				<p class="section-label">Snapshot</p>
				<h2 id="snapshot-title" class="section-title">3,000 animated nodes, four frameworks</h2>
			</div>
			<p class="section-note">
				Cells are a grid of independently animated nodes; rows are a 3,000-node list. FPS is
				capped at the display refresh.
			</p>
		</div>

		<div class="snapshot-table">
			<table>
				<thead>
					<tr>
						<th scope="col">Framework</th>
						<th scope="col">Cells<span>3,000 nodes · fps</span></th>
						<th scope="col">CPU<span>ms per frame</span></th>
						<th scope="col">Rows<span>3,000 nodes · fps</span></th>
						<th scope="col">Rebuild<span>p50 · lower is better</span></th>
					</tr>
				</thead>
				<tbody>
					{#each summary as row}
						<tr class:win={row.id === 'sequoia'}>
							<td>
								<span class="fw-name">{row.name}</span>
								<span class="fw-version">{row.version}</span>
							</td>
							<td class:best={row.cellsFps === best.cellsFps}>
								{num(row.cellsFps, 1)}
							</td>
							<td class:best={row.cpu === best.cpu}>{num(row.cpu, 2)}</td>
							<td class:best={row.rowsFps === best.rowsFps}>
								{num(row.rowsFps, 1)}
							</td>
							<td class:best={row.rebuild === best.rebuild}>
								{row.rebuild == null ? '—' : num(row.rebuild / 1000, 1)}<span class="unit"
									>ms</span
								>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
			<p class="table-hint">
				Higher FPS is better. Lower CPU and rebuild time are better. Rebuild is measured
				differently per framework.
			</p>
		</div>

		<div class="highlights">
			{#each highlights as item}
				<div class="highlight">
					<div class="highlight-value">{item.value}</div>
					<div class="highlight-label">{item.label}</div>
					<div class="highlight-desc">{item.desc}</div>
				</div>
			{/each}
		</div>
	</section>

	<section class="bench-section" id="results" aria-labelledby="results-title">
		<div class="section-head results-head">
			<div>
				<span class="section-number">03</span>
				<p class="section-label">Full results</p>
				<h2 id="results-title" class="section-title">Every run, not just the headline</h2>
			</div>
			<p class="section-note">{scenarioHint}</p>
		</div>

		<div class="filter-bar">
			<div class="tabs" role="group" aria-label="Scenario">
				{#each scenarios as item}
					<button
						type="button"
						class:active={scenario === item.id}
						aria-pressed={scenario === item.id}
						onclick={() => (scenario = item.id)}>{item.label}</button
					>
				{/each}
			</div>

			{#if scenario === 'cells'}
				<div class="tabs small" role="group" aria-label="Cell count">
					{#each cellFilters as item}
						<button
							type="button"
							class:active={cellsFilter === item.id}
							aria-pressed={cellsFilter === item.id}
							onclick={() => (cellsFilter = item.id)}>{item.label}</button
						>
					{/each}
				</div>
			{/if}

			<span class="run-count">{visibleRuns.length} runs</span>
		</div>

		<div class="table-scroll">
			<table class="results-table">
				<thead>
					<tr>
						<th scope="col">Framework</th>
						<th scope="col">Mode</th>
						<th scope="col">Cells</th>
						<th scope="col">Ready <span>ms</span></th>
						<th scope="col">FPS</th>
						<th scope="col">CPU <span>ms/frame</span></th>
						<th scope="col">GPU 3D <span>%</span></th>
						<th scope="col">GPU mem <span>MB</span></th>
						<th scope="col">RAM ws <span>MB</span></th>
						<th scope="col">RAM priv <span>MB</span></th>
						<th scope="col">Rebuild <span>p50 µs</span></th>
					</tr>
				</thead>
				<tbody>
					{#each visibleRuns as run}
						<tr class:win={run.framework === 'sequoia'}>
							<td class="fw-cell">
								<span class="fw-name">{fwName(run.framework)}</span>
								<span class="fw-version">{fwVersion(run.framework)}</span>
							</td>
							<td><span class="mode">{run.mode}</span></td>
							<td>{run.cells ?? '—'}</td>
							<td>{num(run.ready_wall_ms, 1)}</td>
							<td>{num(run.fps)}</td>
							<td>{num(run.cpu_ms_per_frame)}</td>
							<td>{num(run.gpu_util_pct)}</td>
							<td>{num(run.gpu_mem_mb)}</td>
							<td>{num(run.ram_working_set_mb)}</td>
							<td>{num(run.ram_private_mb)}</td>
							<td>{num(run.rebuild_p50_us, 0)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		<p class="table-hint">
			Scroll horizontally for all columns. “—” means the metric was not recorded for that run.
		</p>
	</section>

	<section class="bench-section" id="method" aria-labelledby="method-title">
		<div class="section-head">
			<div>
				<span class="section-number">04</span>
				<p class="section-label">Methodology</p>
				<h2 id="method-title" class="section-title">How the numbers were produced</h2>
			</div>
			<p class="section-note">
				The harness runs each framework from the same release build, in one session, with
				identical scenes and timing protocol.
			</p>
		</div>

		<div class="method-grid">
			<div class="method-col">
				<dl class="facts">
					{#each facts as fact}
						<div class="fact">
							<dt>{fact.label}</dt>
							<dd>{fact.value}</dd>
						</div>
					{/each}
				</dl>
			</div>

			<div class="method-col">
				<h3>Read with care</h3>
				<ul class="caveats">
					{#each caveats as caveat}
						<li>{caveat}</li>
					{/each}
				</ul>
			</div>
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
	.bench-hero h1 .accent {
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
	.lede .accent {
		color: var(--accent, #e6391e);
		font-weight: 700;
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
		transition: color 0.2s ease;
	}
	.hero-link:hover {
		color: var(--accent, #e6391e);
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

	.snapshot-table {
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 16px;
		padding: var(--space-2) var(--space-3) var(--space-3);
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
		color: #8a8a8e;
		white-space: nowrap;
	}
	thead th:first-child {
		text-align: left;
	}
	thead th span {
		display: block;
		margin-top: 2px;
		font-family: var(--font-mono, monospace);
		font-size: var(--text-2xs);
		font-weight: 400;
		letter-spacing: 0;
		text-transform: none;
		color: #a0a0a4;
	}

	tbody td {
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid #ededf0;
		text-align: right;
		font-family: var(--font-mono, monospace);
		font-size: var(--text-sm);
		font-weight: 600;
		white-space: nowrap;
	}
	tbody tr:last-child td {
		border-bottom: 0;
	}
	tbody td:first-child {
		text-align: left;
		font-family: inherit;
	}
	tbody tr.win td {
		background: rgba(230, 57, 30, 0.035);
	}
	tbody tr.win td:first-child {
		box-shadow: inset 3px 0 0 var(--accent, #e6391e);
	}
	tbody tr.win td:first-child .fw-name {
		color: var(--accent, #e6391e);
	}
	tbody td.best {
		font-weight: 800;
		color: var(--accent, #e6391e);
	}

	.fw-name {
		display: block;
		font-size: var(--text-sm);
		font-weight: 700;
		letter-spacing: -0.01em;
	}
	.fw-version {
		display: block;
		margin-top: 2px;
		font-family: var(--font-mono, monospace);
		font-size: var(--text-2xs);
		font-weight: 400;
		color: #9a9a9e;
	}
	.unit {
		margin-left: 2px;
		font-size: var(--text-2xs);
		font-weight: 400;
		color: #8a8a8e;
	}

	.table-hint {
		margin-top: var(--space-2);
		font-size: var(--text-2xs);
		line-height: 1.5;
		color: #8a8a8e;
		text-align: right;
	}

	.highlights {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: var(--space-4);
		margin-top: var(--space-5);
	}
	.highlight {
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 14px;
		padding: var(--space-5);
	}
	.highlight-value {
		font-size: var(--text-3xl);
		font-weight: 800;
		letter-spacing: var(--tracking-tighter);
		line-height: 1.1;
		color: #1a1a1a;
	}
	.highlight-label {
		margin-top: var(--space-2);
		font-size: var(--text-sm);
		font-weight: 700;
		line-height: 1.35;
	}
	.highlight-desc {
		margin-top: var(--space-2);
		font-size: var(--text-xs);
		line-height: 1.5;
		color: #777;
	}

	.results-head {
		align-items: flex-end;
	}

	.filter-bar {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		flex-wrap: wrap;
		margin-bottom: var(--space-4);
	}
	.tabs {
		display: inline-flex;
		padding: var(--space-1);
		border-radius: 10px;
		background: #e9e9ec;
	}
	.tabs button {
		padding: var(--space-2) var(--space-4);
		border: 0;
		border-radius: 7px;
		background: transparent;
		font-family: inherit;
		font-size: var(--text-xs);
		font-weight: 600;
		color: #666;
		cursor: pointer;
		transition:
			background 0.18s ease,
			color 0.18s ease,
			box-shadow 0.18s ease;
	}
	.tabs button:hover {
		color: #1a1a1a;
	}
	.tabs button.active {
		background: #fff;
		color: #1a1a1a;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
	}
	.tabs.small button {
		padding: var(--space-2) var(--space-3);
		font-family: var(--font-mono, monospace);
		font-size: var(--text-xs);
	}
	.run-count {
		margin-left: auto;
		font-family: var(--font-mono, monospace);
		font-size: var(--text-2xs);
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: #8a8a8e;
	}

	.table-scroll {
		overflow-x: auto;
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 16px;
		-webkit-overflow-scrolling: touch;
	}
	.table-scroll::-webkit-scrollbar {
		height: 8px;
	}
	.table-scroll::-webkit-scrollbar-thumb {
		background: #ddd;
		border-radius: 8px;
	}

	.results-table {
		min-width: 1080px;
	}
	.results-table thead th {
		border-bottom: 1px solid #d5d5d8;
		padding: var(--space-3) var(--space-4);
		font-size: var(--text-2xs);
	}
	.results-table tbody td {
		font-size: var(--text-xs);
		padding: var(--space-3) var(--space-4);
	}
	.results-table tbody tr:hover td {
		background: #fafafa;
	}
	.results-table tbody tr.win:hover td {
		background: rgba(230, 57, 30, 0.05);
	}
	.results-table .fw-name {
		font-size: var(--text-sm);
	}
	.results-table .fw-cell {
		min-width: 160px;
	}
	.mode {
		display: inline-block;
		padding: 2px var(--space-2);
		border-radius: 20px;
		background: #f1f1f3;
		font-family: var(--font-mono, monospace);
		font-size: var(--text-2xs);
		font-weight: 500;
		color: #555;
	}

	.method-grid {
		display: grid;
		grid-template-columns: minmax(0, 0.9fr) minmax(0, 1.1fr);
		gap: var(--space-7);
	}
	.facts {
		border-top: 1px solid var(--border, #d5d5d8);
	}
	.fact {
		display: grid;
		grid-template-columns: 130px 1fr;
		gap: var(--space-4);
		padding: var(--space-3) 0;
		border-bottom: 1px solid var(--border, #d5d5d8);
	}
	.fact dt {
		font-family: var(--font-mono, monospace);
		font-size: var(--text-2xs);
		font-weight: 600;
		letter-spacing: var(--tracking-wide);
		text-transform: uppercase;
		color: #8a8a8e;
		padding-top: var(--space-1);
	}
	.fact dd {
		font-size: var(--text-sm);
		line-height: 1.55;
		color: #3d3d3f;
	}
	.method-col h3 {
		font-size: var(--text-sm);
		font-weight: 700;
		letter-spacing: 0.02em;
		margin-bottom: var(--space-4);
	}
	.caveats {
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.caveats li {
		position: relative;
		padding-left: var(--space-5);
		font-size: var(--text-sm);
		line-height: var(--leading-normal);
		color: #555;
	}
	.caveats li::before {
		content: '';
		position: absolute;
		left: 0;
		top: 8px;
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--accent, #e6391e);
	}

	@media (max-width: 1000px) {
		.highlights {
			grid-template-columns: repeat(2, 1fr);
		}
		.method-grid {
			grid-template-columns: 1fr;
			gap: var(--space-6);
		}
	}

	@media (max-width: 768px) {
		.bench-page {
			padding: 0 var(--section-px) var(--space-7);
		}
		.bench-hero {
			padding: var(--space-7) 0 var(--space-6);
		}
		.bench-hero h1 {
			font-size: var(--text-3xl);
		}
		.lede {
			font-size: var(--text-base);
		}
		.bench-section {
			padding: var(--space-7) 0;
		}
		.section-head {
			flex-direction: column;
			align-items: flex-start;
			gap: var(--space-3);
		}
		.section-note {
			max-width: 100%;
		}
		.snapshot-table {
			padding: var(--space-1) 0 var(--space-2);
			overflow-x: auto;
		}
		.snapshot-table table {
			min-width: 560px;
		}
		thead th {
			padding: var(--space-3);
			font-size: var(--text-2xs);
		}
		tbody td {
			padding: var(--space-3);
			font-size: var(--text-sm);
		}
		.highlights {
			grid-template-columns: 1fr;
		}
		.filter-bar {
			align-items: flex-start;
		}
		.run-count {
			margin-left: 0;
			width: 100%;
		}
		.fact {
			grid-template-columns: 110px 1fr;
			gap: var(--space-3);
		}
		.table-hint {
			text-align: left;
		}
	}
</style>
