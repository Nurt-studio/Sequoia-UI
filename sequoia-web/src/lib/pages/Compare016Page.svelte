<script>
	import DocsTopBar from '../components/DocsTopBar.svelte';
	import DocsFooter from '../components/DocsFooter.svelte';
	import { nav } from '../navigation.svelte.js';
	import { baseline, current } from '../version-compare-data.js';

	const oldVersion = baseline.version;
	const newVersion = current.version;

	/**
	 * @param {'0.1.5' | '0.1.6'} set
	 * @param {string} scenario
	 * @param {string} mode
	 * @param {number} cells
	 */
	function findRun(set, scenario, mode, cells) {
		const source = set === '0.1.5' ? baseline.runs : current.runs;
		const run = source.find(
			(item) =>
				item.scenario === scenario && item.mode === mode && item.cells === cells
		);
		if (!run) throw new Error(`Missing ${set} run: ${scenario}/${mode}/${cells}`);
		return run;
	}

	const oldSettings = findRun('0.1.5', 'settings', 'idle', 0);
	const newSettings = findRun('0.1.6', 'settings', 'idle', 0);
	const oldCells3000 = findRun('0.1.5', 'cells', 'anim', 3000);
	const newCells3000 = findRun('0.1.6', 'cells', 'anim', 3000);
	const oldCells3000One = findRun('0.1.5', 'cells', 'one', 3000);
	const newCells3000One = findRun('0.1.6', 'cells', 'one', 3000);
	const oldRows3000 = findRun('0.1.5', 'rows', 'anim', 3000);
	const newRows3000 = findRun('0.1.6', 'rows', 'anim', 3000);
	const oldRows3000One = findRun('0.1.5', 'rows', 'one', 3000);
	const newRows3000One = findRun('0.1.6', 'rows', 'one', 3000);

	const cards = [
		{
			value: '-34.4 ms',
			label: 'Settings startup',
			detail: '186.2 to 151.8 ms'
		},
		{
			value: '-15.7 ms',
			label: '3,000 cells',
			detail: '218.2 to 202.5 ms'
		},
		{
			value: '-51.9 ms',
			label: '3,000 rows',
			detail: '244.2 to 192.2 ms'
		},
		{
			value: '-4.9 pp',
			label: 'GPU 3D, cells',
			detail: '23.8% to 18.9%'
		}
	];

	const headline = [
		{
			label: 'Settings to first frame',
			hint: 'startup / idle',
			old: oldSettings.ready_wall_ms,
			new: newSettings.ready_wall_ms,
			unit: 'ms',
			deltaUnit: 'ms',
			digits: 2,
			status: 'improved'
		},
		{
			label: '3,000 cells to first frame',
			hint: 'cells / anim',
			old: oldCells3000.ready_wall_ms,
			new: newCells3000.ready_wall_ms,
			unit: 'ms',
			deltaUnit: 'ms',
			digits: 2,
			status: 'improved'
		},
		{
			label: '3,000 rows to first frame',
			hint: 'rows / anim',
			old: oldRows3000.ready_wall_ms,
			new: newRows3000.ready_wall_ms,
			unit: 'ms',
			deltaUnit: 'ms',
			digits: 2,
			status: 'improved'
		},
		{
			label: '3,000 cells GPU 3D',
			hint: 'same 60 fps scene',
			old: oldCells3000.gpu_util_pct,
			new: newCells3000.gpu_util_pct,
			unit: '%',
			deltaUnit: 'pp',
			digits: 2,
			status: 'improved'
		},
		{
			label: '3,000 rows GPU 3D',
			hint: 'same 60 fps scene',
			old: oldRows3000.gpu_util_pct,
			new: newRows3000.gpu_util_pct,
			unit: '%',
			deltaUnit: 'pp',
			digits: 2,
			status: 'improved'
		},
		{
			label: 'Rows rebuild p50',
			hint: '3,000 rows',
			old: (oldRows3000.rebuild_p50_us ?? 0) / 1000,
			new: (newRows3000.rebuild_p50_us ?? 0) / 1000,
			unit: 'ms',
			deltaUnit: 'ms',
			digits: 2,
			status: 'improved'
		}
	];

	const timeRows = [
		{ label: 'Settings', hint: 'startup', old: oldSettings, new: newSettings },
		{ label: '3,000 cells', hint: 'anim', old: oldCells3000, new: newCells3000 },
		{ label: '3,000 cells', hint: 'one', old: oldCells3000One, new: newCells3000One },
		{ label: '3,000 rows', hint: 'anim', old: oldRows3000, new: newRows3000 },
		{ label: '3,000 rows', hint: 'one', old: oldRows3000One, new: newRows3000One }
	];

	const runtimeRows = [
		{ label: '3,000 cells', hint: 'anim', old: oldCells3000, new: newCells3000 },
		{ label: '3,000 cells', hint: 'one', old: oldCells3000One, new: newCells3000One },
		{ label: '3,000 rows', hint: 'anim', old: oldRows3000, new: newRows3000 },
		{ label: '3,000 rows', hint: 'one', old: oldRows3000One, new: newRows3000One }
	];

	const scalingRows = [100, 1000, 3000].map((cells) => ({
		cells,
		old: findRun('0.1.5', 'cells', 'anim', cells),
		new: findRun('0.1.6', 'cells', 'anim', cells)
	}));

	/** @type {Record<string, { label: string, tone: string }>} */
	const badges = {
		improved: { label: 'Improved', tone: 'good' }
	};

	/**
	 * @param {number|null|undefined} value
	 * @param {number} [digits]
	 */
	function num(value, digits = 1) {
		if (value == null) return '-';
		const factor = 10 ** digits;
		return String(Math.round(value * factor) / factor);
	}

	/**
	 * @param {number|null|undefined} value
	 * @param {string} unit
	 * @param {number} [digits]
	 */
	function display(value, unit, digits = 1) {
		return value == null ? '-' : `${num(value, digits)} ${unit}`;
	}

	/**
	 * @param {number|null|undefined} value
	 * @param {number} [digits]
	 */
	function signed(value, digits = 1) {
		if (value == null) return '-';
		const factor = 10 ** digits;
		const rounded = Math.round(value * factor) / factor;
		return `${rounded > 0 ? '+' : ''}${rounded}`;
	}

	/**
	 * @param {number|null|undefined} oldValue
	 * @param {number|null|undefined} newValue
	 */
	function percentChange(oldValue, newValue) {
		if (oldValue == null || newValue == null || !oldValue) return '-';
		const value = ((newValue - oldValue) / oldValue) * 100;
		return `${value > 0 ? '+' : ''}${value.toFixed(1)}%`;
	}

	/**
	 * @param {number|null|undefined} oldValue
	 * @param {number|null|undefined} newValue
	 */
	function difference(oldValue, newValue) {
		if (oldValue == null || newValue == null) return null;
		return newValue - oldValue;
	}

	/**
	 * @param {number|null|undefined} value
	 * @param {boolean} [lowerIsBetter]
	 */
	function trend(value, lowerIsBetter = true) {
		if (value == null || Math.abs(value) < 0.0001) return 'neutral';
		return (lowerIsBetter ? value < 0 : value > 0) ? 'good' : 'bad';
	}
</script>

<DocsTopBar version={newVersion} />

<main class="compare-page">
	<header class="hero">
		<a
			class="back"
			href="/"
			onclick={(e) => {
				e.preventDefault();
				nav.page = 'benchmarks';
			}}>&larr; Benchmarks</a
		>

		<div class="eyebrow">
			<span class="eyebrow-tag">Performance (0.1.6 Not Yet Released)</span>
			<span class="eyebrow-date">Upcoming Release</span>
		</div>
		<h1>0.1.6 (Unreleased) vs <span>0.1.5</span>: faster to the first frame, less GPU work.</h1>
		<p class="lede">
			The same 100, 1,000 and 3,000-node scenes, measured on one Windows 11 machine. The
			0.1.6 build gets to the first frame sooner, keeps 60 fps with a lower GPU load, and
			rebuilds the 3,000-row scene faster.
		</p>
		<div class="hero-actions">
			<a
				href="/"
				class="button"
				onclick={(e) => {
					e.preventDefault();
					nav.page = 'benchmarks';
				}}>See the cross-framework results</a
			>
			<a
				href="/"
				class="text-link"
				onclick={(e) => {
					e.preventDefault();
					nav.page = 'release-0.1.6';
				}}>Read the 0.1.6 release notes &rarr;</a
			>
		</div>

		<div class="stat-grid">
			{#each cards as card}
				<div class="stat">
					<strong>{card.value}</strong>
					<span class="stat-label">{card.label}</span>
					<span class="stat-detail">{card.detail}</span>
				</div>
			{/each}
		</div>
	</header>

	<section class="section">
		<div class="section-head">
			<div>
				<span class="section-number">01</span>
				<p class="section-label">At a glance</p>
				<h2>What moved in this release</h2>
			</div>
			<p class="section-note">
				Only the metrics with a clear directional result are shown here. Runtime and
				measurement caveats are in the reference tables below.
			</p>
		</div>

		<div class="table-wrap">
			<table class="compare-table">
				<thead>
					<tr>
						<th scope="col">Metric</th>
						<th scope="col">{oldVersion}</th>
						<th scope="col">{newVersion}</th>
						<th scope="col">Change</th>
						<th scope="col">Verdict</th>
					</tr>
				</thead>
				<tbody>
					{#each headline as row}
						{@const badge = badges[row.status]}
						{@const change = difference(row.old, row.new)}
						<tr>
							<td>
								<span class="metric">{row.label}</span>
								<span class="hint">{row.hint}</span>
							</td>
							<td class="value">{display(row.old, row.unit, row.digits)}</td>
							<td class="value strong">{display(row.new, row.unit, row.digits)}</td>
							<td class="value">
								<span class="change {trend(change)}">
									{signed(change, row.digits)} {row.deltaUnit}
									<small>{percentChange(row.old, row.new)}</small>
								</span>
							</td>
							<td><span class="badge {badge.tone}">{badge.label}</span></td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</section>

	<section class="section">
		<div class="section-head">
			<div>
				<span class="section-number">02</span>
				<p class="section-label">Startup</p>
				<h2>Time to the first presented frame</h2>
			</div>
			<p class="section-note">
				Wall time includes process spawn. The app-self timer starts inside the benchmark
				process and excludes most of that overhead.
			</p>
		</div>

		<div class="table-wrap">
			<table class="compare-table compact">
				<thead>
					<tr>
						<th scope="col">Scene</th>
						<th scope="col">Mode</th>
						<th scope="col">{oldVersion} wall</th>
						<th scope="col">{newVersion} wall</th>
						<th scope="col">Wall delta</th>
						<th scope="col">{oldVersion} self</th>
						<th scope="col">{newVersion} self</th>
						<th scope="col">Self delta</th>
					</tr>
				</thead>
				<tbody>
					{#each timeRows as row}
						<tr>
							<td class="scene">{row.label}</td>
							<td class="mode">{row.hint}</td>
							<td class="value">{num(row.old.ready_wall_ms, 2)} ms</td>
							<td class="value strong">{num(row.new.ready_wall_ms, 2)} ms</td>
							<td class="value {trend(row.new.ready_wall_ms - row.old.ready_wall_ms)}">
								{signed(row.new.ready_wall_ms - row.old.ready_wall_ms, 2)} ms
							</td>
							<td class="value">{num(row.old.ready_self_ms, 0)} ms</td>
							<td class="value strong">{num(row.new.ready_self_ms, 0)} ms</td>
							<td class="value {trend(row.new.ready_self_ms - row.old.ready_self_ms)}">
								{signed(row.new.ready_self_ms - row.old.ready_self_ms, 0)} ms
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</section>

	<section class="section">
		<div class="section-head">
			<div>
				<span class="section-number">03</span>
				<p class="section-label">Reference</p>
				<h2>Runtime, CPU and memory</h2>
			</div>
			<p class="section-note">
				Technical reference, not a headline claim. GPU load is the percentage of 3D engine
				time while the scene animates.
			</p>
		</div>

		<div class="table-wrap">
			<table class="compare-table compact">
				<thead>
					<tr>
						<th scope="col">Scene</th>
						<th scope="col">FPS</th>
						<th scope="col">CPU / frame</th>
						<th scope="col">GPU 3D</th>
						<th scope="col">RAM working set</th>
					</tr>
				</thead>
				<tbody>
					{#each runtimeRows as row}
						<tr>
							<td>
								<span class="scene">{row.label}</span>
								<span class="mode">{row.hint}</span>
							</td>
							<td class="value">
								<span class="pair">{num(row.old.fps, 2)}<span class="to">&rarr;</span
									>{num(row.new.fps, 2)}</span>
								<small class="change neutral">{signed(difference(row.old.fps, row.new.fps), 2)}</small>
							</td>
							<td class="value">
								<span class="pair">{num(row.old.cpu_ms_per_frame, 2)}<span class="to"
										>&rarr;</span
									>{num(row.new.cpu_ms_per_frame, 2)}</span>
								<small class="change warn"
									>{signed(
										difference(row.old.cpu_ms_per_frame, row.new.cpu_ms_per_frame),
										2
									)}
									ms</small
								>
							</td>
							<td class="value">
								<span class="pair"
									>{num(row.old.gpu_util_pct, 2)}<span class="to">&rarr;</span
									>{num(row.new.gpu_util_pct, 2)}</span
								>
								<small class="change good"
									>{signed(row.new.gpu_util_pct - row.old.gpu_util_pct, 2)} pp</small
								>
							</td>
							<td class="value">
								<span class="pair"
									>{num(row.old.ram_working_set_mb, 1)}<span class="to">&rarr;</span
									>{num(row.new.ram_working_set_mb, 1)}</span
								>
								<small class="change neutral"
									>{signed(
										difference(
											row.old.ram_working_set_mb,
											row.new.ram_working_set_mb
										),
										1
									)} MB</small
								>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		<p class="table-note">
			CPU/frame is not an improvement claim. Adjacent clean A/B sessions of the current
			build ranged from roughly 2.6 to 7.4 ms per frame, so the movement here is within
			run-to-run noise. FPS is pinned at 60 by the display.
		</p>
	</section>

	<section class="section">
		<div class="section-head">
			<div>
				<span class="section-number">04</span>
				<p class="section-label">Scaling</p>
				<h2>Animated cells, 100 to 3,000</h2>
			</div>
			<p class="section-note">
				Each cell count was measured with the same animated cell scene and a 10 s window
				after 3 s of warm-up.
			</p>
		</div>

		<div class="table-wrap">
			<table class="compare-table compact">
				<thead>
					<tr>
						<th scope="col">Cells</th>
						<th scope="col">Ready wall</th>
						<th scope="col">Ready self</th>
						<th scope="col">CPU / frame</th>
						<th scope="col">GPU 3D</th>
					</tr>
				</thead>
				<tbody>
					{#each scalingRows as row}
						<tr>
							<td class="scene">{row.cells.toLocaleString('en-US')}</td>
							<td class="value">
								<span class="pair"
									>{num(row.old.ready_wall_ms, 1)}<span class="to">&rarr;</span
									>{num(row.new.ready_wall_ms, 1)}</span
								>
								<small class="change {trend(row.new.ready_wall_ms - row.old.ready_wall_ms)}"
									>{signed(row.new.ready_wall_ms - row.old.ready_wall_ms, 1)} ms</small
								>
							</td>
							<td class="value">
								<span class="pair"
									>{num(row.old.ready_self_ms, 0)}<span class="to">&rarr;</span
									>{num(row.new.ready_self_ms, 0)}</span
								>
								<small class="change {trend(row.new.ready_self_ms - row.old.ready_self_ms)}"
									>{signed(row.new.ready_self_ms - row.old.ready_self_ms, 0)} ms</small
								>
							</td>
							<td class="value">
								<span class="pair"
									>{num(row.old.cpu_ms_per_frame, 2)}<span class="to">&rarr;</span
									>{num(row.new.cpu_ms_per_frame, 2)}</span
								>
								<small class="change warn"
									>{signed(
										difference(row.old.cpu_ms_per_frame, row.new.cpu_ms_per_frame),
										2
									)}
									ms</small
								>
							</td>
							<td class="value">
								<span class="pair"
									>{num(row.old.gpu_util_pct, 2)}<span class="to">&rarr;</span
									>{num(row.new.gpu_util_pct, 2)}</span
								>
								<small class="change good"
									>{signed(row.new.gpu_util_pct - row.old.gpu_util_pct, 2)} pp</small
								>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		<p class="table-note">
			The 100-cell entries come from a separate rerun: the first 100-cell animated rep in
			the full session was an opener outlier. The 1,000 and 3,000-cell entries are from the
			main 0.1.6 session.
		</p>
	</section>

	<section class="section split">
		<div>
			<span class="section-number">05</span>
			<p class="section-label">Performance work</p>
			<h2>What was done, and what it bought</h2>
			<p class="work-summary">
				0.1.6 moves GPU pipeline setup off the startup path, renders directly into the view
				target, and reduces the 3,000-row rebuild cost. Net effect: the same 60 fps, a
				faster first frame and less GPU time in both scenes.
			</p>
			<div class="work-list">
				<article class="work">
					<div class="work-head">
						<span class="work-num">01</span>
						<h3>Pre-initialized pipelines</h3>
					</div>
					<p>
						Pipeline construction starts before the first presented frame, so the first
						draw does not pay for GPU setup.
					</p>
					<div class="work-gain">
						<strong>-34.4 ms</strong>
						<span>settings startup</span>
						<small>3,000 cells -15.7 ms &middot; rows -51.9 ms</small>
					</div>
				</article>
				<article class="work">
					<div class="work-head">
						<span class="work-num">02</span>
						<h3>Lazy pipelines + direct-to-view</h3>
					</div>
					<p>
						Only the pipelines a pass needs are created, and rendering goes straight to the
						view target instead of an intermediate target.
					</p>
					<div class="work-gain">
						<strong>-4.9 pp</strong>
						<span>GPU 3D, 3,000 cells</span>
						<small>23.8% to 18.9% &middot; rows 11.2% to 8.0%</small>
					</div>
				</article>
				<article class="work">
					<div class="work-head">
						<span class="work-num">03</span>
						<h3>Row rebuild path</h3>
					</div>
					<p>
						The 3,000-row reactive rebuild probe is faster, with the same 60 fps output.
					</p>
					<div class="work-gain">
						<strong>-0.45 ms</strong>
						<span>rows rebuild p50</span>
						<small>4.31 ms to 3.85 ms &middot; -10.5%</small>
					</div>
				</article>
			</div>
		</div>
		<aside class="method">
			<h3>Method</h3>
			<dl>
				<div>
					<dt>Machine</dt>
					<dd>Windows 11, one session per build</dd>
				</div>
				<div>
					<dt>Window</dt>
					<dd>10 s measured after 3 s warm-up, median of 3 reps</dd>
				</div>
				<div>
					<dt>Build</dt>
					<dd>Release, LTO fat</dd>
				</div>
				<div>
					<dt>Baseline</dt>
					<dd>
						0.1.5 used the previous runner. Wall, self, FPS, CPU, GPU 3D and RAM are
						comparable.
					</dd>
				</div>
				<div>
					<dt>GPU memory</dt>
					<dd>
						0.1.5 reported adapter-wide memory; 0.1.6 reports per-process committed
						memory. The counters are not compared.
					</dd>
				</div>
			</dl>
			<p>
				These are reference numbers from one machine, not a universal ranking. The
				cross-framework page uses its own frozen 0.1.5 session.
			</p>
		</aside>
	</section>
</main>

<DocsFooter />

<style>
	.compare-page {
		max-width: 1180px;
		margin: 0 auto;
		padding: var(--space-7) var(--section-px) var(--space-9);
		color: var(--fg);
	}

	.hero {
		padding: var(--space-6) 0 var(--space-7);
		border-bottom: 1px solid var(--border);
	}
	.back {
		display: inline-block;
		margin-bottom: var(--space-5);
		color: #777;
		text-decoration: none;
		font-size: var(--text-sm);
		font-weight: 600;
	}
	.back:hover {
		color: var(--accent);
	}
	.eyebrow {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		margin-bottom: var(--space-3);
	}
	.eyebrow-tag {
		padding: var(--space-1) var(--space-2);
		border-radius: 999px;
		background: rgba(230, 57, 30, 0.08);
		color: var(--accent);
		font-size: var(--text-2xs);
		font-weight: 700;
		letter-spacing: var(--tracking-wide);
		text-transform: uppercase;
	}
	.eyebrow-date {
		color: #888;
		font-size: var(--text-xs);
		font-weight: 600;
	}
	h1 {
		max-width: 920px;
		margin: 0;
		font-size: clamp(2.2rem, 5.2vw, 4.4rem);
		font-weight: 750;
		letter-spacing: var(--tracking-tight);
		line-height: 1.02;
	}
	h1 span {
		color: var(--accent);
	}
	.lede {
		max-width: 760px;
		margin: var(--space-4) 0 0;
		color: #555;
		font-size: var(--text-lg);
		line-height: 1.6;
	}
	.hero-actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-4);
		margin-top: var(--space-5);
	}
	.button {
		display: inline-flex;
		align-items: center;
		padding: var(--space-2) var(--space-4);
		border-radius: 8px;
		background: var(--fg);
		color: var(--bg);
		text-decoration: none;
		font-size: var(--text-sm);
		font-weight: 700;
	}
	.button:hover {
		background: var(--accent);
	}
	.text-link {
		color: var(--accent);
		text-decoration: none;
		font-size: var(--text-sm);
		font-weight: 600;
	}
	.text-link:hover {
		text-decoration: underline;
	}
	.stat-grid {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: var(--space-3);
		margin-top: var(--space-6);
	}
	.stat {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: var(--space-4);
		border: 1px solid var(--border);
		border-radius: 12px;
		background: #fff;
	}
	.stat strong {
		color: var(--accent);
		font-size: clamp(1.5rem, 3vw, 2.2rem);
		letter-spacing: var(--tracking-tight);
		line-height: 1;
	}
	.stat-label {
		font-size: var(--text-sm);
		font-weight: 700;
	}
	.stat-detail {
		color: #888;
		font-size: var(--text-xs);
		font-variant-numeric: tabular-nums;
	}
	.section {
		padding: var(--space-7) 0;
		border-bottom: 1px solid var(--border);
	}
	.section-head {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(260px, 0.55fr);
		gap: var(--space-5);
		align-items: end;
		margin-bottom: var(--space-5);
	}
	.section-number {
		display: block;
		margin-bottom: var(--space-1);
		color: var(--accent);
		font-size: var(--text-2xs);
		font-weight: 700;
		letter-spacing: var(--tracking-wide);
	}
	.section-label {
		margin: 0 0 var(--space-1);
		color: #888;
		font-size: var(--text-2xs);
		font-weight: 700;
		letter-spacing: var(--tracking-wide);
		text-transform: uppercase;
	}
	h2 {
		margin: 0;
		font-size: clamp(1.6rem, 3vw, 2.4rem);
		letter-spacing: var(--tracking-tight);
		line-height: 1.1;
	}
	.section-note {
		margin: 0;
		color: #666;
		font-size: var(--text-sm);
		line-height: 1.6;
	}
	.table-wrap {
		overflow-x: auto;
		border: 1px solid var(--border);
		border-radius: 12px;
		background: #fff;
	}
	.compare-table {
		width: 100%;
		min-width: 760px;
		border-collapse: collapse;
		font-size: var(--text-sm);
		font-variant-numeric: tabular-nums;
	}
	.compare-table.compact {
		min-width: 860px;
	}
	.compare-table th {
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid var(--border);
		color: #888;
		font-size: var(--text-2xs);
		font-weight: 700;
		letter-spacing: var(--tracking-wide);
		text-align: left;
		text-transform: uppercase;
		white-space: nowrap;
	}
	.compare-table td {
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid #f0f0f0;
		vertical-align: middle;
	}
	.compare-table tr:last-child td {
		border-bottom: 0;
	}
	.compare-table tbody tr:hover {
		background: #fafafa;
	}
	.metric {
		display: block;
		font-weight: 650;
	}
	.hint,
	.mode {
		display: block;
		margin-top: 2px;
		color: #999;
		font-size: var(--text-xs);
		font-weight: 500;
	}
	.scene {
		font-weight: 650;
		white-space: nowrap;
	}
	.value {
		white-space: nowrap;
	}
	.value.strong {
		font-weight: 700;
	}
	.change {
		display: block;
		font-size: var(--text-xs);
		font-weight: 700;
	}
	.change small {
		display: block;
		margin-top: 1px;
		color: #999;
		font-size: 0.68rem;
		font-weight: 600;
	}
	.change.good {
		color: #2d7a46;
	}
	.change.bad {
		color: #c0392b;
	}
	.change.warn {
		color: #a26a00;
	}
	.change.neutral {
		color: #777;
	}
	.badge {
		display: inline-block;
		padding: 3px 8px;
		border-radius: 999px;
		font-size: var(--text-2xs);
		font-weight: 700;
		white-space: nowrap;
	}
	.badge.good {
		background: #e7f4eb;
		color: #2d7a46;
	}
	.badge.neutral {
		background: #f1f1f1;
		color: #666;
	}
	.badge.warn {
		background: #fff4dc;
		color: #8a5b00;
	}
	.pair {
		font-weight: 650;
	}
	.to {
		padding: 0 var(--space-1);
		color: #aaa;
		font-weight: 500;
	}
	.table-note {
		margin: var(--space-3) 0 0;
		color: #777;
		font-size: var(--text-xs);
		line-height: 1.6;
	}
	.split {
		display: grid;
		grid-template-columns: minmax(0, 1.15fr) minmax(280px, 0.85fr);
		gap: var(--space-7);
		border-bottom: 0;
	}
	.work-summary {
		margin: var(--space-4) 0 var(--space-4);
		color: #555;
		line-height: 1.65;
	}
	.work-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.work {
		padding: var(--space-4);
		border: 1px solid var(--border);
		border-radius: 12px;
		background: #fff;
	}
	.work-head {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
	}
	.work-num {
		color: var(--accent);
		font-size: var(--text-2xs);
		font-weight: 700;
		letter-spacing: var(--tracking-wide);
	}
	.work h3 {
		margin: 0;
		font-size: var(--text-base);
		letter-spacing: var(--tracking-tight);
	}
	.work p {
		margin: var(--space-2) 0;
		color: #555;
		font-size: var(--text-sm);
		line-height: 1.55;
	}
	.work-gain {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		gap: 0 var(--space-2);
		align-items: baseline;
		padding-top: var(--space-2);
		border-top: 1px solid #f0f0f0;
	}
	.work-gain strong {
		color: var(--accent);
		font-size: var(--text-lg);
		letter-spacing: var(--tracking-tight);
	}
	.work-gain span {
		color: #666;
		font-size: var(--text-xs);
		font-weight: 650;
	}
	.work-gain small {
		grid-column: 1 / -1;
		margin-top: 2px;
		color: #999;
		font-size: var(--text-xs);
	}
	.method {
		padding: var(--space-4);
		border: 1px solid var(--border);
		border-radius: 12px;
		background: #fafafa;
	}
	.method h3 {
		margin: 0 0 var(--space-3);
		font-size: var(--text-base);
	}
	.method dl {
		margin: 0;
	}
	.method dl div {
		padding: var(--space-2) 0;
		border-bottom: 1px solid #e9e9e9;
	}
	.method dt {
		color: #888;
		font-size: var(--text-2xs);
		font-weight: 700;
		letter-spacing: var(--tracking-wide);
		text-transform: uppercase;
	}
	.method dd {
		margin: 3px 0 0;
		color: #555;
		font-size: var(--text-sm);
		line-height: 1.5;
	}
	.method p {
		margin: var(--space-3) 0 0;
		color: #777;
		font-size: var(--text-xs);
		line-height: 1.6;
	}
	@media (max-width: 860px) {
		.stat-grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
		.section-head,
		.split {
			grid-template-columns: 1fr;
			gap: var(--space-4);
		}
	}
	@media (max-width: 560px) {
		.compare-page {
			padding-top: var(--space-5);
		}
		.stat-grid {
			grid-template-columns: 1fr;
		}
		.hero-actions {
			align-items: flex-start;
			flex-direction: column;
		}
		.section {
			padding: var(--space-6) 0;
		}
	}
</style>
