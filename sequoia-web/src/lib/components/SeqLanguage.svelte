<script>
	import { softReveal } from '../actions/softReveal';
	import CodeBlock from './CodeBlock.svelte';

	const seqCode = `<span class="kw">app</span> Counter &#123;
  title: <span class="str">"Counter"</span>
  size: (<span class="num">480</span>, <span class="num">360</span>)
  theme: dark

  <span class="kw">prop</span> count_label: <span class="ty">String</span>
  <span class="kw">on</span> add()

  <span class="kw">window</span> Main &#123;
    Column(padding: <span class="num">16</span>, gap: <span class="num">12</span>) &#123;
      Text(<span class="str">"Counter .seq"</span>, size: <span class="num">20</span>, color: theme.ink, bold: <span class="kw">true</span>)
      Text(dynamic: count_label, size: <span class="num">14</span>, color: theme.accent)
      Button(<span class="str">"Add"</span>, on_click: &#123; add() &#125;)
    &#125;
  &#125;
&#125;`;

	const rustCode = `<span class="kw">use</span> sequoia_ui::prelude::*;

<span class="kw">let</span> label = Signal::new(<span class="ty">String</span>::from(<span class="str">"Kliknięto 0 razy"</span>));
<span class="kw">let</span> c_add = label.clone();

<span class="kw">let</span> bindings = HostBindings::new()
    .prop_str(<span class="str">"count_label"</span>, label)
    .on(<span class="str">"add"</span>, <span class="kw">move</span> || &#123;
        c_add.set(<span class="str">"Kliknięto!"</span>.into());
    &#125;);

<span class="kw">let</span> loaded = SeqLoader::load_with(
    <span class="str">"Counter.seq"</span>, &amp;bindings
)?;`;
</script>

<section class="section" use:softReveal>
	<div class="seq-header">
		<span class="section-number">05</span>
		<p class="section-label">Experimental format</p>
		<h2 class="section-title">Declare the UI, keep the logic in Rust</h2>
		<p class="section-description">
			<span class="highlight">.seq</span> is an optional, experimental declarative format available
			through SeqLoader. It describes windows, components and shapes; state and callbacks stay in
			Rust, and a host callback without a matching binding fails at load time. The Rust API remains
			the supported path.
		</p>
		<div class="seq-badges">
			<span>Optional &amp; experimental</span>
			<span>Typed host bindings</span>
			<span>Preview with --watch</span>
		</div>
	</div>

	<div class="seq-demo">
		<div class="seq-pane">
			<CodeBlock lang="Seq" code={seqCode} />
		</div>
		<div class="seq-pane">
			<CodeBlock lang="Rust" code={rustCode} />
		</div>
	</div>
</section>

<style>
	.seq-badges {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin-top: var(--space-2);
	}
	.seq-badges span {
		font-size: var(--text-xs);
		font-weight: 600;
		color: #555;
		background: #fff;
		border: 1px solid var(--border);
		padding: var(--space-2) var(--space-3);
		border-radius: 20px;
		letter-spacing: 0.01em;
	}

	.seq-demo {
		margin-top: var(--section-gap);
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		gap: var(--space-5);
		align-items: stretch;
	}

	.seq-pane {
		min-width: 0;
		display: flex;
	}

	.seq-pane :global(.code-block-wrapper) {
		margin: 0;
		flex: 1;
	}

	@media (max-width: 900px) {
		.seq-demo {
			grid-template-columns: 1fr;
		}
	}
</style>