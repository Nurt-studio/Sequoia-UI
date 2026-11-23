<script>
	import { onMount } from 'svelte';
	import DocsTopBar from '../components/DocsTopBar.svelte';
	import DocsFooter from '../components/DocsFooter.svelte';
	import CodeBlock from '../components/CodeBlock.svelte';
	import LayoutPreview from '../components/LayoutPreview.svelte';
	import LayoutPlayground from '../components/LayoutPlayground.svelte';
	import Icon from '../components/Icon.svelte';

	const BOOK = 'https://nurt-studio.github.io/Sequoia-UI/';
	const DOCS_RS = 'https://docs.rs/sequoia-ui';
	const CRATES = 'https://crates.io/crates/sequoia-ui';
	const GITHUB = 'https://github.com/Nurt-studio/Sequoia-UI';

	const quickLinks = [
		{
			icon: 'package',
			title: 'Installation',
			desc: 'Add the facade crate and get a window on screen in minutes.',
			href: '#installation'
		},
		{
			icon: 'rocket',
			title: 'Your first app',
			desc: 'A reactive counter in about twenty lines of Rust.',
			href: '#first-app'
		},
		{
			icon: 'blocks',
			title: 'Widget library',
			desc: 'Browse buttons, inputs, tables, docks and overlays.',
			href: '#widgets'
		},
		{
			icon: 'book',
			title: 'API reference',
			desc: 'Every public type, documented on docs.rs.',
			href: DOCS_RS,
			external: true
		}
	];

	const capabilities = [
		{ icon: 'monitor', title: 'Native windows', desc: 'Real OS windows through winit, painted by wgpu. No web view and no JavaScript runtime.' },
		{ icon: 'zap', title: 'Fine-grained reactivity', desc: 'Signals, memos and effects update only the views that read them — no virtual DOM.' },
		{ icon: 'grid', title: 'IDE-style layouts', desc: 'Rows, columns, grids, stacks, split panels, docks and floating windows.' },
		{ icon: 'palette', title: 'Themable styles', desc: 'Semantic themes plus typed per-widget styles with per-property overrides.' },
		{ icon: 'film', title: 'Animation', desc: 'Springs, timelines and state-driven transitions wired into the same runtime.' },
		{ icon: 'camera', title: 'Headless tooling', desc: 'Render to PNG, GIF or AVI without opening a window, and test the same view tree.' }
	];

	const sidebarGroups = [
		{
			title: 'Getting Started',
			links: [
				{ label: 'Introduction', href: '#introduction' },
				{ label: 'Installation', href: '#installation' },
				{ label: 'Your first app', href: '#first-app' },
				{ label: 'Project structure', href: '#project-structure' },
				{ label: 'Runnable examples', href: '#examples' }
			]
		},
		{
			title: 'Core Concepts',
			links: [
				{ label: 'Reactivity', href: '#reactive-state' },
				{ label: 'Layout', href: '#layout' },
				{ label: 'Layout playground', href: '#layout-playground' },
				{ label: 'Styling', href: '#styling' },
				{ label: 'Events and focus', href: '#events' },
				{ label: 'Async and animation', href: '#async' }
			]
		},
		{
			title: 'Widget Library',
			links: [
				{ label: 'Widgets overview', href: '#widgets' },
				{ label: 'Component recipes', href: '#component-recipes' },
				{ label: 'Forms and validation', href: '#forms' },
				{ label: 'Collections and overlays', href: '#collections' },
				{ label: 'Inputs guide', href: `${BOOK}guides/inputs.html` },
				{ label: 'Docking', href: `${BOOK}guides/docking.html` }
			]
		},
		{
			title: 'Rendering & Tooling',
			links: [
				{ label: 'Rendering pipeline', href: '#rendering' },
				{ label: 'Crate map', href: '#crates' },
				{ label: 'Export and testing', href: '#export-testing' },
				{ label: 'Custom shaders', href: `${BOOK}guides/shaders.html` },
				{ label: 'The .seq format', href: `${BOOK}guides/seq.html` }
			]
		},
		{
			title: 'Reference',
			links: [
				{ label: 'API reference', href: '#api-reference' },
				{ label: 'FAQ', href: '#faq' },
				{ label: 'Full book', href: BOOK },
				{ label: 'docs.rs', href: DOCS_RS },
				{ label: 'crates.io', href: CRATES }
			]
		}
	];

	const tocLinks = [
		{ id: 'why-sequoia', label: 'Why SequoiaUI?' },
		{ id: 'installation', label: 'Installation' },
		{ id: 'first-app', label: 'Your first app' },
		{ id: 'project-structure', label: 'Project structure' },
		{ id: 'reactive-state', label: 'Reactivity' },
		{ id: 'layout', label: 'Layout' },
		{ id: 'layout-playground', label: 'Live playground', sub: true },
		{ id: 'layout-examples', label: 'Container examples', sub: true },
		{ id: 'styling', label: 'Styling' },
		{ id: 'events', label: 'Events and focus' },
		{ id: 'widgets', label: 'Widgets' },
		{ id: 'component-recipes', label: 'Component recipes', sub: true },
		{ id: 'forms', label: 'Forms and validation', sub: true },
		{ id: 'collections', label: 'Collections and overlays', sub: true },
		{ id: 'rendering', label: 'Rendering pipeline' },
		{ id: 'async', label: 'Async and animation' },
		{ id: 'export-testing', label: 'Export and testing' },
		{ id: 'examples', label: 'Runnable examples' },
		{ id: 'api-reference', label: 'API reference' },
		{ id: 'faq', label: 'FAQ' }
	];

	let activeToc = $state('why-sequoia');
	let query = $state('');

	const lowercaseQuery = $derived(query.trim().toLowerCase());
	const filteredGroups = $derived(
		sidebarGroups
			.map((group) => ({
				title: group.title,
				links: group.links.filter((link) => link.label.toLowerCase().includes(lowercaseQuery))
			}))
			.filter((group) => group.links.length > 0)
	);

	onMount(() => {
		const headings = tocLinks
			.map((link) => document.getElementById(link.id))
			.filter((element) => element !== null);
		let ticking = false;

		const update = () => {
			ticking = false;
			const marker = window.scrollY + 100;
			let current = headings[0];
			for (const heading of headings) {
				if (heading.offsetTop <= marker) current = heading;
			}
			if (current) activeToc = current.id;
		};

		const onScroll = () => {
			if (!ticking) {
				ticking = true;
				requestAnimationFrame(update);
			}
		};

		update();
		window.addEventListener('scroll', onScroll, { passive: true });
		return () => window.removeEventListener('scroll', onScroll);
	});

	const installHtml = `<span class="cm"># Cargo.toml</span>
[dependencies]
sequoia-ui = <span class="str">"0.1.6"</span>`;

	const cargoHtml = `cargo new my-app
<span class="kw">cd</span> my-app
cargo add sequoia-ui
rustc --version   <span class="cm"># 1.88 or newer</span>
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

	const structureHtml = `my-app/
├─ Cargo.toml
└─ src/
   ├─ main.rs          <span class="cm"># App::new().run()</span>
   └─ ui/
      ├─ mod.rs        <span class="cm"># pub mod toolbar; pub mod editor;</span>
      ├─ toolbar.rs    <span class="cm"># a view that returns impl View</span>
      └─ editor.rs

<span class="cm"># larger workspaces keep one crate for UI and one for domain logic</span>`;

	const reactiveHtml = `<span class="kw">let</span> width = Signal::new(<span class="num">2i32</span>);
<span class="kw">let</span> height = Signal::new(<span class="num">3i32</span>);

<span class="kw">let</span> area = memo(&#123;
    <span class="kw">let</span> width = width.clone();
    <span class="kw">let</span> height = height.clone();
    <span class="kw">move</span> || width.get() * height.get()
&#125;);

assert_eq!(area.get(), <span class="num">6</span>);

batch(|| &#123;
    width.set(<span class="num">4</span>);
    height.set(<span class="num">5</span>);
&#125;);

assert_eq!(area.get(), <span class="num">20</span>); <span class="cm"># recomputed once</span>`;

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

	const formsHtml = `<span class="kw">let</span> form = Form::new();
<span class="kw">let</span> email = Signal::new(String::new());
<span class="kw">let</span> seats = Signal::new(<span class="num">2.0</span>);

<span class="kw">let</span> field = Field::new(email, <span class="str">"Email"</span>)
    .placeholder(<span class="str">"you@example.com"</span>);

<span class="kw">let</span> seat_field = NumberField::new(seats, <span class="str">"Seats"</span>)
    .min(<span class="num">1.0</span>)
    .max(<span class="num">50.0</span>);

form.register(&amp;field);
form.register(&amp;seat_field);
assert!(form.is_valid());

<span class="kw">let</span> layout = Column::new()
    .gap(<span class="num">12.0</span>)
    .child(field)
    .child(seat_field)
    .child(Button::new(<span class="str">"Create account"</span>).style(ButtonStyle::PRIMARY));`;

	const themeHtml = `<span class="kw">let mut</span> theme = Theme::light();
theme.accent = Color::hex(<span class="num">0x5B8DEF</span>);
theme.radius = <span class="num">6.0</span>;

<span class="kw">let</span> app = App::new()
    .title(<span class="str">"Styled"</span>)
    .theme(theme);

<span class="kw">let</span> card = Container::new()
    .background(Color::hex(<span class="num">0x202024</span>))
    .border_left(<span class="num">4.0</span>, Color::hex(<span class="num">0x4DA3FF</span>))
    .border_y(<span class="num">1.0</span>, Color::hex(<span class="num">0x2E2E36</span>))
    .radius(<span class="num">6.0</span>);`;

	const styleHtml = `<span class="kw">let</span> save = Button::new(<span class="str">"Save"</span>)
    .style(ButtonStyle::PRIMARY)
    .style_with(|style| style.radius(<span class="num">10.0</span>))
    .on_click(|| &#123; <span class="cm">/* persist */</span> &#125;);

<span class="kw">let</span> warning = Text::new(<span class="str">"Unsaved changes"</span>)
    .size(<span class="num">13.0</span>)
    .color(Color::hex(<span class="num">0xE6391E</span>));`;

	const layoutRowsHtml = `<span class="kw">let</span> name = Signal::new(String::new());

<span class="kw">let</span> header = Row::new()
    .justify_content(Justify::SpaceBetween)
    .align_items(Align::Center)
    .padding(<span class="num">16.0</span>)
    .child(Text::new(<span class="str">"Project settings"</span>).size(<span class="num">20.0</span>).bold())
    .child(Button::new(<span class="str">"Save"</span>).style(ButtonStyle::PRIMARY));

<span class="kw">let</span> split = Row::new()
    .fill_size()
    .gap(<span class="num">16.0</span>)
    .padding(<span class="num">16.0</span>)
    .child(
        Column::new()
            .width(<span class="num">240.0</span>)
            .flex_shrink(<span class="num">0.0</span>)
            .gap(<span class="num">8.0</span>)
            .child(Text::new(<span class="str">"Sections"</span>).semibold())
            .child(Text::new(<span class="str">"General"</span>))
            .child(Text::new(<span class="str">"Members"</span>)),
    )
    .child_flex(
        Column::new()
            .gap(<span class="num">12.0</span>)
            .child(Field::new(name, <span class="str">"Name"</span>))
            .child(Button::new(<span class="str">"Rename"</span>)),
        <span class="num">1.0</span>,
    );

<span class="kw">let</span> page = Column::new().fill_size().child(header).child(split);`;

	const layoutGridHtml = `<span class="kw">let</span> metrics = Grid::new([
    Track::minmax(<span class="num">160.0</span>, Track::Fill(<span class="num">1.0</span>)),
    Track::minmax(<span class="num">160.0</span>, Track::Fill(<span class="num">1.0</span>)),
    Track::minmax(<span class="num">160.0</span>, Track::Fill(<span class="num">1.0</span>)),
])
    .gap(<span class="num">12.0</span>)
    .fill_width()
    .child_span(Text::new(<span class="str">"Usage"</span>).bold(), <span class="num">0</span>, <span class="num">0</span>, <span class="num">3</span>, <span class="num">1</span>)
    .child(Text::new(<span class="str">"CPU"</span>))
    .child(Text::new(<span class="str">"Memory"</span>))
    .child(Text::new(<span class="str">"Disk"</span>));

<span class="kw">let</span> tags = Wrap::new()
    .gap(<span class="num">6.0</span>)
    .children((<span class="num">0</span>..<span class="num">8</span>).map(|i| Text::new(format!(<span class="str">"tag-&#123;&#125;"</span>, i))));`;

	const layoutStackHtml = `<span class="kw">let</span> canvas = Stack::new()
    .fill_size()
    .child(Container::new().fill_size().background(Color::hex(<span class="num">0xF4F4F6</span>)))
    .child(Text::new(<span class="str">"Drop an image here"</span>))
    .child(Button::new(<span class="str">"Close"</span>));`;

	const layoutDockHtml = `<span class="kw">let</span> shell = DockArea::new()
    .top(
        Row::new()
            .gap(<span class="num">8.0</span>)
            .padding(<span class="num">8.0</span>)
            .child(Button::new(<span class="str">"Open"</span>))
            .child(Spacer::flex(<span class="num">1.0</span>))
            .child(Text::new(<span class="str">"Ready"</span>)),
    )
    .left(
        Column::new()
            .width(<span class="num">240.0</span>)
            .padding(<span class="num">16.0</span>)
            .gap(<span class="num">8.0</span>)
            .child(Text::new(<span class="str">"Explorer"</span>).bold())
            .child(Text::new(<span class="str">"src"</span>))
            .child(Text::new(<span class="str">"Cargo.toml"</span>)),
    )
    .bottom(Text::new(<span class="str">"Status: ready"</span>).height(<span class="num">24.0</span>))
    .fill(Container::new().fill_size().padding(<span class="num">16.0</span>).child(Text::new(<span class="str">"Editor"</span>)))
    .gap(<span class="num">2.0</span>)
    .padding(<span class="num">4.0</span>);`;

	const componentButtonsHtml = `<span class="kw">let</span> save = Button::new(<span class="str">"Save"</span>)
    .style(ButtonStyle::PRIMARY)
    .on_click(|| &#123; <span class="cm">/* persist the document */</span> &#125;);

<span class="kw">let</span> toolbar = Row::new()
    .gap(<span class="num">6.0</span>)
    .align_items(Align::Center)
    .child(Button::icon(<span class="str">"folder"</span>).style(ButtonStyle::GHOST))
    .child(Button::ribbon(<span class="str">"save"</span>, <span class="str">"Save"</span>).style(ButtonStyle::PRIMARY))
    .child(save)
    .child(Button::new(<span class="str">"Delete"</span>).style(ButtonStyle::DANGER).disabled(<span class="kw">true</span>))
    .child(Divider::vertical())
    .child(Kbd::new(<span class="str">"Ctrl+S"</span>));`;

	const componentTextHtml = `<span class="kw">let</span> user = Row::new()
    .gap(<span class="num">8.0</span>)
    .align_items(Align::Center)
    .child(Avatar::named(<span class="str">"Ada Lovelace"</span>).size(<span class="num">28.0</span>))
    .child(
        Column::new()
            .child(Text::new(<span class="str">"Ada Lovelace"</span>).semibold())
            .child(Text::new(<span class="str">"@ada"</span>).size(<span class="num">12.0</span>).color(Color::hex(<span class="num">0x9A9A9A</span>))),
    )
    .child(Badge::count_capped(<span class="num">12</span>, <span class="num">9</span>));`;

	const componentInputsHtml = `<span class="kw">let</span> email = Signal::new(String::new());
<span class="kw">let</span> notify = Signal::new(<span class="kw">true</span>);
<span class="kw">let</span> plan = Signal::new(<span class="num">1usize</span>);
<span class="kw">let</span> volume = Signal::new(<span class="num">40.0</span>);

<span class="kw">let</span> settings = Column::new()
    .gap(<span class="num">12.0</span>)
    .padding(<span class="num">24.0</span>)
    .child(TextField::new(email).placeholder(<span class="str">"you@example.com"</span>).width(<span class="num">240.0</span>))
    .child(Checkbox::new(notify.clone()).label(<span class="str">"Notify on save"</span>))
    .child(Switch::new(notify).label(<span class="str">"Desktop notifications"</span>))
    .child(RadioGroup::new(plan, [<span class="str">"Free"</span>, <span class="str">"Pro"</span>, <span class="str">"Team"</span>]).label(<span class="str">"Plan"</span>).horizontal())
    .child(Slider::new(volume).min(<span class="num">0.0</span>).max(<span class="num">100.0</span>).width(<span class="num">200.0</span>));`;

	const componentTableHtml = `#[derive(Clone, PartialEq)]
<span class="kw">struct</span> FileRow &#123;
    id: <span class="ty">u64</span>,
    name: <span class="ty">String</span>,
    size: <span class="ty">u64</span>,
&#125;

<span class="kw">let</span> rows = Signal::new(vec![
    FileRow &#123; id: <span class="num">1</span>, name: <span class="str">"notes.txt"</span>.to_string(), size: <span class="num">2_048</span> &#125;,
    FileRow &#123; id: <span class="num">2</span>, name: <span class="str">"photo.png"</span>.to_string(), size: <span class="num">512_000</span> &#125;,
]);

<span class="kw">let</span> files = Table::new(&#123;
        <span class="kw">let</span> rows = rows.clone();
        <span class="kw">move</span> || rows.get()
    &#125;)
    .key(|row: &amp;FileRow| row.id)
    .label(<span class="str">"Files"</span>)
    .column(
        TableColumn::new(<span class="str">"Name"</span>)
            .width(Length::Fill(<span class="num">2.0</span>))
            .sort_key(|a: &amp;FileRow, b: &amp;FileRow| a.name.cmp(&amp;b.name))
            .text_cell(|row: &amp;FileRow| row.name.clone()),
    )
    .column(
        TableColumn::new(<span class="str">"Size"</span>)
            .width(Length::Px(<span class="num">100.0</span>))
            .text_cell(|row: &amp;FileRow| format!(<span class="str">"&#123;&#125; KB"</span>, row.size / <span class="num">1024</span>)),
    )
    .row_height(<span class="num">32.0</span>);`;

	const componentOverlaysHtml = `<span class="kw">let</span> open = Signal::new(<span class="kw">false</span>);

<span class="kw">let</span> dialog = Dialog::new(open.clone(), <span class="str">"Delete project?"</span>)
    .content(Text::new(<span class="str">"This action cannot be undone."</span>))
    .cancel(<span class="str">"Cancel"</span>)
    .danger(<span class="str">"Delete"</span>, || &#123; <span class="cm">/* delete the project */</span> &#125;);

<span class="kw">let</span> menu = FloatingMenu::wrap(Button::new(<span class="str">"Actions"</span>))
    .below_invoker()
    .open_on_click()
    .text_row(<span class="str">"Duplicate"</span>, || &#123; <span class="cm">/* duplicate */</span> &#125;)
    .separator()
    .text_row(<span class="str">"Delete"</span>, || &#123; <span class="cm">/* delete */</span> &#125;);

<span class="kw">let</span> queue = ToastQueue::new();
<span class="kw">let</span> stack = ToastStack::new(queue.clone()).max_visible(<span class="num">3</span>);
queue.error(<span class="str">"Upload failed"</span>, <span class="str">"Check your connection and retry."</span>);`;

	const componentFeedbackHtml = `<span class="kw">let</span> progress = Signal::new(<span class="num">0.42</span>);

<span class="kw">let</span> status = Row::new()
    .gap(<span class="num">12.0</span>)
    .align_items(Align::Center)
    .child(Spinner::new().size(<span class="num">18.0</span>))
    .child(ProgressBar::new(progress).height(<span class="num">8.0</span>).fill(Color::hex(<span class="num">0x3FB950</span>)))
    .child(Skeleton::new().width(<span class="num">160.0</span>).height(<span class="num">12.0</span>))
    .child(Tooltip::new(Button::icon(<span class="str">"check"</span>), <span class="str">"Mark as done"</span>));`;

	const componentScreenHtml = `<span class="kw">let</span> show_archived = Signal::new(<span class="kw">false</span>);
<span class="cm">// files: the Table from the previous example</span>

<span class="kw">let</span> page = DockArea::new()
    .top(
        Row::new()
            .gap(<span class="num">8.0</span>)
            .padding(<span class="num">8.0</span>)
            .align_items(Align::Center)
            .child(Button::new(<span class="str">"New file"</span>))
            .child(Spacer::flex(<span class="num">1.0</span>))
            .child(Kbd::new(<span class="str">"Ctrl+S"</span>)),
    )
    .left(
        Column::new()
            .width(<span class="num">220.0</span>)
            .gap(<span class="num">8.0</span>)
            .padding(<span class="num">16.0</span>)
            .child(Text::new(<span class="str">"Filters"</span>).bold())
            .child(Checkbox::new(show_archived).label(<span class="str">"Show archived"</span>)),
    )
    .fill(Column::new().fill_size().padding(<span class="num">12.0</span>).child(files))
    .gap(<span class="num">2.0</span>);

<span class="kw">let</span> app = App::new().title(<span class="str">"Files"</span>).size(<span class="num">900.0</span>, <span class="num">600.0</span>).root(page);`;

	const asyncHtml = `<span class="kw">let</span> users = Resource::&lt;Vec&lt;String&gt;&gt;::new();
<span class="kw">let</span> generation = users.begin_loading();
users.complete(generation, vec![<span class="str">"Ada"</span>.to_string()]);

<span class="kw">let</span> view = AsyncView::new(users.clone())
    .loading(|| Text::new(<span class="str">"Loading users…"</span>))
    .data(|rows| &#123;
        Column::new().children(
            rows.iter()
                .map(|name| Text::new(name.clone()))
                .collect::&lt;Vec&lt;_&gt;&gt;(),
        )
    &#125;)
    .error(|message| Text::new(format!(<span class="str">"Could not load: &#123;&#125;"</span>, message)));`;

	const examplesHtml = `cargo run -p sequoia-ui --example showcase
cargo run -p sequoia-ui --example form
cargo run -p sequoia-ui --example counter
cargo run -p sequoia-ui --example layout
cargo run -p sequoia-ui --example photo_lab
cargo run -p sequoia-ui --example seq_dashboard
cargo run -p sequoia-ui --example split-panel
cargo run -p sequoia-ui --example wgpu_surface
cargo run -p vision-design`;

	const faqs = [
		{
			q: 'Does SequoiaUI run in a browser or a web view?',
			a: 'No. Every window is a native OS window painted through wgpu. There is no web build, no JavaScript runtime, and no embedded browser.'
		},
		{
			q: 'Why does my window fail to open?',
			a: 'App::run returns a Result. Print the error — usually it is a missing GPU adapter or driver. Update the graphics driver and check that DirectX 12 or Vulkan is available (Metal on macOS).'
		},
		{
			q: 'Can I share a Signal with a worker thread?',
			a: 'No. Signals use Rc internally and are single-threaded. Compute on the worker thread, then deliver the result to the UI thread through the async bridge.'
		},
		{
			q: 'Do I add the internal crates directly?',
			a: 'No. Depend on the sequoia-ui facade crate only. It re-exports the whole framework through sequoia_ui::prelude, and the internal crate names are not a stable API.'
		},
		{
			q: 'Can I use .seq instead of Rust?',
			a: 'Yes, .seq is supported as an optional declarative format loaded through SeqLoader, with typed host bindings. It is still experimental; the Rust API remains the supported path.'
		},
		{
			q: 'How do I take a screenshot or export a video?',
			a: 'The export module renders a view tree to PNG, animated GIF or Motion JPEG AVI without opening a window — including on machines without a display server.'
		}
	];

	const crates = [
		{ name: 'sequoia-ui', role: 'Facade crate most applications depend on; re-exports everything through the prelude.' },
		{ name: 'sequoia-core', role: 'Geometry, colors, paint primitives and the shared types used across the stack.' },
		{ name: 'sequoia-reactive', role: 'Signals, memos, effects, batching, stores and undo history.' },
		{ name: 'sequoia-view', role: 'The View trait, layout engine and primitive containers.' },
		{ name: 'sequoia-elements', role: 'Buttons, inputs, tables, trees, docks, overlays and the rest of the widget library.' },
		{ name: 'sequoia-render', role: 'wgpu renderer, render passes, shaders, effects and GPU resource budgets.' },
		{ name: 'sequoia-platform', role: 'Native windowing, the event loop, input routing and OS integration.' },
		{ name: 'sequoia-assets', role: 'Fonts, icons, images and the asset registry.' },
		{ name: 'sequoia-export', role: 'Headless rendering to PNG, GIF and AVI.' },
		{ name: 'sequoia-seq', role: 'Parser, evaluator and bindings for the experimental .seq format.' }
	];
</script>

<DocsTopBar />

<div class="docs-layout">
	<aside class="sidebar">
		<div class="sidebar-search">
			<input type="search" placeholder="Filter navigation…" bind:value={query} aria-label="Filter navigation" />
		</div>

		{#each filteredGroups as group}
			<div class="sidebar-group">
				<div class="sidebar-group-title">{group.title}</div>
				{#each group.links as link}
					<a
						href={link.href}
						class="sidebar-link"
						class:active={link.href.startsWith('#') && link.href === `#${activeToc}`}
						target={link.href.startsWith('http') ? '_blank' : undefined}
						rel={link.href.startsWith('http') ? 'noreferrer' : undefined}>{link.label}</a
					>
				{/each}
			</div>
		{/each}

		{#if filteredGroups.length === 0}
			<p class="sidebar-empty">No navigation entries match “{query}”.</p>
		{/if}

		<div class="sidebar-help">
			<div class="sidebar-help-title">Need a hand?</div>
			<p>Open an issue or read the full book for chapter-length guides.</p>
			<a href={GITHUB} target="_blank" rel="noreferrer">Ask on GitHub →</a>
		</div>
	</aside>

	<main class="docs-main" id="introduction">
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

		<div class="hero-meta">
			<span class="meta-chip"><strong>v0.1.6</strong> current release</span>
			<span class="meta-chip">Rust 1.88+</span>
			<span class="meta-chip">Windows · macOS · Linux</span>
			<span class="meta-chip">MIT licensed</span>
		</div>

		<div class="quick-links">
			{#each quickLinks as link}
				<a
					class="quick-card"
					href={link.href}
					target={link.external ? '_blank' : undefined}
					rel={link.external ? 'noreferrer' : undefined}>
					<span class="quick-icon"><Icon name={link.icon} size={20} /></span>
					<span class="quick-title">{link.title}</span>
					<span class="quick-desc">{link.desc}</span>
				</a>
			{/each}
		</div>

		<details class="mobile-toc">
			<summary>On this page</summary>
			<nav>
				{#each tocLinks as link}
					<a href={`#${link.id}`} class:sub={link.sub}>{link.label}</a>
				{/each}
			</nav>
		</details>

		<div class="callout callout-info">
			<span class="callout-icon"><Icon name="info" size={18} /></span>
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

		<div class="capability-grid">
			{#each capabilities as item}
				<div class="capability-card">
					<span class="capability-icon"><Icon name={item.icon} size={20} /></span>
					<div class="capability-title">{item.title}</div>
					<p>{item.desc}</p>
				</div>
			{/each}
		</div>

		<ul>
			<li><strong>Native rendering</strong> — wgpu on Metal, Vulkan, DX12 or OpenGL</li>
			<li><strong>Memory safe</strong> — owned Rust values, no garbage collector; the workspace forbids unsafe code</li>
			<li><strong>Declarative API</strong> — describe your UI as a tree of views</li>
			<li><strong>One dependency</strong> — <span class="inline-code">sequoia-ui</span> re-exports the whole framework</li>
			<li><strong>No web toolchain</strong> — no JavaScript runtime, no bundled browser, no HTML/CSS layer</li>
		</ul>

		<h2 id="installation">Installation</h2>
		<p>Add the facade crate to your project using Cargo:</p>

		<CodeBlock lang="Terminal" code={installHtml} />

		<p>Or create a new binary crate and let Cargo resolve the framework crates:</p>

		<CodeBlock lang="Terminal" code={cargoHtml} />

		<h3>Requirements</h3>
		<ul class="check-list">
			<li><strong>Rust 1.88</strong> or newer — check with <span class="inline-code">rustc --version</span>.</li>
			<li>A desktop platform supported by winit and wgpu: Windows, macOS or Linux.</li>
			<li>A working GPU driver: DirectX 12 or Vulkan on Windows, Metal on macOS, Vulkan on Linux.</li>
			<li>No web toolchain, JavaScript runtime or system web view.</li>
		</ul>

		<div class="callout callout-tip">
			<span class="callout-icon"><Icon name="lightbulb" size={18} /></span>
			<div>
				<strong>Do not add internal crates by hand</strong>
				<span class="inline-code">sequoia-view</span>, <span class="inline-code">sequoia-reactive</span> and
				the other crates are resolved through the facade. Their names are not a stable API surface.
			</div>
		</div>

		<h3>Import the prelude</h3>
		<p>
			Most applications need a single import. The prelude exposes <span class="inline-code">App</span>,
			<span class="inline-code">Signal</span>, common views such as <span class="inline-code">Column</span>,
			<span class="inline-code">Row</span> and <span class="inline-code">Text</span>, built-in widgets,
			layout types, colors and animation helpers.
		</p>

		<CodeBlock
			lang="Rust"
			code={`<span class="kw">use</span> sequoia_ui::prelude::*;\n<span class="kw">use</span> sequoia_ui::render::GpuOptions; <span class="cm">// modules outside the prelude</span>`}
		/>

		<p>
			Every subsystem is available as a module from the facade:
			<span class="inline-code">animation</span>, <span class="inline-code">assets</span>,
			<span class="inline-code">async_bridge</span>, <span class="inline-code">core</span>,
			<span class="inline-code">draw</span>, <span class="inline-code">elements</span>,
			<span class="inline-code">export</span>, <span class="inline-code">input</span>,
			<span class="inline-code">layout</span>, <span class="inline-code">platform</span>,
			<span class="inline-code">reactive</span>, <span class="inline-code">render</span>,
			<span class="inline-code">seq</span>, <span class="inline-code">shader</span>,
			<span class="inline-code">style</span>, <span class="inline-code">text</span> and
			<span class="inline-code">view</span>.
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

		<h3>What runs when</h3>
		<ol class="step-list">
			<li><strong>Startup</strong> — <span class="inline-code">run</span> creates the window, the GPU surface and the renderer.</li>
			<li><strong>Mount</strong> — the root view is mounted and laid out for the current window size.</li>
			<li><strong>Input</strong> — window events are routed to views and widgets; focus and hover are tracked.</li>
			<li><strong>Update</strong> — changing a signal marks its subscribers dirty and requests a redraw.</li>
			<li><strong>Exit</strong> — the event loop returns when the last window closes.</li>
		</ol>

		<div class="callout callout-tip">
			<span class="callout-icon"><Icon name="lightbulb" size={18} /></span>
			<div>
				<strong>Try it yourself</strong>
				Add a <em>Decrement</em> button, a <em>Reset</em> button that calls
				<span class="inline-code">count.set(0)</span>, then rewrite the label as a full sentence.
			</div>
		</div>

		<h2 id="project-structure">Project structure</h2>
		<p>
			A small application can live in a single <span class="inline-code">main.rs</span>. As the UI grows,
			split views into their own modules and keep domain logic out of the view tree; views stay cheap
			when they only read signals and compose children.
		</p>

		<CodeBlock lang="Text" code={structureHtml} />

		<p>
			Views are ordinary Rust functions or structs that return a view. Because they compose with
			<span class="inline-code">.child(...)</span>, a module boundary is just a function boundary — there
			is no registration step and no hidden global registry.
		</p>

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

		<h3>Signal API</h3>
		<table class="docs-table">
			<thead>
				<tr>
					<th>Method</th>
					<th>What it does</th>
					<th>Tracks a read?</th>
				</tr>
			</thead>
			<tbody>
				<tr>
					<td><span class="inline-code">get()</span></td>
					<td>Clones the current value.</td>
					<td>Yes</td>
				</tr>
				<tr>
					<td><span class="inline-code">set(value)</span></td>
					<td>Replaces the value and notifies subscribers. Equal values are a no-op for <span class="inline-code">PartialEq</span> types.</td>
					<td>—</td>
				</tr>
				<tr>
					<td><span class="inline-code">update(fn)</span></td>
					<td>Clones, mutates and writes back.</td>
					<td>—</td>
				</tr>
				<tr>
					<td><span class="inline-code">with(fn)</span></td>
					<td>Borrows the value and returns a derived result.</td>
					<td>Yes</td>
				</tr>
				<tr>
					<td><span class="inline-code">get_untracked()</span></td>
					<td>Reads without subscribing the caller. <span class="inline-code">untracked(|| ...)</span> wraps a block.</td>
					<td>No</td>
				</tr>
				<tr>
					<td><span class="inline-code">subscribe(fn)</span></td>
					<td>Runs the listener once and again on change; the returned <span class="inline-code">Subscription</span> disposes it.</td>
					<td>—</td>
				</tr>
			</tbody>
		</table>

		<h3>Memos, effects and batching</h3>
		<p>
			A <span class="inline-code">Memo&lt;T&gt;</span> caches a derived value and recomputes it only when a
			dependency changes. An <span class="inline-code">effect</span> runs immediately and again whenever a
			signal it read changes — keep the returned handle alive, because dropping it disposes the effect.
			<span class="inline-code">batch</span> defers effect flushing until the outermost closure finishes.
		</p>

		<CodeBlock lang="Rust" code={reactiveHtml} />

		<p>
			For values that need history, <span class="inline-code">Store&lt;T&gt;</span> records up to 100 undo
			entries and <span class="inline-code">UndoSignal&lt;T&gt;</span> can coalesce rapid edits such as
			typing. Wrap compound edits in <span class="inline-code">transact</span> so they become one undo step.
		</p>

		<div class="callout callout-warning">
			<span class="callout-icon"><Icon name="alert" size={18} /></span>
			<div>
				<strong>Signals are single-threaded</strong>
				They use <span class="inline-code">Rc</span> internally. Compute on a worker thread, then deliver
				the result to the UI thread with the async bridge.
			</div>
		</div>

		<h2 id="layout">Layout</h2>
		<p>
			Layout runs in two passes: <strong>measure</strong> asks each view how large it wants to be for the
			available constraints, then <strong>arrange</strong> assigns final rectangles. Both passes repeat
			after a resize or a reactive update that changes layout.
		</p>
		<p>
			<strong>Row</strong> and <strong>Column</strong> handle linear flow, with <strong>Grid</strong>,
			<strong>Wrap</strong>, <strong>Stack</strong>, <strong>SplitPanel</strong> and
			<strong>DockArea</strong> for the rest. <strong>Spacer::flex(1.0)</strong> pushes siblings apart.
			Switch between the examples to compare the code with a visual approximation of the result.
		</p>

		<LayoutPreview />

		<h3 id="layout-playground">Try the layouts live</h3>
		<p>
			Drag the canvas width and change the controls to watch each layout reflow. Grid has four track
			modes: equal columns, auto-fit with a minimum track, weighted fractions, and a fixed sidebar plus
			fill. The generated block shows both the preview CSS and the equivalent Rust builders.
		</p>

		<LayoutPlayground />

		<h3 id="layout-examples">Container examples</h3>
		<p>
			Each container solves one arrangement problem. Start with the simplest one that works, and nest
			containers only when two directions are involved.
		</p>

		<table class="docs-table">
			<thead>
				<tr>
					<th>Container</th>
					<th>Arranges children</th>
					<th>Reach for it when</th>
				</tr>
			</thead>
			<tbody>
				<tr><td><span class="inline-code">Column</span></td><td>Top to bottom</td><td>Forms, sidebars, vertical stacks</td></tr>
				<tr><td><span class="inline-code">Row</span></td><td>Left to right</td><td>Toolbars, split views, inline labels</td></tr>
				<tr><td><span class="inline-code">Grid</span></td><td>Rows and columns</td><td>Dashboards, aligned panels</td></tr>
				<tr><td><span class="inline-code">Wrap</span></td><td>Lines that flow</td><td>Tags, toolbars, galleries</td></tr>
				<tr><td><span class="inline-code">Stack</span></td><td>Overlapping layers</td><td>Badges, floating labels, canvases</td></tr>
				<tr><td><span class="inline-code">DockArea</span></td><td>Edges plus a fill child</td><td>Editor shells, IDE-style layouts</td></tr>
			</tbody>
		</table>

		<h4>Toolbar above a split body</h4>
		<p>
			A <span class="inline-code">Column</span> stacks the header and the body. Inside the body, a
			<span class="inline-code">Row</span> holds a fixed sidebar and a flexible content area.
		</p>

		<CodeBlock lang="Rust" code={layoutRowsHtml} />

		<ul>
			<li><span class="inline-code">.justify_content(Justify::SpaceBetween)</span> pushes the title and button apart; <span class="inline-code">.align_items(Align::Center)</span> centers them vertically.</li>
			<li><span class="inline-code">.flex_shrink(0.0)</span> keeps the sidebar at 240 px when the window narrows.</li>
			<li><span class="inline-code">.child_flex(view, 1.0)</span> gives the content all remaining width; a second flexible child would share it by weight.</li>
			<li><span class="inline-code">.fill_size()</span> on the page and body makes them take the space their parent provides.</li>
		</ul>

		<h4>Grids and wrapping rows</h4>
		<p>
			Use <span class="inline-code">Grid</span> when cells must line up across rows, and
			<span class="inline-code">Wrap</span> when children should flow onto another line as the width shrinks.
		</p>

		<CodeBlock lang="Rust" code={layoutGridHtml} />

		<ul>
			<li><span class="inline-code">Track::Fill(1.0)</span> shares free width; wrap it in <span class="inline-code">Track::minmax(160.0, ...)</span> so a column never collapses below 160 px.</li>
			<li><span class="inline-code">.child_span(view, column, row, column_span, row_span)</span> places a heading across all three columns.</li>
			<li><span class="inline-code">Wrap</span> moves children to the next line automatically; <span class="inline-code">.columns(n)</span> replaces that with a fixed count.</li>
		</ul>

		<h4>Layers with Stack</h4>
		<p>
			<span class="inline-code">Stack</span> places children on top of each other. The first child is at the
			back, later children paint over it, and children are measured against the same box.
		</p>

		<CodeBlock lang="Rust" code={layoutStackHtml} />

		<p>
			Use it for a canvas with floating controls, an image with a caption, or a badge in a corner. For
			dismissible layers and focus handling, use the overlay widgets instead of a Stack.
		</p>

		<h4>Editor shell with DockArea</h4>
		<p>
			<span class="inline-code">DockArea</span> reserves edges and gives everything that is left to a
			<span class="inline-code">fill</span> child. It is the shortest path to a toolbar + sidebar + status bar
			shell and does not create tabs or draggable panels.
		</p>

		<CodeBlock lang="Rust" code={layoutDockHtml} />

		<ul>
			<li><span class="inline-code">.top()</span>, <span class="inline-code">.bottom()</span>, <span class="inline-code">.left()</span> and <span class="inline-code">.right()</span> reserve edges; <span class="inline-code">.fill()</span> receives the remaining space.</li>
			<li>Edges take their size from the child — give them an explicit width or height.</li>
			<li>For tabs, draggable splits and floating panels, use <span class="inline-code">DockView</span> with a <span class="inline-code">DockModel</span>.</li>
		</ul>

		<div class="callout callout-tip">
			<span class="callout-icon"><Icon name="lightbulb" size={18} /></span>
			<div>
				<strong>Debugging layout</strong>
				If a view does not grow, its parent probably does not bound the axis. <span class="inline-code">Fill</span>
				and <span class="inline-code">Percent</span> fall back to the intrinsic size when the available space is
				unbounded.
			</div>
		</div>

		<h3 id="lengths">Lengths and the box model</h3>
		<p>
			Every view has a <span class="inline-code">LayoutStyle</span> with requested size, padding, margin,
			min/max sizes, flex behaviour, positioning and overflow. Containers take
			<span class="inline-code">impl Into&lt;Length&gt;</span>, and an <span class="inline-code">f32</span>
			converts to pixels.
		</p>

		<table class="docs-table">
			<thead>
				<tr>
					<th>Value</th>
					<th>Meaning</th>
				</tr>
			</thead>
			<tbody>
				<tr><td><span class="inline-code">Length::Px(v)</span></td><td>Fixed logical pixels.</td></tr>
				<tr><td><span class="inline-code">Length::Percent(p)</span></td><td>Percentage of the parent’s available space.</td></tr>
				<tr><td><span class="inline-code">Length::Fill(weight)</span></td><td>Share of the remaining space, proportional to weight.</td></tr>
				<tr><td><span class="inline-code">Length::Fit</span></td><td>The view’s intrinsic size.</td></tr>
			</tbody>
		</table>

		<ul>
			<li><span class="inline-code">.fill_width()</span>, <span class="inline-code">.fill_height()</span> and <span class="inline-code">.fill_size()</span> are shorthands for <span class="inline-code">Length::Fill(1.0)</span>.</li>
			<li><span class="inline-code">Align</span> controls the cross axis (Start, Center, End, Stretch); <span class="inline-code">Justify</span> controls the main axis and adds SpaceBetween, SpaceAround and SpaceEvenly.</li>
			<li><span class="inline-code">.center()</span> centers on both axes; <span class="inline-code">.gap(...)</span> spaces siblings; padding is inside the border, margin outside.</li>
			<li>Set <span class="inline-code">.flex_shrink(0.0)</span> on a sidebar that must keep its width when the window narrows.</li>
		</ul>

		<h2 id="styling">Styling</h2>
		<p>
			A theme supplies colors, typography, radii and shadows; widgets ship with typed style presets and
			per-property builders. Styles are plain Rust values, so they can be computed, stored in signals and
			shared between views.
		</p>

		<CodeBlock lang="Rust" code={themeHtml} />

		<p>
			Every widget has a dedicated style struct whose fields are optional, so a style can be a partial
			override. <span class="inline-code">ButtonStyle</span> ships PRIMARY, SECONDARY, DANGER, GHOST,
			OUTLINE, DARK_FILLED and OUTLINE_ROSE presets; <span class="inline-code">style_with</span> tweaks one
			detail without defining a new style.
		</p>

		<CodeBlock lang="Rust" code={styleHtml} />

		<ul>
			<li><strong>BoxStyle</strong> — background, per-edge borders, radius, shadows and inset shadows for containers.</li>
			<li><strong>TextStyle</strong> — size, weight, color, line height and font; <span class="inline-code">RichText</span> mixes spans with colors, italics, underlines and links.</li>
			<li><strong>Visual states</strong> — interactive widgets resolve normal, hovered, pressed, focused, selected and disabled colors from the theme.</li>
			<li><strong>Backdrops</strong> — <span class="inline-code">BackdropBlur</span> frosts whatever was painted underneath, for sheets, menus and toasts.</li>
		</ul>

		<h2 id="events">Events and focus</h2>
		<p>
			Window events are routed to the view tree by the runtime. Widgets translate raw input into
			high-level hooks, so most applications never touch a platform event. Focus, hover and press state
			are tracked per view and repaint only the affected region.
		</p>

		<table class="docs-table">
			<thead>
				<tr>
					<th>Interaction</th>
					<th>What the runtime handles</th>
					<th>Hook you use</th>
				</tr>
			</thead>
			<tbody>
				<tr><td>Pointer</td><td>Hover, press, release and click with the correct target view.</td><td><span class="inline-code">on_press</span>, <span class="inline-code">on_click</span></td></tr>
				<tr><td>Keyboard</td><td>Tab traversal, Enter and Space activation, arrow navigation in collections.</td><td>Focus ring, <span class="inline-code">Kbd</span> hints</td></tr>
				<tr><td>Text input</td><td>IME composition, selection, clipboard, secure fields and soft wrap.</td><td><span class="inline-code">TextField</span>, <span class="inline-code">Field</span></td></tr>
				<tr><td>Continuous input</td><td>Drag updates during the gesture, one commit at the end.</td><td><span class="inline-code">on_change_start</span>, <span class="inline-code">on_commit</span></td></tr>
				<tr><td>Window</td><td>Resize, DPI change, close request and custom title bars.</td><td><span class="inline-code">App</span> builders, <span class="inline-code">WindowControls</span></td></tr>
			</tbody>
		</table>

		<p>
			Buttons activate with Enter or Space when focused. A checkbox is for an option confirmed with a
			form; a switch applies a setting immediately. Both accept <span class="inline-code">.disabled()</span>
			and <span class="inline-code">.on_toggle</span>.
		</p>

		<h2 id="widgets">Widgets and state</h2>
		<p>
			Built-in widgets bind directly to signals. The runtime handles focus, keyboard activation and
			accessibility; you keep the state.
		</p>

		<CodeBlock lang="Rust" code={widgetsHtml} />

		<h3 id="component-recipes">Component recipes</h3>
		<p>
			Every widget is a view: it composes with layout containers, reads its value from a signal and is
			configured with builder methods. The recipes below show the usual shape of each family; all of them
			can be nested inside the containers from the previous section.
		</p>

		<h4>Buttons and actions</h4>
		<p>
			<span class="inline-code">Button::new</span> is a text button, <span class="inline-code">Button::icon</span> is icon-only
			and <span class="inline-code">Button::ribbon</span> places an icon next to a label. Use
			<span class="inline-code">on_click</span> for the completed action and
			<span class="inline-code">on_press</span> for repeatable toolbar buttons.
		</p>

		<CodeBlock lang="Rust" code={componentButtonsHtml} />

		<ul>
			<li>Styles: <span class="inline-code">PRIMARY</span>, <span class="inline-code">SECONDARY</span>, <span class="inline-code">DANGER</span>, <span class="inline-code">GHOST</span>, <span class="inline-code">OUTLINE</span>, <span class="inline-code">DARK_FILLED</span> and <span class="inline-code">OUTLINE_ROSE</span>.</li>
			<li><span class="inline-code">.disabled(true)</span> blocks clicks and removes the button from keyboard activation.</li>
			<li>Buttons activate with Enter or Space when focused; add a focus ring when focus visibility matters.</li>
		</ul>

		<h4>Text, avatars and badges</h4>
		<p>
			Text is a view like any other, so it can be styled inline or wrapped in a
			<span class="inline-code">Row</span> with an avatar and a badge.
		</p>

		<CodeBlock lang="Rust" code={componentTextHtml} />

		<ul>
			<li><span class="inline-code">Text::with(|| ...)</span> makes the label reactive; <span class="inline-code">Text::new</span> is for fixed strings.</li>
			<li>Typographic builders include <span class="inline-code">.size()</span>, <span class="inline-code">.bold()</span>, <span class="inline-code">.semibold()</span>, <span class="inline-code">.italic()</span>, <span class="inline-code">.color()</span> and <span class="inline-code">.font()</span>.</li>
			<li><span class="inline-code">RichText</span> mixes spans with their own weight, color, underline and link.</li>
		</ul>

		<h4>Inputs and selections</h4>
		<p>
			Input widgets own a signal and write user edits back into it. There is no separate form state to
			synchronize — reading the signal anywhere else updates that view automatically.
		</p>

		<CodeBlock lang="Rust" code={componentInputsHtml} />

		<ul>
			<li><span class="inline-code">TextField</span> binds a <span class="inline-code">Signal&lt;String&gt;</span>; <span class="inline-code">Field</span> adds a label, error text and touched state.</li>
			<li><span class="inline-code">Checkbox</span> and <span class="inline-code">Switch</span> both bind a <span class="inline-code">Signal&lt;bool&gt;</span>; <span class="inline-code">RadioGroup</span> writes the selected index.</li>
			<li><span class="inline-code">Slider</span> writes an <span class="inline-code">f32</span>; use <span class="inline-code">on_change_start</span> and <span class="inline-code">on_commit</span> around expensive work.</li>
			<li><span class="inline-code">DropdownButton</span>, <span class="inline-code">MultiSelect</span> and <span class="inline-code">ColorPicker</span> cover the remaining pickers.</li>
		</ul>

		<h4>Lists and tables</h4>
		<p>
			Collections render rows from a closure and never own your selection state. The key decides which
			row is reused when the data changes, so derive it from a stable id.
		</p>

		<CodeBlock lang="Rust" code={componentTableHtml} />

		<ul>
			<li><span class="inline-code">Table</span> virtualizes rows; give it a height and never wrap it in a <span class="inline-code">ScrollView</span>.</li>
			<li><span class="inline-code">sort_key</span> makes a column sortable; <span class="inline-code">matches</span> adds filtering for that column.</li>
			<li>For a simple vertical list, use <span class="inline-code">List</span>; for very long uniform rows, <span class="inline-code">VirtualList</span>.</li>
		</ul>

		<h4>Dialogs, menus and toasts</h4>
		<p>
			Overlays share one runtime layer system, so focus, dismissal and stacking are handled for you.
			Bind a <span class="inline-code">Signal&lt;bool&gt;</span> for visibility and let the widget manage the rest.
		</p>

		<CodeBlock lang="Rust" code={componentOverlaysHtml} />

		<ul>
			<li><span class="inline-code">Dialog</span> is modal and traps focus; <span class="inline-code">Modal</span>, <span class="inline-code">Drawer</span> and <span class="inline-code">Sheet</span> are the generic surfaces.</li>
			<li><span class="inline-code">FloatingMenu</span> attaches to a trigger; <span class="inline-code">ContextMenu</span> opens on right-click.</li>
			<li>Keep one <span class="inline-code">ToastStack</span> near the root and share its queue instead of creating a stack per screen.</li>
		</ul>

		<h4>Progress and feedback</h4>
		<p>
			Feedback widgets read state and paint the result; none of them needs its own controller.
		</p>

		<CodeBlock lang="Rust" code={componentFeedbackHtml} />

		<h4>Putting it together</h4>
		<p>
			Layouts and components are ordinary values, so a screen is just a tree: a
			<span class="inline-code">DockArea</span> shell, a toolbar <span class="inline-code">Row</span>, a sidebar
			<span class="inline-code">Column</span>, and a <span class="inline-code">Table</span> in the fill slot.
		</p>

		<CodeBlock lang="Rust" code={componentScreenHtml} />

		<h3>Map of the library</h3>
		<div class="widget-grid">
			<a class="widget-card" href={`${BOOK}guides/widgets.html`} target="_blank" rel="noreferrer">
				<span class="widget-name">Actions</span>
				<span class="widget-items">Button, icon and ribbon buttons, Kbd, Divider, WindowControls</span>
			</a>
			<a class="widget-card" href={`${BOOK}concepts/text.html`} target="_blank" rel="noreferrer">
				<span class="widget-name">Text &amp; identity</span>
				<span class="widget-items">Text, RichText, Icon, Image, Avatar, Badge</span>
			</a>
			<a class="widget-card" href={`${BOOK}guides/inputs.html`} target="_blank" rel="noreferrer">
				<span class="widget-name">Inputs</span>
				<span class="widget-items">TextField, NumberField, DateField, Checkbox, Switch, RadioGroup, Slider, DragValue, Chip</span>
			</a>
			<a class="widget-card" href={`${BOOK}guides/collections.html`} target="_blank" rel="noreferrer">
				<span class="widget-name">Collections</span>
				<span class="widget-items">List, Table, Tree, virtualized rows, selection models</span>
			</a>
			<a class="widget-card" href={`${BOOK}guides/overlays.html`} target="_blank" rel="noreferrer">
				<span class="widget-name">Overlays</span>
				<span class="widget-items">Dialog, Drawer, Menu, Tooltip, Toast, Popover</span>
			</a>
			<a class="widget-card" href={`${BOOK}guides/scrolling.html`} target="_blank" rel="noreferrer">
				<span class="widget-name">Scrolling</span>
				<span class="widget-items">ScrollView, virtualization, sticky headers</span>
			</a>
			<a class="widget-card" href={`${BOOK}guides/widgets.html`} target="_blank" rel="noreferrer">
				<span class="widget-name">Feedback</span>
				<span class="widget-items">ProgressBar, Spinner, Skeleton, validation banners</span>
			</a>
			<a class="widget-card" href={`${BOOK}guides/docking.html`} target="_blank" rel="noreferrer">
				<span class="widget-name">Navigation &amp; docking</span>
				<span class="widget-items">Tabs, DockArea, DockPanel, SplitPanel, floating windows</span>
			</a>
		</div>

		<h2 id="forms">Forms and validation</h2>
		<p>
			A form is a validation registry, not a container. Build the layout yourself and register each field
			so the form can report validity as a whole. <span class="inline-code">Form</span> exposes
			<span class="inline-code">is_valid</span>, <span class="inline-code">touch_all</span> and
			<span class="inline-code">summary</span>; the summary renders the first error in a form-level banner.
		</p>

		<CodeBlock lang="Rust" code={formsHtml} />

		<ul>
			<li><strong>Field</strong> wraps a text field with a label, validation error and touched state.</li>
			<li><strong>NumberField</strong>, <strong>DateField</strong> and <strong>MultiSelect</strong> follow the same pattern.</li>
			<li>Errors appear only after a field is touched, so a fresh form is not covered in red text.</li>
		</ul>

		<h2 id="collections">Collections and overlays</h2>
		<p>
			Large data sets are handled by list, table and tree views backed by selection models and
			virtualization. Only visible rows are shaped and painted, so a table with tens of thousands of
			entries scrolls at the same cost as a short one.
		</p>
		<ul>
			<li><strong>Table</strong> — resizable columns, pinned headers, row selection and per-column alignment.</li>
			<li><strong>Tree</strong> — expandable nodes, keyboard navigation and lazy children.</li>
			<li><strong>ScrollView</strong> — clipping, scroll momentum and sticky regions.</li>
			<li><strong>Overlays</strong> — dialogs, drawers, menus, tooltips, toasts and popovers, layered above the painted content.</li>
			<li><strong>Tabs and docks</strong> — tab strips, draggable panels, split view and floating windows with restorable dock state.</li>
		</ul>

		<h2 id="rendering">Rendering pipeline</h2>
		<p>
			The renderer batches painted geometry and submits it to wgpu. Each frame reuses cached layouts and
			shaped text, so a static interface costs almost nothing to repaint.
		</p>

		<ol class="step-list">
			<li><strong>Measure</strong> — views resolve intrinsic sizes against the available constraints.</li>
			<li><strong>Arrange</strong> — final rectangles are assigned; layout is cached until its inputs change.</li>
			<li><strong>Paint</strong> — backgrounds, borders, shadows, text and images become paint commands.</li>
			<li><strong>Batch</strong> — commands are grouped by pass, pipeline and texture to reduce draw calls.</li>
			<li><strong>Composite</strong> — offscreen targets, opacity groups and shader passes are blended into the surface.</li>
		</ol>

		<p>
			Groups of views can be rendered into their own target, which powers opacity layers, backdrop blur
			and custom shader chains. GPU surfaces are composited in paint order, and frame metrics expose CPU
			and GPU timings for profiling.
		</p>

		<h3 id="crates">Crate map</h3>
		<table class="docs-table">
			<thead>
				<tr>
					<th>Crate</th>
					<th>Responsibility</th>
				</tr>
			</thead>
			<tbody>
				{#each crates as crate}
					<tr>
						<td><span class="inline-code">{crate.name}</span></td>
						<td>{crate.role}</td>
					</tr>
				{/each}
			</tbody>
		</table>

		<h2 id="async">Async and animation</h2>
		<p>
			Long-running work should not block the UI thread. <span class="inline-code">Resource&lt;T&gt;</span>
			tracks loading, ready, failed and cancelled states, while <span class="inline-code">AsyncView</span>
			maps each state to a view. The <span class="inline-code">Executor</span> runs work on a worker pool
			and delivers results to the UI thread, so signals are never touched from another thread.
		</p>

		<CodeBlock lang="Rust" code={asyncHtml} />

		<p>
			Start work with <span class="inline-code">spawn_resource</span>,
			<span class="inline-code">spawn_to_signal</span> or <span class="inline-code">spawn_stream</span>;
			<span class="inline-code">Debouncer</span>, <span class="inline-code">Throttler</span>,
			<span class="inline-code">sleep</span> and <span class="inline-code">timeout</span> cover time-based
			workflows.
		</p>

		<ul>
			<li><strong>Springs</strong> animate a value toward a target with configurable stiffness and damping.</li>
			<li><strong>Timelines</strong> drive a value over a duration with easing, and can be chained.</li>
			<li><strong>Transitions</strong> react to state changes, so hover, selection and route changes animate without manual bookkeeping.</li>
			<li><strong>Frame hooks</strong> run once per frame when you need to advance custom state.</li>
		</ul>

		<h2 id="export-testing">Export and testing</h2>
		<div class="split-cards">
			<div class="split-card">
				<span class="split-card-icon"><Icon name="camera" size={20} /></span>
				<h3>Headless export</h3>
				<p>
					Render a view tree to a HiDPI PNG without opening a window, or capture an animated GIF or
					Motion JPEG AVI. CPU shaders work in headless renders too, so CI can produce screenshots on
					machines without a display.
				</p>
				<a href={`${BOOK}guides/export.html`} target="_blank" rel="noreferrer">Export guide →</a>
			</div>
			<div class="split-card">
				<span class="split-card-icon"><Icon name="flask" size={20} /></span>
				<h3>Testing</h3>
				<p>
					The same view tree runs in tests: build a UI, dispatch input, assert on signal state and
					compare exported frames. Widgets and layouts are ordinary values, so most tests need no
					window at all.
				</p>
				<a href={`${BOOK}guides/testing.html`} target="_blank" rel="noreferrer">Testing guide →</a>
			</div>
		</div>

		<h2 id="examples">Runnable examples</h2>
		<p>
			The repository ships examples that need no extra setup. Clone it and run them from the workspace
			root:
		</p>

		<CodeBlock lang="Terminal" code={examplesHtml} />

		<p>
			<strong>showcase</strong> is a tour of the widget library; <strong>form</strong>,
			<strong>photo_lab</strong> and <strong>seq_dashboard</strong> are small complete apps; and
			<strong>vision-design</strong> is the editor in the screenshot on the home page.
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
					<td><span class="inline-code">View</span></td>
					<td>Core trait implemented by every element</td>
					<td><span class="inline-code">sequoia_ui::view</span></td>
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
					<td><span class="inline-code">Container</span></td>
					<td>Styled box with optional decoration</td>
					<td><span class="inline-code">sequoia_ui::view</span></td>
				</tr>
				<tr>
					<td><span class="inline-code">Grid</span>, <span class="inline-code">Stack</span></td>
					<td>Track-based grid and overlapping layers</td>
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
				<tr>
					<td><span class="inline-code">Memo&lt;T&gt;</span></td>
					<td>Cached derived value</td>
					<td><span class="inline-code">sequoia_ui::reactive</span></td>
				</tr>
				<tr>
					<td><span class="inline-code">Store&lt;T&gt;</span></td>
					<td>Reactive value with undo and redo history</td>
					<td><span class="inline-code">sequoia_ui::reactive</span></td>
				</tr>
				<tr>
					<td><span class="inline-code">Theme</span></td>
					<td>Semantic colors, radii and metrics</td>
					<td><span class="inline-code">sequoia_ui::style</span></td>
				</tr>
				<tr>
					<td><span class="inline-code">Color</span></td>
					<td>RGBA color with hex and rgb8 constructors</td>
					<td><span class="inline-code">sequoia_ui::core</span></td>
				</tr>
				<tr>
					<td><span class="inline-code">Length</span></td>
					<td>Pixel, percent, fill or fit sizing</td>
					<td><span class="inline-code">sequoia_ui::layout</span></td>
				</tr>
			</tbody>
		</table>

		<p>
			Every type above is re-exported by
			<span class="inline-code">sequoia_ui::prelude</span>. The complete, searchable list lives on
			<a href={DOCS_RS} target="_blank" rel="noreferrer">docs.rs</a>.
		</p>

		<h2 id="faq">FAQ</h2>
		<div class="faq">
			{#each faqs as faq}
				<details>
					<summary>{faq.q}</summary>
					<p>{faq.a}</p>
				</details>
			{/each}
		</div>

		<div class="callout callout-warning">
			<span class="callout-icon"><Icon name="alert" size={18} /></span>
			<div>
				<strong>Pre-1.0</strong>
				This book targets SequoiaUI 0.1.x. APIs may still change between minor releases; every example is
				written for the version in the repository. See the
				<a href="https://github.com/Nurt-studio/Sequoia-UI/releases" target="_blank" rel="noreferrer">release notes</a> for changes.
			</div>
		</div>

		<div class="docs-nav-bottom">
			<a href="#export-testing" class="docs-nav-link prev">
				<div class="label">← Previous</div>
				<div class="title">Export and testing</div>
			</a>
			<a href={BOOK} class="docs-nav-link next" target="_blank" rel="noreferrer">
				<div class="label">Next →</div>
				<div class="title">Read the full book</div>
			</a>
		</div>
	</main>

	<aside class="toc">
		<div class="toc-title">On this page</div>
		{#each tocLinks as link}
			<a
				href={`#${link.id}`}
				class="toc-link"
				class:active={activeToc === link.id}
				class:sub={link.sub}
				onclick={() => (activeToc = link.id)}>{link.label}</a
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
		padding: var(--space-5) 0 var(--space-6);
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
	.sidebar-search {
		padding: 0 var(--space-4) var(--space-4);
	}
	.sidebar-search input {
		width: 100%;
		font-family: var(--font-sans);
		font-size: var(--text-xs);
		color: #1a1a1a;
		background: #fff;
		border: 1px solid #e0e0e3;
		border-radius: 8px;
		padding: var(--space-2) var(--space-3);
		outline: none;
		transition: border-color 0.15s;
	}
	.sidebar-search input:focus {
		border-color: #e6391e;
	}
	.sidebar-search input::placeholder {
		color: #aaa;
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
	.sidebar-empty {
		padding: var(--space-3) var(--space-5);
		font-size: var(--text-xs);
		color: #999;
		line-height: 1.5;
	}
	.sidebar-help {
		margin: var(--space-5) var(--space-4) 0;
		padding: var(--space-4);
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 10px;
	}
	.sidebar-help-title {
		font-size: var(--text-xs);
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: #999;
		margin-bottom: var(--space-2);
	}
	.sidebar-help p {
		font-size: var(--text-xs);
		line-height: 1.55;
		color: #777;
		margin-bottom: var(--space-2);
	}
	.sidebar-help a {
		font-size: var(--text-xs);
		font-weight: 700;
		color: #e6391e;
		text-decoration: none;
	}
	.sidebar-help a:hover {
		text-decoration: underline;
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
		margin-bottom: var(--space-5);
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
	.docs-main h3 {
		font-size: var(--text-lg);
		font-weight: 700;
		letter-spacing: -0.01em;
		line-height: var(--leading-snug);
		margin-top: var(--space-6);
		margin-bottom: var(--space-3);
		scroll-margin-top: 72px;
	}
	.docs-main h4 {
		font-size: var(--text-xs);
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: #777;
		margin-top: var(--space-6);
		margin-bottom: var(--space-2);
		scroll-margin-top: 72px;
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
	.docs-main ul,
	.docs-main ol {
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

	.hero-meta {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin-bottom: var(--space-6);
	}
	.meta-chip {
		font-size: var(--text-xs);
		font-weight: 500;
		color: #666;
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 20px;
		padding: 5px 12px;
	}
	.meta-chip strong {
		color: #e6391e;
		font-weight: 700;
	}

	.quick-links {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3);
		margin-bottom: var(--space-6);
	}
	.quick-card {
		display: flex;
		flex-direction: column;
		gap: 2px;
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 12px;
		padding: var(--space-4);
		text-decoration: none !important;
		transition: all 0.2s;
	}
	.quick-card:hover {
		border-color: #e6391e;
		box-shadow: 0 4px 16px rgba(230, 57, 30, 0.08);
		transform: translateY(-1px);
	}
	.quick-icon {
		display: flex;
		align-items: center;
		color: #e6391e;
		margin-bottom: 4px;
	}
	.quick-title {
		font-size: var(--text-sm);
		font-weight: 700;
		color: #1a1a1a;
	}
	.quick-desc {
		font-size: var(--text-xs);
		line-height: 1.5;
		color: #888;
	}

	.capability-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3);
		margin: var(--space-5) 0 var(--space-6);
	}
	.capability-card {
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 12px;
		padding: var(--space-4);
	}
	.capability-icon {
		display: flex;
		align-items: center;
		color: #e6391e;
		margin-bottom: var(--space-2);
	}
	.capability-title {
		font-size: var(--text-sm);
		font-weight: 700;
		color: #1a1a1a;
		margin-bottom: 4px;
	}
	.capability-card p {
		font-size: var(--text-xs);
		line-height: 1.55;
		color: #777;
		margin: 0;
	}

	.check-list {
		list-style: none;
		padding-left: 0 !important;
	}
	.check-list li {
		padding-left: var(--space-6);
		position: relative;
	}
	.check-list li::before {
		content: '✓';
		position: absolute;
		left: 0;
		top: 0;
		color: #4a7a30;
		font-weight: 700;
	}

	.step-list {
		list-style: none;
		padding-left: 0 !important;
		counter-reset: step;
	}
	.step-list li {
		counter-increment: step;
		position: relative;
		padding-left: var(--space-7);
		margin-bottom: var(--space-3) !important;
	}
	.step-list li::before {
		content: counter(step);
		position: absolute;
		left: 0;
		top: 2px;
		width: 22px;
		height: 22px;
		border-radius: 50%;
		background: rgba(230, 57, 30, 0.1);
		color: #e6391e;
		font-size: var(--text-2xs);
		font-weight: 800;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.widget-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3);
		margin: var(--space-4) 0 var(--space-6);
	}
	.widget-card {
		display: flex;
		flex-direction: column;
		gap: 4px;
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 10px;
		padding: var(--space-4);
		text-decoration: none !important;
		transition: all 0.2s;
	}
	.widget-card:hover {
		border-color: #e6391e;
		background: #fffafa;
	}
	.widget-name {
		font-size: var(--text-sm);
		font-weight: 700;
		color: #1a1a1a;
	}
	.widget-items {
		font-size: var(--text-xs);
		line-height: 1.5;
		color: #888;
	}

	.split-cards {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-4);
		margin: var(--space-5) 0 var(--space-6);
	}
	.split-card {
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 12px;
		padding: var(--space-5);
	}
	.split-card-icon {
		display: flex;
		align-items: center;
		color: #e6391e;
		margin-bottom: var(--space-2);
	}
	.split-card h3 {
		margin: 0 0 var(--space-2);
		font-size: var(--text-base);
	}
	.split-card p {
		font-size: var(--text-sm);
		line-height: 1.6;
		color: #666;
		margin-bottom: var(--space-3);
	}
	.split-card a {
		font-size: var(--text-sm);
		font-weight: 700;
	}

	.faq {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin: var(--space-4) 0 var(--space-6);
	}
	.faq details {
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 10px;
		padding: var(--space-4) var(--space-5);
	}
	.faq details[open] {
		border-color: #dcdcdf;
		box-shadow: 0 4px 14px rgba(0, 0, 0, 0.04);
	}
	.faq summary {
		cursor: pointer;
		font-size: var(--text-sm);
		font-weight: 700;
		color: #1a1a1a;
		list-style: none;
		display: flex;
		justify-content: space-between;
		gap: var(--space-3);
	}
	.faq summary::-webkit-details-marker {
		display: none;
	}
	.faq summary::after {
		content: '+';
		color: #e6391e;
		font-weight: 700;
		flex-shrink: 0;
	}
	.faq details[open] summary::after {
		content: '−';
	}
	.faq p {
		font-size: var(--text-sm);
		line-height: 1.65;
		color: #666;
		margin: var(--space-3) 0 0;
	}

	.mobile-toc {
		display: none;
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 10px;
		padding: var(--space-3) var(--space-4);
		margin-bottom: var(--space-5);
	}
	.mobile-toc summary {
		cursor: pointer;
		font-size: var(--text-sm);
		font-weight: 700;
		color: #1a1a1a;
	}
	.mobile-toc nav {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin-top: var(--space-3);
	}
	.mobile-toc a {
		font-size: var(--text-sm);
		color: #666;
		text-decoration: none;
	}
	.mobile-toc a.sub {
		padding-left: var(--space-4);
		font-size: var(--text-xs);
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
		display: flex;
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
		line-height: 1.5;
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
		.mobile-toc {
			display: block;
		}
	}
	@media (max-width: 900px) {
		.sidebar {
			display: none;
		}
		.docs-main {
			padding: var(--space-6) var(--section-px);
			max-width: none;
		}
		.docs-nav-bottom {
			flex-direction: column;
		}
		.docs-nav-link.next {
			text-align: left;
		}
	}
	@media (max-width: 640px) {
		.quick-links,
		.capability-grid,
		.widget-grid,
		.split-cards {
			grid-template-columns: 1fr;
		}
		.docs-table {
			font-size: var(--text-xs);
		}
		.docs-table th,
		.docs-table td {
			padding: var(--space-2) var(--space-3);
		}
	}
	@media (max-width: 768px) {
		.docs-main h1 {
			font-size: var(--text-3xl);
		}
	}
</style>
