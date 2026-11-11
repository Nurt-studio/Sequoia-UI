<script>
    import { softReveal } from '../actions/softReveal'
</script>
<section class="section" use:softReveal>
	<div>
		<span class="section-number">04</span>
		<p class="section-label">Quick Start</p>
		<h2 class="section-title">One crate, a builder, a view tree</h2>
		<p class="section-description">
			No macros and no separate store: the window is built with App, and the UI is a tree of
			views that read signals.
		</p>
	</div>
	<div class="code-container">
		<div class="code-header">
			<div class="code-dot red"></div>
			<div class="code-dot yellow"></div>
			<div class="code-dot green"></div>
		</div>
		<pre class="code-block"><span class="keyword">use</span> sequoia_ui::prelude::*;

<span class="keyword">fn</span> <span class="function">main</span>() &#123;
    <span class="keyword">let</span> count = Signal::new(<span class="number">0i32</span>);
    <span class="keyword">let</span> display = count.clone();

    App::new()
        .title(<span class="string">"Counter"</span>)
        .size(<span class="number">480.0</span>, <span class="number">320.0</span>)
        .root(
            Column::new()
                .gap(<span class="number">12.0</span>)
                .padding(<span class="number">24.0</span>)
                .child(Text::with(<span class="keyword">move</span> || format!(<span class="string">"Count: &#123;&#125;"</span>, display.get())))
                .child(
                    Button::new(<span class="string">"Increment"</span>)
                        .on_click(<span class="keyword">move</span> || count.update(|n| *n += <span class="number">1</span>)),
                ),
        )
        .run()
        .expect(<span class="string">"failed to run the app"</span>);
&#125;</pre>
	</div>
</section>

<style>
	.code-container {
		background: #1a1a1a;
		border-radius: 12px;
		padding: var(--space-6);
		margin-top: var(--section-gap);
		overflow-x: auto;
		max-width: 700px;
	}

	.code-header {
		display: flex;
		gap: var(--space-2);
		margin-bottom: var(--space-5);
	}

	.code-dot {
		width: 12px;
		height: 12px;
		border-radius: 50%;
	}
	.code-dot.red {
		background: #ff5f56;
	}
	.code-dot.yellow {
		background: #ffbd2e;
	}
	.code-dot.green {
		background: #27c93f;
	}

	.code-block {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		line-height: var(--leading-relaxed);
		color: #e5e5e7;
		margin: 0;
	}

	.code-block :global(.keyword) {
		color: #e6391e;
		font-weight: 600;
	}
	.code-block :global(.function) {
		color: #7ec8e3;
	}
	.code-block :global(.string) {
		color: #98c379;
	}
	.code-block :global(.comment) {
		color: #6a6a6a;
		font-style: italic;
	}
	.code-block :global(.number) {
		color: #d19a66;
	}
</style>