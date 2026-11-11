<script>
	import DocsTopBar from '../components/DocsTopBar.svelte';
	import DocsFooter from '../components/DocsFooter.svelte';
	import CodeBlock from '../components/CodeBlock.svelte';
	import LayoutPreview from '../components/LayoutPreview.svelte';

	const BOOK = 'https://nurt-studio.github.io/Sequoia-UI/';

	const sidebarGroups = [
		{
			title: 'Getting Started',
			links: [
				{ label: 'Introduction', href: '#why-sequoia', active: true },
				{ label: 'Installation', href: '#installation' },
				{ label: 'Your first app', href: '#first-app' },
				{ label: 'Runnable examples', href: '#examples' }
			]
		},
		{
			title: 'Building UI',
			links: [
				{ label: 'Layout', href: '#layout' },
				{ label: 'Widgets and state', href: '#widgets' },
				{ label: 'Reactivity', href: '#reactive-state' },
				{ label: 'Styling', href: '#styling' }
			]
		},
		{
			title: 'Reference',
			links: [
				{ label: 'API reference', href: '#api-reference' },
				{ label: 'Full book', href: BOOK },
				{ label: 'Widget guide', href: `${BOOK}guides/widgets.html` }
			]
		},
		{
			title: 'Advanced',
			links: [
				{ label: 'Custom shaders', href: `${BOOK}guides/shaders.html` },
				{ label: '.seq format (experimental)', href: `${BOOK}guides/seq.html` },
				{ label: 'Docking', href: `${BOOK}guides/docking.html` },
				{ label: 'Testing', href: `${BOOK}guides/testing.html` }
			]
		}
	];

	const tocLinks = [
		{ id: 'why-sequoia', label: 'Why SequoiaUI?' },
		{ id: 'installation', label: 'Installation' },
		{ id: 'first-app', label: 'Your first app' },
		{ id: 'layout', label: 'Layout' },
		{ id: 'widgets', label: 'Widgets and state' },
		{ id: 'reactive-state', label: 'Reactivity', sub: true },
		{ id: 'styling', label: 'Styling', sub: true },
		{ id: 'examples', label: 'Runnable examples' },
		{ id: 'api-reference', label: 'API reference' }
	];

	let activeToc = $state(0);

	const installHtml = `<span class="cm"># Cargo.toml</span>
[dependencies]
sequoia-ui = <span class="str">"0.1.4"</span>`;

	const cargoHtml = `cargo new my-app
<span class="kw">cd</span> my-app
cargo add sequoia-ui
cargo run`;

	const rustHtml = `<span class="kw">use</span> sequoia_ui::prelude::*;

<span class="kw">fn</span> <span class="fn">main</span>() &#123;
    <span class="kw">let</span> count = Signal::new(<span class="num">0i32</span>);
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
        .expect(<span class="str">"event loop failed"</span>);
&#125;`;

	const widgetsHtml = `<span class="kw">let</span> email = Signal::new(String::new());
<span class="kw">let</span> notify = Signal::new(<span class="kw">true</span>);
<span class="kw">let</span> volume = Signal::new(<span class="num">40.0</span>);

<span class="kw">let</span> form = Column::new()
    .gap(<span class="num">12.0</span>)
    .padding(<span class="num">24.0</span>)
    .child(Field::new(email, <span class="str">"Email"</span>).placeholder(<span class="str">"you@example.com"</span>))
    .child(Checkbox::new(notify).label(<span class="str">"Notify on save"</span>))
    .child(Slider::new(volume).min(<span class="num">0.0</span>).max(<span class="num">100.0</span>).width(<span class="num">200.0</span>))
    .child(Button::new(<span class="str">"Save"</span>).style(ButtonStyle::PRIMARY));`;

	const styleHtml = `<span class="kw">let</span> save = Button::new(<span class="str">"Save"</span>)
    .style(ButtonStyle::PRIMARY)
    .on_click(|| &#123; <span class="cm">/* persist */</span> &#125;);

<span class="kw">let</span> warning = Text::new(<span class="str">"Unsaved changes"</span>)
    .size(<span class="num">13.0</span>)
    .color(Color::hex(<span class="num">0xE6391E</span>));`;

	const examplesHtml = `cargo run -p sequoia-ui --example showcase
cargo run -p sequoia-ui --example form
cargo run -p sequoia-ui --example todo
cargo run -p sequoia-ui --example tabs
cargo run -p sequoia-ui --example seq_hello
cargo run -p vision-design`;
</script>

<DocsTopBar />

<div class="docs-layout">
	<aside class="sidebar">
		{#each sidebarGroups as group}
			<div class="sidebar-group">
				<div class="sidebar-group-title">{group.title}</div>
				{#each group.links as link}
					<a
						href={link.href}
						class="sidebar-link"
						class:active={link.active}
						target={link.href.startsWith('http') ? '_blank' : undefined}
						rel={link.href.startsWith('http') ? 'noreferrer' : undefined}>{link.label}</a
					>
				{/each}
			</div>
		{/each}
	</aside>

	<main class="docs-main">
		<div class="breadcrumbs">
			<a href={BOOK} target="_blank" rel="noreferrer">Docs</a>
			<span class="sep">/</span>
			<a href={`${BOOK}getting-started/installation.html`} target="_blank" rel="noreferrer">Getting Started</a>
			<span class="sep">/</span>
			<span class="current">Introduction</span>
		</div>

		<h1>Introduction</h1>
		<p class="subtitle">
			SequoiaUI is a native, reactive GUI framework for Rust built on wgpu. It uses a retained view tree
			and fine-grained signals, with no virtual DOM and no embedded web view.
		</p>

		<div class="callout callout-info">
			<span class="callout-icon">ℹ️</span>
			<div>
				<strong>New to SequoiaUI?</strong>
				Start with the <a href="#first-app">Your first app</a> section below. When you need the full
				reference, the <a href={BOOK} target="_blank" rel="noreferrer">book</a> has a chapter for every subsystem.
			</div>
		</div>

		<h2 id="why-sequoia">Why SequoiaUI?</h2>
		<p>
			SequoiaUI does not wrap an existing C/C++ toolkit and does not run inside a web view. Views are
			retained in a tree, signals update only the views that read them, and everything is painted
			through <a href="https://wgpu.rs" target="_blank" rel="noreferrer">wgpu</a> on a native window.
		</p>

		<ul>
			<li><strong>Native rendering</strong> — wgpu on Metal, Vulkan, DX12 or OpenGL</li>
			<li><strong>Memory safe</strong> — owned Rust values, no garbage collector; the workspace forbids unsafe code</li>
			<li><strong>Declarative API</strong> — describe your UI as a tree of views</li>
			<li><strong>Widget library</strong> — buttons, text fields, sliders, tables, trees, tabs, docks and overlays</li>
			<li><strong>Desktop platforms</strong> — Windows, macOS and Linux; there is no web build</li>
			<li><strong>One dependency</strong> — <span class="inline-code">sequoia-ui</span> re-exports the whole framework</li>
		</ul>

		<h2 id="installation">Installation</h2>
		<p>Add the facade crate to your project using Cargo:</p>

		<CodeBlock lang="Terminal" code={installHtml} />

		<p>Or create a new binary crate:</p>

		<CodeBlock lang="Terminal" code={cargoHtml} />

		<p>
			Requires Rust 1.88 or newer. The framework is distributed as the
			<span class="inline-code">sequoia-ui</span> facade crate; the internal crates in the repository are
			not meant to be added by hand.
		</p>

		<h2 id="first-app">Your first app</h2>
		<p>
			A complete program: a signal holds the count, a label reads it, and a button updates it. Nothing
			else is needed to make the UI refresh.
		</p>

		<CodeBlock lang="Rust" code={rustHtml} />

		<ul>
			<li><span class="inline-code">Signal::new(0i32)</span> creates the reactive value; <span class="inline-code">.clone()</span> shares it with the label.</li>
			<li><span class="inline-code">Text::with(...)</span> takes a closure, so reading <span class="inline-code">shown.get()</span> subscribes the label.</li>
			<li><span class="inline-code">Button::on_click(...)</span> runs on the UI thread and updates the signal.</li>
			<li><span class="inline-code">.run()</span> returns a <span class="inline-code">Result</span>, so <span class="inline-code">.expect(...)</span> surfaces startup errors.</li>
		</ul>

		<div class="callout callout-tip">
			<span class="callout-icon">💡</span>
			<div>
				<strong>Tip</strong>
				The <span class="inline-code">.fill_size()</span> builder makes a view expand to fill the space
				its parent gives it.
			</div>
		</div>

		<h2 id="layout">Layout</h2>
		<p>
			<strong>Row</strong> and <strong>Column</strong> handle linear flow, with <strong>Grid</strong>,
			<strong>Stack</strong>, <strong>SplitPanel</strong> and <strong>DockArea</strong> for the rest.
			<strong>Spacer::flex(1.0)</strong> pushes siblings apart. Switch between the examples to compare
			the code with a visual approximation of the result.
		</p>

		<LayoutPreview />

		<h2 id="widgets">Widgets and state</h2>
		<p>
			Built-in widgets bind directly to signals. The runtime handles focus, keyboard activation and
			accessibility; you keep the state.
		</p>

		<CodeBlock lang="Rust" code={widgetsHtml} />

		<h2 id="reactive-state">Reactivity</h2>
		<p>
			Every UI element is a <strong>View</strong>: a composable, reusable building block. Views form a
			retained tree that mirrors your UI hierarchy. Reading a signal inside a view subscribes that view
			to it; when the value changes, only the subscribers are scheduled for another frame — there is no
			diff step and no virtual DOM.
		</p>
		<p>
			Views are configured with <strong>builder methods</strong> rather than long constructor
			arguments. Chain <span class="inline-code">.size()</span>,
			<span class="inline-code">.bold()</span> and <span class="inline-code">.padding()</span> to
			customize any view.
		</p>

		<h2 id="styling">Styling</h2>
		<p>
			A theme supplies colors, typography, radii and shadows; widgets ship with typed style presets and
			per-property builders. Override one detail without defining a new style.
		</p>

		<CodeBlock lang="Rust" code={styleHtml} />

		<h2 id="examples">Runnable examples</h2>
		<p>
			The repository ships examples that need no extra setup. Clone it and run them from the workspace
			root:
		</p>

		<CodeBlock lang="Terminal" code={examplesHtml} />

		<p>
			<strong>showcase</strong> is a tour of the widget library, <strong>form</strong> and
			<strong>todo</strong> are small complete apps, and <strong>vision-design</strong> is the editor in
			the screenshot on the home page.
		</p>

		<h2 id="api-reference">API Reference</h2>
		<p>Here's a quick overview of the most commonly used types:</p>

		<table class="docs-table">
			<thead>
				<tr>
					<th>Type</th>
					<th>Description</th>
					<th>Module</th>
				</tr>
			</thead>
			<tbody>
				<tr>
					<td><span class="inline-code">App</span></td>
					<td>Root application instance</td>
					<td><span class="inline-code">sequoia_ui::platform</span></td>
				</tr>
				<tr>
					<td><span class="inline-code">Column</span></td>
					<td>Vertical layout container</td>
					<td><span class="inline-code">sequoia_ui::view</span></td>
				</tr>
				<tr>
					<td><span class="inline-code">Row</span></td>
					<td>Horizontal layout container</td>
					<td><span class="inline-code">sequoia_ui::view</span></td>
				</tr>
				<tr>
					<td><span class="inline-code">Button</span></td>
					<td>Clickable button component</td>
					<td><span class="inline-code">sequoia_ui::elements</span></td>
				</tr>
				<tr>
					<td><span class="inline-code">Text</span></td>
					<td>Text label component</td>
					<td><span class="inline-code">sequoia_ui::view</span></td>
				</tr>
				<tr>
					<td><span class="inline-code">Signal&lt;T&gt;</span></td>
					<td>Reactive state wrapper</td>
					<td><span class="inline-code">sequoia_ui::reactive</span></td>
				</tr>
			</tbody>
		</table>

		<p>
			Every type above is re-exported by
			<span class="inline-code">sequoia_ui::prelude</span>.
		</p>

		<div class="callout callout-warning">
			<span class="callout-icon">⚠️</span>
			<div>
				<strong>Pre-1.0</strong>
				This book targets SequoiaUI 0.1.x. APIs may still change between minor releases; every example is
				written for the version in the repository. See the
				<a href="https://github.com/Nurt-studio/Sequoia-UI/releases" target="_blank" rel="noreferrer">release notes</a> for changes.
			</div>
		</div>

		<div class="docs-nav-bottom">
			<a href="#styling" class="docs-nav-link prev">
				<div class="label">← Previous</div>
				<div class="title">Styling</div>
			</a>
			<a href={BOOK} class="docs-nav-link next" target="_blank" rel="noreferrer">
				<div class="label">Next →</div>
				<div class="title">Read the full book</div>
			</a>
		</div>
	</main>

	<aside class="toc">
		<div class="toc-title">On this page</div>
		{#each tocLinks as link, i}
			<a
				href={`#${link.id}`}
				class="toc-link"
				class:active={activeToc === i}
				class:sub={link.sub}
				onclick={() => (activeToc = i)}>{link.label}</a
			>
		{/each}
	</aside>
</div>

<DocsFooter />

<style>
	.docs-layout {
		display: flex;
		max-width: 1400px;
		margin: 0 auto;
		min-height: calc(100vh - 56px);
	}

	.sidebar {
		width: 260px;
		position: sticky;
		top: 56px;
		height: calc(100vh - 56px);
		overflow-y: auto;
		padding: var(--space-5) 0;
		border-right: 1px solid #e5e5e7;
		flex-shrink: 0;
	}
	.sidebar::-webkit-scrollbar {
		width: 4px;
	}
	.sidebar::-webkit-scrollbar-thumb {
		background: #ddd;
		border-radius: 4px;
	}
	.sidebar-group {
		margin-bottom: 8px;
	}
	.sidebar-group-title {
		font-size: var(--text-xs);
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: var(--tracking-wide);
		color: #999;
		padding: var(--space-2) var(--space-5);
	}
	.sidebar-link {
		display: block;
		padding: var(--space-2) var(--space-5) var(--space-2) var(--space-6);
		font-size: var(--text-sm);
		font-weight: 500;
		color: #555;
		text-decoration: none;
		transition: all 0.15s;
		border-left: 2px solid transparent;
	}
	.sidebar-link:hover {
		color: #1a1a1a;
		background: #f0f0f0;
	}
	.sidebar-link.active {
		color: #e6391e;
		background: rgba(230, 57, 30, 0.04);
		border-left-color: #e6391e;
		font-weight: 600;
	}

	.docs-main {
		flex: 1;
		padding: var(--space-7);
		max-width: 780px;
		min-width: 0;
	}

	.breadcrumbs {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-xs);
		color: #999;
		margin-bottom: var(--space-6);
	}
	.breadcrumbs a {
		color: #999;
		text-decoration: none;
		transition: color 0.2s;
	}
	.breadcrumbs a:hover {
		color: #e6391e;
	}
	.breadcrumbs .sep {
		color: #ccc;
	}
	.breadcrumbs .current {
		color: #1a1a1a;
		font-weight: 600;
	}

	.docs-main h1 {
		font-size: var(--text-4xl);
		font-weight: 800;
		letter-spacing: var(--tracking-tighter);
		line-height: var(--leading-tight);
		margin-bottom: var(--space-4);
		text-wrap: balance;
	}
	.docs-main .subtitle {
		font-size: var(--text-lg);
		color: #666;
		line-height: var(--leading-normal);
		margin-bottom: var(--space-7);
		font-weight: 400;
	}
	.docs-main h2 {
		font-size: var(--text-2xl);
		font-weight: 700;
		letter-spacing: var(--tracking-tight);
		line-height: var(--leading-snug);
		margin-top: var(--space-7);
		margin-bottom: var(--space-4);
		padding-bottom: var(--space-3);
		border-bottom: 1px solid #e5e5e7;
		scroll-margin-top: 72px;
		text-wrap: balance;
	}
	.docs-main p {
		font-size: var(--text-base);
		line-height: var(--leading-relaxed);
		color: #444;
		margin-bottom: var(--space-5);
	}
	.docs-main a {
		color: #e6391e;
		text-decoration: none;
		font-weight: 500;
	}
	.docs-main a:hover {
		text-decoration: underline;
	}
	.docs-main ul {
		margin-bottom: var(--space-5);
		padding-left: var(--space-5);
	}
	.docs-main li {
		font-size: var(--text-base);
		line-height: var(--leading-relaxed);
		color: #444;
		margin-bottom: var(--space-2);
	}
	.docs-main li::marker {
		color: #e6391e;
	}

	.callout {
		padding: var(--space-5);
		border-radius: 10px;
		margin: var(--space-6) 0;
		font-size: var(--text-sm);
		line-height: var(--leading-normal);
		display: flex;
		gap: var(--space-3);
		align-items: flex-start;
	}
	.callout-icon {
		font-size: var(--text-lg);
		flex-shrink: 0;
		margin-top: 1px;
	}
	.callout-info {
		background: rgba(126, 200, 227, 0.1);
		border: 1px solid rgba(126, 200, 227, 0.3);
		color: #2a7a8c;
	}
	.callout-warning {
		background: rgba(230, 57, 30, 0.06);
		border: 1px solid rgba(230, 57, 30, 0.2);
		color: #a02000;
	}
	.callout-tip {
		background: rgba(152, 195, 121, 0.1);
		border: 1px solid rgba(152, 195, 121, 0.3);
		color: #4a7a30;
	}
	.callout strong {
		display: block;
		margin-bottom: 4px;
		font-weight: 700;
	}

	.inline-code {
		background: #eee;
		padding: 2px 7px;
		border-radius: 5px;
		font-family: var(--font-mono);
		font-size: 0.875em;
		color: #e6391e;
		font-weight: 500;
		overflow-wrap: anywhere;
	}

	.docs-table {
		width: 100%;
		border-collapse: collapse;
		margin: var(--space-6) 0;
		font-size: var(--text-sm);
	}
	.docs-table th {
		text-align: left;
		padding: var(--space-3) var(--space-4);
		font-weight: 700;
		font-size: var(--text-xs);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: #999;
		border-bottom: 2px solid #e5e5e7;
	}
	.docs-table td {
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid #eee;
		color: #444;
	}
	.docs-table tr:hover td {
		background: #fafafa;
	}

	.docs-nav-bottom {
		display: flex;
		gap: var(--space-4);
		margin-top: var(--space-8);
		padding-top: var(--space-6);
		border-top: 1px solid #e5e5e7;
	}
	.docs-nav-link {
		flex: 1;
		padding: var(--space-5);
		border: 1px solid #e5e5e7;
		border-radius: 12px;
		text-decoration: none;
		transition: all 0.2s;
		background: #fff;
	}
	.docs-nav-link:hover {
		border-color: var(--accent);
		box-shadow: 0 4px 12px rgba(230, 57, 30, 0.08);
	}
	.docs-nav-link .label {
		font-size: var(--text-xs);
		color: #999;
		font-weight: 500;
		margin-bottom: var(--space-1);
	}
	.docs-nav-link .title {
		font-size: var(--text-base);
		font-weight: 700;
		color: var(--fg);
	}
	.docs-nav-link.next {
		text-align: right;
	}

	.toc {
		width: 200px;
		position: sticky;
		top: 56px;
		height: calc(100vh - 56px);
		padding: var(--space-7) var(--space-5);
		flex-shrink: 0;
		overflow-y: auto;
	}
	.toc-title {
		font-size: var(--text-xs);
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: var(--tracking-wide);
		color: #999;
		margin-bottom: var(--space-4);
	}
	.toc-link {
		display: block;
		font-size: var(--text-xs);
		color: #888;
		text-decoration: none;
		padding: var(--space-1) 0 var(--space-1) var(--space-3);
		border-left: 2px solid #e5e5e7;
		margin-bottom: var(--space-2);
		transition: all 0.15s;
		line-height: 1.4;
	}
	.toc-link:hover {
		color: #1a1a1a;
		border-left-color: #ccc;
	}
	.toc-link.active {
		color: #e6391e;
		border-left-color: #e6391e;
		font-weight: 600;
	}
	.toc-link.sub {
		padding-left: var(--space-5);
		font-size: var(--text-2xs);
	}

	@media (max-width: 1200px) {
		.toc {
			display: none;
		}
	}
	@media (max-width: 900px) {
		.sidebar {
			display: none;
		}
		.docs-main {
			padding: var(--space-6) var(--section-px);
		}
		.docs-nav-bottom {
			flex-direction: column;
		}
		.docs-nav-link.next {
			text-align: left;
		}
	}
	@media (max-width: 768px) {
		.docs-main h1 {
			font-size: var(--text-3xl);
		}
	}
</style>