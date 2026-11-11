<script>
	import CodeBlock from './CodeBlock.svelte';

	let active = $state(0);
	let count = $state(0);

	const examples = [
		{
			label: 'Toolbar & sidebar',
			window: 'Dock layout',
			code: `<span class="kw">let</span> toolbar = Row::new()
    .gap(<span class="num">8.0</span>)
    .padding(<span class="num">12.0</span>)
    .child(Button::new(<span class="str">"Open"</span>))
    .child(Spacer::flex(<span class="num">1.0</span>))
    .child(Text::new(<span class="str">"Ready"</span>));

<span class="kw">let</span> body = Row::new()
    .fill_size()
    .child(
        Column::new()
            .width(<span class="num">220.0</span>)
            .gap(<span class="num">8.0</span>)
            .padding(<span class="num">16.0</span>)
            .child(Text::new(<span class="str">"Layers"</span>).bold())
            .child(Text::new(<span class="str">"Background"</span>))
            .child(Text::new(<span class="str">"Card"</span>)),
    )
    .child(Container::new().fill_size().padding(<span class="num">16.0</span>).child(Text::new(<span class="str">"Canvas"</span>)));

<span class="kw">let</span> page = Column::new().fill_size().child(toolbar).child(body);`
		},
		{
			label: 'Reactive counter',
			window: 'Counter',
			code: `<span class="kw">let</span> count = Signal::new(<span class="num">0i32</span>);
<span class="kw">let</span> shown = count.clone();

App::new()
    .title(<span class="str">"Counter"</span>)
    .size(<span class="num">420.0</span>, <span class="num">280.0</span>)
    .root(
        Column::new()
            .center()
            .gap(<span class="num">16.0</span>)
            .fill_size()
            .padding(<span class="num">24.0</span>)
            .child(
                Text::with(<span class="kw">move</span> || format!(<span class="str">"Count: &#123;&#125;"</span>, shown.get()))
                    .size(<span class="num">32.0</span>)
                    .bold(),
            )
            .child(
                Button::new(<span class="str">"Increment"</span>)
                    .on_click(<span class="kw">move</span> || count.update(|n| *n += <span class="num">1</span>)),
            ),
    )
    .run()
    .expect(<span class="str">"event loop failed"</span>);`
		}
	];
</script>

<div class="preview-shell">
	<div class="preview-bar">
		<div class="tabs" role="group" aria-label="Layout examples">
			{#each examples as example, i}
				<button
					type="button"
					class="tab"
					class:active={active === i}
					aria-pressed={active === i}
					onclick={() => (active = i)}>{example.label}</button
				>
			{/each}
		</div>
		<span class="html-note">HTML / CSS preview</span>
	</div>

	<div class="preview-grid">
		<div class="preview-code">
			<CodeBlock lang="Rust" code={examples[active].code} />
		</div>

		<div class="preview-visual">
			<div class="mock-window">
				<div class="mock-title">
					<span class="mock-dot"></span>
					<span class="mock-dot"></span>
					<span class="mock-dot"></span>
					<span class="mock-title-text">{examples[active].window}</span>
				</div>

				{#if active === 0}
					<div class="mock-app">
						<div class="mock-toolbar">
							<button type="button" class="mock-button">Open</button>
							<span class="mock-grow"></span>
							<span class="mock-muted">Ready</span>
						</div>
						<div class="mock-main">
							<div class="mock-sidebar">
								<strong>Layers</strong>
								<span>Background</span>
								<span>Card</span>
							</div>
							<div class="mock-canvas">Canvas</div>
						</div>
					</div>
				{:else}
					<div class="mock-counter">
						<div class="mock-count">Count: {count}</div>
						<button type="button" class="mock-button primary" onclick={() => count++}>
							Increment
						</button>
					</div>
				{/if}
			</div>
		</div>
	</div>

	<p class="preview-caption">
		The right side is an HTML/CSS approximation of the same view tree. SequoiaUI paints the real UI
		natively through wgpu.
	</p>
</div>

<style>
	.preview-shell {
		margin: 28px 0 8px;
		border: 1px solid #e5e5e7;
		border-radius: 14px;
		overflow: hidden;
		background: #fff;
	}

	.preview-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 10px 14px;
		background: #fafafa;
		border-bottom: 1px solid #e5e5e7;
	}

	.tabs {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.tab {
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
	.tab:hover {
		background: #efeff1;
		color: #1a1a1a;
	}
	.tab.active {
		background: #fff;
		border-color: #e0e0e3;
		color: #1a1a1a;
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
	}

	.html-note {
		font-size: var(--text-2xs);
		font-weight: 700;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: #999;
		white-space: nowrap;
	}

	.preview-grid {
		display: grid;
		grid-template-columns: minmax(0, 1.15fr) minmax(0, 1fr);
		align-items: stretch;
	}

	.preview-code {
		min-width: 0;
		border-right: 1px solid #e5e5e7;
	}
	.preview-code :global(.code-block-wrapper) {
		margin: 0;
		border-radius: 0;
		height: 100%;
	}
	.preview-code :global(.code-block) {
		padding: 16px;
	}
	.preview-code :global(.code-block pre) {
		font-size: var(--text-xs);
	}

	.preview-visual {
		min-width: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 20px;
		background: #f4f4f6;
	}

	.mock-window {
		width: 100%;
		max-width: 420px;
		border-radius: 10px;
		overflow: hidden;
		background: #fff;
		border: 1px solid #dcdcdf;
		box-shadow: 0 18px 40px -24px rgba(0, 0, 0, 0.45);
	}

	.mock-title {
		display: flex;
		align-items: center;
		gap: 6px;
		height: 32px;
		padding: 0 12px;
		background: #ececee;
		border-bottom: 1px solid #dcdcdf;
	}
	.mock-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #d0d0d4;
	}
	.mock-title-text {
		margin-left: auto;
		font-size: var(--text-2xs);
		color: #777;
	}

	.mock-app {
		display: flex;
		flex-direction: column;
		min-height: 250px;
	}
	.mock-toolbar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 12px;
		border-bottom: 1px solid #eee;
	}
	.mock-button {
		font: inherit;
		font-size: var(--text-2xs);
		font-weight: 600;
		padding: 5px 10px;
		border-radius: 6px;
		border: 1px solid #d5d5d8;
		background: #fff;
		color: #1a1a1a;
		cursor: default;
	}
	.mock-button.primary {
		background: #e6391e;
		border-color: #e6391e;
		color: #fff;
		cursor: pointer;
	}
	.mock-button.primary:hover {
		background: #c9301a;
		border-color: #c9301a;
	}
	.mock-grow {
		flex: 1;
	}
	.mock-muted {
		font-size: var(--text-2xs);
		color: #888;
	}

	.mock-main {
		display: flex;
		flex: 1;
	}
	.mock-sidebar {
		width: 34%;
		min-width: 110px;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-4);
		background: #fafafa;
		border-right: 1px solid #eee;
		font-size: var(--text-xs);
		color: #555;
	}
	.mock-sidebar strong {
		font-size: var(--text-2xs);
		letter-spacing: 0.04em;
		text-transform: uppercase;
		color: #1a1a1a;
	}
	.mock-canvas {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: var(--text-2xs);
		color: #aaa;
		background:
			linear-gradient(#f0f0f2 1px, transparent 1px) 0 0 / 24px 24px,
			linear-gradient(90deg, #f0f0f2 1px, transparent 1px) 0 0 / 24px 24px, #fff;
	}

	.mock-counter {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 16px;
		padding: 24px;
		min-height: 250px;
		background: #fff;
	}
	.mock-count {
		font-size: 26px;
		font-weight: 800;
		letter-spacing: -0.02em;
		color: #1a1a1a;
		font-variant-numeric: tabular-nums;
	}

	.preview-caption {
		margin: 0;
		padding: var(--space-3) var(--space-4);
		font-size: var(--text-xs);
		line-height: 1.5;
		color: #777;
		background: #fafafa;
		border-top: 1px solid #e5e5e7;
	}

	@media (max-width: 860px) {
		.preview-grid {
			grid-template-columns: 1fr;
		}
		.preview-code {
			border-right: 0;
			border-bottom: 1px solid #e5e5e7;
		}
		.html-note {
			display: none;
		}
	}
</style>
