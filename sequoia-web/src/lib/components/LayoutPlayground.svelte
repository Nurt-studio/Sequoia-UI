<script>
	const tabs = [
		{ id: 'grid', label: 'Grid' },
		{ id: 'flex', label: 'Row & Column' },
		{ id: 'wrap', label: 'Wrap' },
		{ id: 'stack', label: 'Stack' },
		{ id: 'dock', label: 'DockArea' }
	];

	/** @type {Record<string, string>} */
	const alignNames = { start: 'Start', center: 'Center', end: 'End', stretch: 'Stretch' };

	let tab = $state('grid');
	let frameWidth = $state(560);

	// Row / Column
	let rcDirection = $state('row');
	let rcGap = $state(12);
	let rcPadding = $state(16);
	let rcAlign = $state('stretch');
	let rcSidebar = $state(140);
	let rcWeight = $state(2);

	const alignCss = $derived(
		rcAlign === 'start'
			? 'flex-start'
			: rcAlign === 'end'
				? 'flex-end'
				: rcAlign === 'center'
					? 'center'
					: 'stretch'
	);

	const flexFrameStyle = $derived(
		`width:${frameWidth}px;padding:${rcPadding}px;gap:${rcGap}px;align-items:${alignCss};` +
			(rcDirection === 'column' ? 'flex-direction:column;' : '')
	);

	const flexSummary = $derived(
		`${rcDirection} · gap ${rcGap} px · align ${alignNames[rcAlign].toLowerCase()}`
	);

	const flexSnippet = $derived(
		`/* preview CSS */
display: flex;
flex-direction: ${rcDirection};
gap: ${rcGap}px;
padding: ${rcPadding}px;
align-items: ${alignCss};

// Rust
${rcDirection === 'row' ? 'Row' : 'Column'}::new()
    .gap(${rcGap}.0)
    .padding(${rcPadding}.0)
    .align_items(Align::${alignNames[rcAlign]})
    .child(
        sidebar
            ${rcDirection === 'row' ? `.width(${rcSidebar}.0)` : '.height(56.0)'}
            .flex_shrink(0.0),
    )
    .child_flex(content, ${rcWeight}.0)
    .child_flex(aside, 1.0);`
	);

	// Grid
	let gridMode = $state('auto');
	let gridCols = $state(3);
	let gridMin = $state(120);
	let gridGap = $state(12);
	let gridPadding = $state(12);
	let gridSpan = $state(true);

	const gridCards = [1, 2, 3, 4, 5, 6, 7];
	const gridInner = $derived(frameWidth - gridPadding * 2);
	const autoColumns = $derived(
		Math.max(1, Math.min(8, Math.floor((gridInner + gridGap) / (gridMin + gridGap))))
	);

	const gridTemplate = $derived(
		gridMode === 'auto'
			? `repeat(auto-fit, minmax(${gridMin}px, 1fr))`
			: gridMode === 'weighted'
				? '1.6fr 1fr 1fr'
				: gridMode === 'fixed'
					? '160px minmax(0, 1fr)'
					: `repeat(${gridCols}, minmax(0, 1fr))`
	);

	const gridColumnCount = $derived(
		gridMode === 'auto' ? autoColumns : gridMode === 'weighted' ? 3 : gridMode === 'fixed' ? 2 : gridCols
	);

	const gridFrameStyle = $derived(
		`width:${frameWidth}px;grid-template-columns:${gridTemplate};gap:${gridGap}px;padding:${gridPadding}px`
	);

	const gridSummary = $derived(
		gridMode === 'auto'
			? `${autoColumns} columns fit at ${frameWidth} px · min track ${gridMin} px`
			: gridMode === 'weighted'
				? '3 tracks · 1.6fr / 1fr / 1fr'
				: gridMode === 'fixed'
					? '160 px track + fill track'
					: `${gridCols} equal columns`
	);

	const equalTracks = $derived(
		Array.from({ length: gridCols }, () => '    Track::Fill(1.0),').join('\n')
	);

	const gridSpanLine = $derived(
		gridSpan ? `.child_span(header, 0, 0, ${gridColumnCount}, 1)\n` : ''
	);

	const gridRust = $derived(
		gridMode === 'auto'
			? `// Grid uses explicit tracks; add one per column.
// For automatic reflow use Wrap instead.
Grid::new([Track::minmax(${gridMin}.0, Track::Fill(1.0))])
    .gap(${gridGap}.0)
    .padding(${gridPadding}.0)
${gridSpanLine}    .children(cards);`
			: gridMode === 'weighted'
				? `Grid::new([
    Track::Fill(1.6),
    Track::Fill(1.0),
    Track::Fill(1.0),
])
    .gap(${gridGap}.0)
    .padding(${gridPadding}.0)
${gridSpanLine}    .children(cards);`
				: gridMode === 'fixed'
					? `Grid::new([
    Track::Px(160.0),
    Track::Fill(1.0),
])
    .gap(${gridGap}.0)
    .padding(${gridPadding}.0)
${gridSpanLine}    .children(cards);`
					: `Grid::new([
${equalTracks}
])
    .gap(${gridGap}.0)
    .padding(${gridPadding}.0)
${gridSpanLine}    .children(cards);`
	);

	const gridSnippet = $derived(
		`/* preview CSS */
display: grid;
grid-template-columns: ${gridTemplate};
gap: ${gridGap}px;
padding: ${gridPadding}px;

// Rust
${gridRust}`
	);

	// Wrap
	let wrapGap = $state(8);
	let wrapCard = $state(120);
	let wrapCount = $state(9);

	const wrapItems = $derived(Array.from({ length: wrapCount }, (_, i) => i + 1));
	const wrapPerRow = $derived(
		Math.max(1, Math.floor((frameWidth - 32 + wrapGap) / (wrapCard + wrapGap)))
	);
	const wrapSummary = $derived(`${wrapPerRow} cards per row · ${wrapCount} cards total`);

	const wrapSnippet = $derived(
		`/* preview CSS */
display: flex;
flex-wrap: wrap;
gap: ${wrapGap}px;
padding: 16px;

// Rust
Wrap::new()
    .gap(${wrapGap}.0)
    .children(cards);`
	);

	// Stack
	let stackOffset = $state(64);
	let stackFront = $state(true);
	const stackSummary = $derived(`${stackFront ? 'child 2' : 'child 1'} paints on top`);

	const stackSnippet = $derived(
		`/* preview CSS */
display: grid;
/* both children share one cell */
grid-area: 1 / 1;

// Rust — later children paint on top
Stack::new()
    .fill_size()
    .child(${stackFront ? 'panel' : 'floating_controls'})
    .child(${stackFront ? 'floating_controls' : 'panel'});`
	);

	// DockArea
	let dockTop = $state(true);
	let dockLeft = $state(true);
	let dockBottom = $state(false);
	let dockGap = $state(4);
	let dockPadding = $state(8);

	const dockSummary = $derived(
		[daoTopLabel(), dockLeft ? 'left' : '', dockBottom ? 'bottom' : ''].filter(Boolean).join(' · ') +
			' · fill'
	);

	function daoTopLabel() {
		return dockTop ? 'top' : '';
	}

	const dockSnippet = $derived(
		`/* preview CSS */
display: flex;
flex-direction: column;
gap: ${dockGap}px;
padding: ${dockPadding}px;

// Rust
DockArea::new()
${dockTop ? '    .top(toolbar)\n' : ''}${dockLeft ? '    .left(explorer)\n' : ''}${dockBottom ? '    .bottom(status)\n' : ''}    .fill(editor)
    .gap(${dockGap}.0)
    .padding(${dockPadding}.0);`
	);
</script>

<div class="playground">
	<div class="pg-tabs" role="tablist" aria-label="Layout previews">
		{#each tabs as item}
			<button
				type="button"
				role="tab"
				class:active={tab === item.id}
				aria-selected={tab === item.id}
				onclick={() => (tab = item.id)}>{item.label}</button
			>
		{/each}
	</div>

	<div class="pg-width">
		<label for="pg-width">Canvas width</label>
		<input id="pg-width" type="range" min="320" max="640" step="10" bind:value={frameWidth} />
		<span class="pg-value">{frameWidth} px</span>
		<div class="pg-presets">
			<button type="button" onclick={() => (frameWidth = 360)}>Narrow</button>
			<button type="button" onclick={() => (frameWidth = 480)}>Medium</button>
			<button type="button" onclick={() => (frameWidth = 640)}>Wide</button>
		</div>
	</div>

	{#if tab === 'flex'}
		<div class="pg-controls">
			<div class="pg-control">
				<span class="pg-label">Direction</span>
				<div class="pg-seg" role="group" aria-label="Flex direction">
					<button
						type="button"
						class:active={rcDirection === 'row'}
						aria-pressed={rcDirection === 'row'}
						onclick={() => (rcDirection = 'row')}>Row</button
					>
					<button
						type="button"
						class:active={rcDirection === 'column'}
						aria-pressed={rcDirection === 'column'}
						onclick={() => (rcDirection = 'column')}>Column</button
					>
				</div>
			</div>

			<label class="pg-control">
				<span class="pg-label">Gap <span class="pg-value">{rcGap} px</span></span>
				<input type="range" min="0" max="32" step="2" bind:value={rcGap} />
			</label>

			<label class="pg-control">
				<span class="pg-label">Padding <span class="pg-value">{rcPadding} px</span></span>
				<input type="range" min="0" max="32" step="2" bind:value={rcPadding} />
			</label>

			<label class="pg-control">
				<span class="pg-label">Align items</span>
				<select bind:value={rcAlign}>
					<option value="stretch">Stretch</option>
					<option value="start">Start</option>
					<option value="center">Center</option>
					<option value="end">End</option>
				</select>
			</label>

			<label class="pg-control">
				<span class="pg-label">Sidebar <span class="pg-value">{rcSidebar} px</span></span>
				<input type="range" min="80" max="220" step="10" bind:value={rcSidebar} />
			</label>

			<label class="pg-control">
				<span class="pg-label">Content weight <span class="pg-value">{rcWeight}.0</span></span>
				<input type="range" min="1" max="3" step="1" bind:value={rcWeight} />
			</label>
		</div>

		<div class="pg-stage">
			<div class="pg-stage-meta">
				<span>{flexSummary}</span>
				<span>HTML / CSS preview</span>
			</div>
			<div class="pg-frame pg-flex" style={flexFrameStyle}>
				<div
					class="pg-cell sidebar"
					style={rcDirection === 'row'
						? `width:${rcSidebar}px;flex:0 0 auto;`
						: 'min-height:56px;flex:0 0 auto;'}>
					<span class="cell-name">sidebar</span>
					<span class="cell-meta">{rcDirection === 'row' ? `${rcSidebar} px · fixed` : 'fixed height'}</span>
				</div>
				<div class="pg-cell content" style={`flex:${rcWeight} 1 0;min-width:0;`}>
					<span class="cell-name">content</span>
					<span class="cell-meta">child_flex · {rcWeight}.0</span>
				</div>
				<div class="pg-cell aside" style="flex:1 1 0;min-width:0;">
					<span class="cell-name">aside</span>
					<span class="cell-meta">child_flex · 1.0</span>
				</div>
			</div>
		</div>
	{:else if tab === 'grid'}
		<div class="pg-controls">
			<div class="pg-control">
				<span class="pg-label">Tracks</span>
				<div class="pg-seg" role="group" aria-label="Grid mode">
					<button
						type="button"
						class:active={gridMode === 'equal'}
						aria-pressed={gridMode === 'equal'}
						onclick={() => (gridMode = 'equal')}>Equal</button
					>
					<button
						type="button"
						class:active={gridMode === 'auto'}
						aria-pressed={gridMode === 'auto'}
						onclick={() => (gridMode = 'auto')}>Auto-fit</button
					>
					<button
						type="button"
						class:active={gridMode === 'weighted'}
						aria-pressed={gridMode === 'weighted'}
						onclick={() => (gridMode = 'weighted')}>Weighted</button
					>
					<button
						type="button"
						class:active={gridMode === 'fixed'}
						aria-pressed={gridMode === 'fixed'}
						onclick={() => (gridMode = 'fixed')}>Sidebar</button
					>
				</div>
			</div>

			{#if gridMode === 'equal'}
				<label class="pg-control">
					<span class="pg-label">Columns <span class="pg-value">{gridCols}</span></span>
					<input type="range" min="2" max="5" step="1" bind:value={gridCols} />
				</label>
			{:else if gridMode === 'auto'}
				<label class="pg-control">
					<span class="pg-label">Min track <span class="pg-value">{gridMin} px</span></span>
					<input type="range" min="80" max="200" step="10" bind:value={gridMin} />
				</label>
			{/if}

			<label class="pg-control">
				<span class="pg-label">Gap <span class="pg-value">{gridGap} px</span></span>
				<input type="range" min="0" max="24" step="2" bind:value={gridGap} />
			</label>

			<label class="pg-control">
				<span class="pg-label">Padding <span class="pg-value">{gridPadding} px</span></span>
				<input type="range" min="0" max="24" step="2" bind:value={gridPadding} />
			</label>

			<label class="pg-check">
				<input type="checkbox" bind:checked={gridSpan} />
				Header spans all columns
			</label>
		</div>

		<div class="pg-stage">
			<div class="pg-stage-meta">
				<span>{gridSummary}</span>
				<span>HTML / CSS preview</span>
			</div>
			<div class="pg-frame pg-grid" style={gridFrameStyle}>
				{#if gridSpan}
					<div class="pg-gcell header" style="grid-column: 1 / -1;">header · spans all columns</div>
				{/if}
				{#each gridCards as card}
					<div class="pg-gcell">{card}</div>
				{/each}
			</div>
		</div>
	{:else if tab === 'wrap'}
		<div class="pg-controls">
			<label class="pg-control">
				<span class="pg-label">Gap <span class="pg-value">{wrapGap} px</span></span>
				<input type="range" min="0" max="20" step="2" bind:value={wrapGap} />
			</label>

			<label class="pg-control">
				<span class="pg-label">Card width <span class="pg-value">{wrapCard} px</span></span>
				<input type="range" min="80" max="180" step="10" bind:value={wrapCard} />
			</label>

			<label class="pg-control">
				<span class="pg-label">Cards <span class="pg-value">{wrapCount}</span></span>
				<input type="range" min="4" max="12" step="1" bind:value={wrapCount} />
			</label>
		</div>

		<div class="pg-stage">
			<div class="pg-stage-meta">
				<span>{wrapSummary}</span>
				<span>HTML / CSS preview</span>
			</div>
			<div class="pg-frame pg-wrap-frame" style={`width:${frameWidth}px`}>
				<div class="pg-wrap" style={`gap:${wrapGap}px;`}>
					{#each wrapItems as card}
						<div class="pg-chip" style={`width:${wrapCard}px`}>card {card}</div>
					{/each}
				</div>
			</div>
		</div>
	{:else if tab === 'stack'}
		<div class="pg-controls">
			<label class="pg-control">
				<span class="pg-label">Second child offset <span class="pg-value">{stackOffset} px</span></span>
				<input type="range" min="0" max="90" step="6" bind:value={stackOffset} />
			</label>

			<label class="pg-check">
				<input type="checkbox" bind:checked={stackFront} />
				Child 2 paints on top
			</label>
		</div>

		<div class="pg-stage">
			<div class="pg-stage-meta">
				<span>{stackSummary}</span>
				<span>HTML / CSS preview</span>
			</div>
			<div class="pg-frame pg-stack" style={`width:${frameWidth}px`}>
				{#if stackFront}
					<div class="pg-stack-card a" style="left:24px;top:48px;">
						<span class="stack-label">child 1 · panel</span>
						<span class="stack-meta">fills the shared cell</span>
					</div>
					<div class="pg-stack-card b" style={`left:${24 + stackOffset}px;top:${48 + stackOffset}px;`}>
						<span class="stack-label">child 2 · controls</span>
						<span class="stack-meta">painted last → above</span>
						<span class="stack-badge">on top</span>
					</div>
				{:else}
					<div class="pg-stack-card b" style={`left:${24 + stackOffset}px;top:${48 + stackOffset}px;`}>
						<span class="stack-label">child 2 · controls</span>
						<span class="stack-meta">painted first → below</span>
					</div>
					<div class="pg-stack-card a" style="left:24px;top:48px;">
						<span class="stack-label">child 1 · panel</span>
						<span class="stack-meta">painted last → above</span>
						<span class="stack-badge">on top</span>
					</div>
				{/if}
			</div>
		</div>
	{:else}
		<div class="pg-controls">
			<div class="pg-control">
				<span class="pg-label">Edges</span>
				<div class="pg-checks">
					<label class="pg-check"><input type="checkbox" bind:checked={dockTop} /> Top</label>
					<label class="pg-check"><input type="checkbox" bind:checked={dockLeft} /> Left</label>
					<label class="pg-check"><input type="checkbox" bind:checked={dockBottom} /> Bottom</label>
				</div>
			</div>

			<label class="pg-control">
				<span class="pg-label">Gap <span class="pg-value">{dockGap} px</span></span>
				<input type="range" min="0" max="12" step="1" bind:value={dockGap} />
			</label>

			<label class="pg-control">
				<span class="pg-label">Padding <span class="pg-value">{dockPadding} px</span></span>
				<input type="range" min="0" max="16" step="2" bind:value={dockPadding} />
			</label>
		</div>

		<div class="pg-stage">
			<div class="pg-stage-meta">
				<span>{dockSummary}</span>
				<span>HTML / CSS preview</span>
			</div>
			<div
				class="pg-frame pg-dock"
				style={`width:${frameWidth}px;padding:${dockPadding}px;gap:${dockGap}px;`}>
				{#if dockTop}
					<div class="dock-top">toolbar · 40 px</div>
				{/if}
				<div class="dock-middle" style={`gap:${dockGap}px;`}>
					{#if dockLeft}
						<div class="dock-left">explorer · 150 px</div>
					{/if}
					<div class="dock-fill">
						<span>editor</span>
						<span class="dock-fill-meta">fill · takes what is left</span>
					</div>
				</div>
				{#if dockBottom}
					<div class="dock-bottom">status · 26 px</div>
				{/if}
			</div>
		</div>
	{/if}

	<div class="pg-code-wrap">
		<div class="pg-code-head">
			<span>Generated for this preview</span>
			<span>CSS + Rust</span>
		</div>
		<pre class="pg-code">{tab === 'flex'
				? flexSnippet
				: tab === 'grid'
					? gridSnippet
					: tab === 'wrap'
						? wrapSnippet
						: tab === 'stack'
							? stackSnippet
							: dockSnippet}</pre>
	</div>

	<p class="pg-foot">
		The visual is an HTML/CSS approximation of the same view tree. SequoiaUI paints the real layout
		natively through wgpu; the Rust block shows the builder calls that produce it.
	</p>
</div>

<style>
	.playground {
		margin: var(--space-5) 0 var(--space-4);
		border: 1px solid #e5e5e7;
		border-radius: 14px;
		overflow: hidden;
		background: #fff;
	}

	.pg-tabs {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
		padding: 8px 10px;
		background: #fafafa;
		border-bottom: 1px solid #e5e5e7;
	}
	.pg-tabs button {
		font: inherit;
		font-size: var(--text-xs);
		font-weight: 600;
		color: #555;
		background: transparent;
		border: 1px solid transparent;
		border-radius: 8px;
		padding: 6px 12px;
		cursor: pointer;
		transition:
			background 0.15s ease,
			color 0.15s ease;
	}
	.pg-tabs button:hover {
		background: #efeff1;
		color: #1a1a1a;
	}
	.pg-tabs button.active {
		background: #fff;
		border-color: #e0e0e3;
		color: #1a1a1a;
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
	}

	.pg-width {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 10px;
		padding: 10px 14px;
		border-bottom: 1px solid #eee;
		background: #fff;
	}
	.pg-width label {
		font-size: var(--text-2xs);
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: #888;
	}
	.pg-width input[type='range'] {
		flex: 1;
		min-width: 120px;
		max-width: 260px;
		accent-color: var(--accent);
	}
	.pg-value {
		font-family: var(--font-mono);
		font-size: 10px;
		font-weight: 600;
		color: var(--accent);
		white-space: nowrap;
	}
	.pg-presets {
		display: flex;
		gap: 4px;
		margin-left: auto;
	}
	.pg-presets button {
		font: inherit;
		font-size: 10px;
		font-weight: 600;
		color: #777;
		background: #f5f5f7;
		border: 1px solid #e5e5e7;
		border-radius: 20px;
		padding: 3px 9px;
		cursor: pointer;
	}
	.pg-presets button:hover {
		color: #1a1a1a;
		border-color: #d3d3d7;
	}

	.pg-controls {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
		gap: 12px 16px;
		padding: 12px 14px;
		border-bottom: 1px solid #eee;
		align-items: end;
	}
	.pg-control {
		display: flex;
		flex-direction: column;
		gap: 6px;
		min-width: 0;
	}
	.pg-label {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		font-size: var(--text-2xs);
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: #888;
	}
	.pg-control input[type='range'] {
		width: 100%;
		accent-color: var(--accent);
	}
	.pg-control select {
		font-family: var(--font-sans);
		font-size: var(--text-xs);
		color: #333;
		background: #fff;
		border: 1px solid #e0e0e3;
		border-radius: 7px;
		padding: 6px 8px;
	}
	.pg-seg {
		display: inline-flex;
		flex-wrap: wrap;
		gap: 2px;
		background: #f0f0f2;
		border-radius: 8px;
		padding: 2px;
	}
	.pg-seg button {
		font: inherit;
		font-size: 10px;
		font-weight: 600;
		color: #666;
		background: transparent;
		border: none;
		border-radius: 6px;
		padding: 5px 9px;
		cursor: pointer;
		white-space: nowrap;
	}
	.pg-seg button:hover {
		color: #1a1a1a;
	}
	.pg-seg button.active {
		background: #fff;
		color: #1a1a1a;
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
	}
	.pg-checks {
		display: flex;
		flex-wrap: wrap;
		gap: 8px 14px;
	}
	.pg-check {
		display: flex;
		align-items: center;
		gap: 7px;
		font-size: var(--text-xs);
		font-weight: 500;
		color: #555;
		cursor: pointer;
	}
	.pg-check input {
		accent-color: var(--accent);
		margin: 0;
	}

	.pg-stage {
		padding: 18px;
		background: #f5f5f7;
		overflow-x: auto;
	}
	.pg-stage-meta {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		max-width: 700px;
		margin: 0 auto 10px;
		font-size: 10px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: #999;
	}
	.pg-stage-meta span:last-child {
		white-space: nowrap;
		color: #aaa;
	}

	.pg-frame {
		flex-shrink: 0;
		margin: 0 auto;
		box-sizing: border-box;
		background: #fff;
		border: 1px solid #dcdcdf;
		border-radius: 10px;
		box-shadow: 0 12px 28px -22px rgba(0, 0, 0, 0.45);
		overflow: hidden;
	}
	.pg-flex {
		height: 280px;
		display: flex;
		overflow: hidden;
	}
	.pg-cell {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 2px;
		min-height: 84px;
		padding: 8px;
		border: 1px solid #dfe7ec;
		border-radius: 8px;
		background: #f4f9fc;
		min-width: 0;
	}
	.pg-cell.sidebar {
		background: #eef6fa;
	}
	.pg-cell.content {
		background: #fff8f5;
		border-color: #f0d6cc;
	}
	.pg-cell.aside {
		background: #f7f7f9;
		border-color: #e3e3e7;
	}
	.cell-name {
		font-size: 12px;
		font-weight: 700;
		color: #1a1a1a;
	}
	.cell-meta {
		font-size: 10px;
		line-height: 1.35;
		color: #7a8b93;
		text-align: center;
	}

	.pg-grid {
		height: 300px;
		display: grid;
		align-content: stretch;
		grid-auto-rows: minmax(36px, 1fr);
	}
	.pg-gcell {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 0;
		border: 1px solid #dfe7ec;
		border-radius: 8px;
		background: #f4f9fc;
		font-size: 12px;
		font-weight: 700;
		color: #3c5a66;
	}
	.pg-gcell:nth-child(3n) {
		background: #fff8f5;
		border-color: #f0d6cc;
		color: #8a4b39;
	}
	.pg-gcell.header {
		background: rgba(230, 57, 30, 0.08);
		border-color: rgba(230, 57, 30, 0.24);
		color: #e6391e;
		font-size: 9px;
		letter-spacing: 0.05em;
		text-transform: uppercase;
	}

	.pg-wrap-frame {
		height: 280px;
	}
	.pg-wrap {
		display: flex;
		flex-wrap: wrap;
		align-content: flex-start;
		height: 100%;
		padding: 16px;
		box-sizing: border-box;
	}
	.pg-chip {
		height: 46px;
		display: flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		border: 1px solid #dfe7ec;
		border-radius: 8px;
		background: #f4f9fc;
		font-size: 11px;
		font-weight: 700;
		color: #3c5a66;
	}
	.pg-chip:nth-child(3n) {
		background: #fff8f5;
		border-color: #f0d6cc;
		color: #8a4b39;
	}

	.pg-stack {
		position: relative;
		height: 300px;
	}
	.pg-stack-card {
		position: absolute;
		width: 230px;
		height: 130px;
		box-sizing: border-box;
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 14px;
		border-radius: 10px;
		box-shadow: 0 16px 34px -18px rgba(0, 0, 0, 0.45);
	}
	.pg-stack-card.a {
		background: #eef6fa;
		border: 1px solid #cfe2ec;
	}
	.pg-stack-card.b {
		background: #1a1a1a;
		border: 1px solid #333;
		color: #eee;
	}
	.stack-label {
		font-size: 12px;
		font-weight: 700;
	}
	.stack-meta {
		font-size: 10px;
		line-height: 1.4;
		opacity: 0.6;
	}
	.stack-badge {
		align-self: flex-start;
		margin-top: auto;
		font-size: 9px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		background: #e6391e;
		color: #fff;
		padding: 2px 7px;
		border-radius: 20px;
	}

	.pg-dock {
		height: 320px;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.dock-top {
		flex: 0 0 40px;
		display: flex;
		align-items: center;
		padding: 0 12px;
		border: 1px solid #d6e6ef;
		border-radius: 6px;
		background: #eef6fa;
		font-size: 11px;
		font-weight: 600;
		color: #3c5a66;
	}
	.dock-middle {
		display: flex;
		flex: 1;
		min-height: 0;
	}
	.dock-left {
		flex: 0 0 150px;
		padding: 10px;
		box-sizing: border-box;
		border: 1px solid #d6e6ef;
		border-radius: 6px;
		background: #f4f9fc;
		font-size: 11px;
		font-weight: 600;
		color: #3c5a66;
	}
	.dock-fill {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 4px;
		border: 1px dashed #c9c9cf;
		border-radius: 6px;
		background: #fff;
		font-size: 12px;
		font-weight: 700;
		color: #666;
	}
	.dock-fill-meta {
		font-size: 10px;
		font-weight: 500;
		color: #999;
	}
	.dock-bottom {
		flex: 0 0 26px;
		display: flex;
		align-items: center;
		padding: 0 10px;
		border: 1px solid #d6e6ef;
		border-radius: 6px;
		background: #f4f9fc;
		font-size: 10px;
		font-weight: 600;
		color: #3c5a66;
	}

	.pg-code-wrap {
		background: #1a1a1a;
	}
	.pg-code-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 8px 14px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.07);
		font-size: 10px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: #888;
	}
	.pg-code {
		margin: 0;
		padding: 14px;
		overflow-x: auto;
		font-family: var(--font-mono);
		font-size: 11px;
		line-height: 1.7;
		color: #e5e5e7;
		white-space: pre;
	}

	.pg-foot {
		margin: 0;
		padding: 10px 14px;
		border-top: 1px solid #e5e5e7;
		background: #fafafa;
		font-size: var(--text-2xs);
		line-height: 1.55;
		color: #888;
	}

	@media (max-width: 640px) {
		.pg-controls {
			grid-template-columns: 1fr;
		}
		.pg-presets {
			margin-left: 0;
		}
		.pg-stage {
			padding: 12px;
		}
	}
</style>
