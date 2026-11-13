<script>
	import DocsTopBar from '../components/DocsTopBar.svelte';
	import DocsFooter from '../components/DocsFooter.svelte';
	import CodeBlock from '../components/CodeBlock.svelte';
	import { nav } from '../navigation.svelte.js';
	import glassPanel from '../../assets/glass-panel.gif';
	import hdrLayers from '../../assets/hdr-layers.png';
	import photoShader from '../../assets/photo-shader.jpg';
	import lutPhoto from '../../assets/lut-photo.png';

	const lutHtml = `<span class="cm">// WGSL: a 3D LUT is a Sampler3D input</span>
<span class="kw">pub fn</span> film_lut(uv: FragUv, content: Sampler2D, lut: Sampler3D) -&gt; Color &#123;
    <span class="kw">let</span> src = textureSample(content, content_sampler, uv);
    <span class="kw">let</span> mapped = textureSample(lut, lut_sampler, src.rgb);
    <span class="kw">return</span> vec4&lt;f32&gt;(mapped.rgb, src.a);
&#125;

<span class="cm">// The .cube table becomes a texture_3d, uploaded once and cached by path.</span>
g.shader_at(FilmLut, right, [<span class="num">0.5</span>, <span class="num">0.0</span>, <span class="num">1.0</span>, <span class="num">1.0</span>], &amp;[
    (<span class="str">"content"</span>, TexInput::Image(photo.clone())),
    (<span class="str">"lut"</span>, TexInput::Image(ImageSource::cube(<span class="str">"teal-orange.cube"</span>))),
]);`;

	const BOOK = 'https://nurt-studio.github.io/Sequoia-UI/';

	const exportHtml = `<span class="kw">use</span> sequoia_export::&#123;AnimationOptions, ExportOptions, Exporter&#125;;

<span class="kw">let</span> opts = ExportOptions::new(<span class="num">320.0</span>, <span class="num">240.0</span>).scale(<span class="num">2.0</span>);
<span class="kw">let</span> anim = AnimationOptions::new(<span class="num">12</span>, <span class="num">1.5</span>); <span class="cm">// 12 fps, 1.5 s</span>

<span class="cm">// Register the CPU counterpart of the WGSL shader used by the scene.</span>
<span class="kw">let</span> exporter = Exporter::new().shader(Glass &#123;
    u0: <span class="num">0.175</span>, v0: <span class="num">0.200</span>, u1: <span class="num">0.825</span>, v1: <span class="num">0.800</span>,
    width: <span class="num">208.0</span>, height: <span class="num">144.0</span>, radius: <span class="num">26.0</span>,
    blur: <span class="num">18.0</span>, tint: <span class="num">0.24</span>,
&#125;);

exporter.export_gif(Canvas::new().on_draw(scene), &amp;opts, &amp;anim, <span class="str">"glass-panel.gif"</span>)?;

<span class="cm">// Same scene and options: a self-contained Motion JPEG AVI.</span>
exporter.export_video(Canvas::new().on_draw(scene), &amp;opts, &amp;anim, <span class="str">"glass-panel.avi"</span>)?;`;

	const shaderHtml = `<span class="cm">// Typed WGSL shader: validated when the crate compiles.</span>
<span class="mac">shader!</span> &#123;
    <span class="kw">pub fn</span> <span class="fn">glass</span>(uv: FragUv, content: Sampler2D,
                       blur: f32, tint: f32) -&gt; Color &#123;
        <span class="cm">// Frosted backdrop: 12 taps on a golden-angle spiral.</span>
        <span class="kw">var</span> acc = vec4&lt;f32&gt;(<span class="num">0.0</span>);
        <span class="kw">for</span> (<span class="kw">var</span> i = <span class="num">0</span>; i &lt; <span class="num">12</span>; i = i + <span class="num">1</span>) &#123;
            <span class="kw">let</span> a = f32(i) * <span class="num">2.399963</span>;
            <span class="kw">let</span> ring = sqrt((f32(i) + <span class="num">0.5</span>) / <span class="num">12.0</span>);
            <span class="kw">let</span> suv = clamp(uv + vec2&lt;f32&gt;(cos(a), sin(a)) * ring * blur,
                            vec2&lt;f32&gt;(<span class="num">0.0</span>), vec2&lt;f32&gt;(<span class="num">1.0</span>));
            acc = acc + textureSample(content, content_sampler, suv);
        &#125;
        <span class="kw">let</span> body = mix(acc.rgb / <span class="num">12.0</span>, vec3&lt;f32&gt;(<span class="num">0.93</span>, <span class="num">0.95</span>, <span class="num">1.0</span>), tint);
        <span class="cm">// ... rounded-rect alpha, soft border, inner glow and reflection ...</span>
        <span class="kw">return</span> vec4&lt;f32&gt;(body, <span class="num">1.0</span>);
    &#125;
&#125;

<span class="cm">// Headless export runs on the CPU, so the same math is registered as a CpuShader.</span>
<span class="kw">impl</span> CpuShader <span class="kw">for</span> Glass &#123;
    <span class="kw">fn</span> <span class="fn">shade</span>(&amp;self, ctx: &amp;ShaderCtx&lt;'_&gt;) -&gt; [f32; <span class="num">4</span>] &#123;
        <span class="cm">/* ... */</span>
    &#125;
&#125;`;

	const commandHtml = `<span class="cm"># Regenerate the GIF above from the workspace.</span>
cargo run --release -p sequoia-export --example glass_panel -- glass-panel.gif`;

	const hdrHtml = `<span class="kw">let</span> (hdr, _) = g.group_with_format(
    canvas.size,
    BlendMode::Normal,
    TargetFormat::Rgba16Float,
    |inner| paint_layers(inner, canvas),
);

<span class="cm">// Tone-map the float target in the final pass.</span>
g.shader_at(
    ToneMap &#123; exposure: <span class="num">1.6</span> &#125;,
    rect,
    [<span class="num">0.0</span>, <span class="num">0.0</span>, <span class="num">1.0</span>, <span class="num">1.0</span>],
    &amp;[(&quot;content&quot;, TexInput::Target(hdr))],
);`;

	const photoHtml = `<span class="mac">shader!</span> &#123;
    <span class="kw">pub fn</span> <span class="fn">photo_curve</span>(uv: FragUv, content: Sampler2D,
                       contrast: f32, lift: f32,
                       saturation: f32, warmth: f32) -&gt; Color &#123;
        <span class="kw">let</span> src = textureSample(content, content_sampler, uv);
        <span class="kw">let</span> s = src.rgb * src.rgb * (vec3&lt;f32&gt;(<span class="num">3.0</span>) - <span class="num">2.0</span> * src.rgb);
        <span class="cm">// ... contrast, lift, luminance and warmth ...</span>
        <span class="kw">return</span> vec4&lt;f32&gt;(graded, src.a);
    &#125;
&#125;

<span class="cm">// The window and the headless export share the same uniforms.</span>
Exporter::new()
    .shader(PhotoCurve &#123; contrast: <span class="num">0.78</span>, lift: <span class="num">0.035</span>, saturation: <span class="num">1.15</span>, warmth: <span class="num">0.18</span> &#125;)
    .export_png(Canvas::new().on_draw(scene), &amp;opts, <span class="str">"photo-shader.png"</span>)?;`;

	const seqExpressionHtml = `<span class="cm">// Component arguments may be expressions over the parameters.</span>
<span class="kw">component</span> <span class="fn">Badge</span>(size: Float) &#123;
  <span class="fn">Text</span>(<span class="str">"NEW"</span>, size: size * <span class="num">2</span>, color: theme.accent)
&#125;

<span class="kw">app</span> Shop &#123;
  title: <span class="str">"Shop"</span>
  size: (<span class="num">960</span>, <span class="num">640</span>)
  theme: dark
  prop count: Float

  <span class="cm">// Styles are resolved at load time: 4 * 2 becomes 8.</span>
  <span class="kw">style</span> Card: Box &#123;
    radius: <span class="num">4</span> * <span class="num">2</span>
    background: theme.surface
  &#125;

  <span class="kw">window</span> Main &#123;
    <span class="fn">Column</span>(padding: <span class="num">16</span>, gap: <span class="num">12</span>) &#123;
      <span class="fn">Badge</span>(size: <span class="num">10</span> + <span class="num">2</span>)          <span class="cm">// 12 in, 24 pt out</span>

      <span class="fn">Container</span>(style: Card, padding: <span class="num">12</span>) &#123;
        <span class="fn">If</span>(when: count &gt; <span class="num">0</span>) &#123;
          <span class="cm">// Signal-backed: recomputed when count changes.</span>
          <span class="fn">Width</span>(value: count * <span class="num">40</span>) &#123;
            <span class="fn">Text</span>(<span class="str">"In stock"</span>)
          &#125;
        &#125;
      &#125;
    &#125;
  &#125;
&#125;`;

	const blockEditorHtml = `<span class="kw">use</span> sequoia_elements::BlockEditor;
<span class="kw">use</span> sequoia_reactive::Signal;
<span class="kw">use</span> sequoia_text::&#123;BlockType, Document, HeadingLevel&#125;;

<span class="kw">let mut</span> document = Document::new();
document.add_block(
    BlockType::Heading(HeadingLevel::H1),
    <span class="str">"Release notes"</span>,
);
document.add_block(
    BlockType::TodoItem &#123; checked: <span class="kw">true</span> &#125;,
    <span class="str">"Ship 0.1.6"</span>,
);
document.add_block(BlockType::Quote, <span class="str">"Blocks, not a text buffer."</span>);

<span class="kw">let</span> editor = BlockEditor::new(Signal::new(document))
    .font_size(<span class="num">16.0</span>)
    .padding(<span class="num">18.0</span>, <span class="num">14.0</span>);`;
</script>

<DocsTopBar version="0.1.6" />

<article class="release">
	<a
		class="back"
		href="/"
		onclick={(e) => {
			e.preventDefault();
			nav.page = 'blog';
		}}>← All release notes</a
	>

	<header class="release-head">
		<div class="meta">
			<span class="tag">Release (Unreleased)</span>
			<span class="version">v0.1.6</span>
			<span class="date">Not yet released (November 2026)</span>
		</div>
		<h1>Continuous export and block-based editing</h1>
		<p class="lede">
			<strong>sequoia-export</strong> can now render an animated scene without opening a window:
			animated GIF, Motion JPEG AVI, or a numbered PNG sequence. The same pass runs custom shaders on
			the CPU, so what you see on the GPU can be exported headlessly. This release also adds
			<strong>BlockEditor</strong>, a complete block-based text surface with rich inline content.
			The declarative <strong>.seq</strong> format also gains expressions in component arguments,
			style blocks and signal-backed attributes.
		</p>
		<a
			class="bench-link"
			href="/"
			onclick={(e) => {
				e.preventDefault();
				nav.page = 'compare-0.1.6';
			}}>See the 0.1.6 vs 0.1.5 benchmark comparison &rarr;</a
		>
	</header>

	<section class="perf-strip">
		<div class="perf-copy">
			<span class="perf-tag">Performance</span>
			<h2>Faster first frame, same 60 fps, less GPU work</h2>
			<p>
				Pre-initialized pipelines move GPU setup off the startup path. Lazy pipeline creation
				and direct-to-view rendering cut GPU time in both benchmark scenes.
			</p>
		</div>
		<div class="perf-grid">
			<div>
				<strong>-34.4 ms</strong>
				<span>settings startup</span>
				<small>186.2 to 151.8 ms</small>
			</div>
			<div>
				<strong>-15.7 ms</strong>
				<span>3,000 cells</span>
				<small>time to first frame</small>
			</div>
			<div>
				<strong>-4.9 pp</strong>
				<span>GPU 3D, cells</span>
				<small>23.8% to 18.9%</small>
			</div>
			<div>
				<strong>-0.45 ms</strong>
				<span>rows rebuild p50</span>
				<small>4.31 to 3.85 ms</small>
			</div>
		</div>
	</section>

	<figure class="shot">
		<img
			src={glassPanel}
			alt="A frosted glass panel with a soft white border over a blurred purple, pink and cyan background, exported as an animated GIF"
		/>
		<figcaption>
			<strong>glass-panel.gif</strong> — a frosted panel blurred over drifting colour, rendered by
			<span class="inline-code">sequoia-export</span> with no window and no GPU.
		</figcaption>
	</figure>

	<h2>What 0.1.6 adds</h2>
	<ul>
		<li>
			<strong>One exporter, every output.</strong>
			<span class="inline-code">Exporter</span> carries the asset registry and CPU shader
			implementations, and exposes <span class="inline-code">render_to_image</span>,
			<span class="inline-code">render_animation</span>, <span class="inline-code">export_png</span>,
			<span class="inline-code">export_gif</span>, <span class="inline-code">export_video</span> and
			<span class="inline-code">export_frames</span>.
		</li>
		<li>
			<strong>Animated GIF.</strong> <span class="inline-code">AnimationOptions</span> controls frame
			count and duration; the frame clock advances between frames, and the GIF loops forever.
		</li>
		<li>
			<strong>Video without ffmpeg.</strong> <span class="inline-code">export_video</span> writes a
			Motion JPEG AVI — one JPEG per frame in a plain RIFF container. It plays in common players and
			needs no external binary or codec.
		</li>
		<li>
			<strong>Shaders in headless renders.</strong> Implement <span class="inline-code">CpuShader</span>
			for a shader and register it with <span class="inline-code">Exporter::shader</span>. Offscreen
			targets, opacity groups and texture inputs are resolved on the CPU.
		</li>
		<li>
			<strong>The existing PNG API is unchanged.</strong> The free functions are now thin wrappers over
			<span class="inline-code">Exporter</span>, so current code keeps compiling.
		</li>
		<li>
			<strong>BlockEditor.</strong> A block-based editing surface with typed blocks, styled inline
			runs,			embeddable objects, cross-block selection, clipboard history, IME composition and smooth
			scrolling.
		</li>
		<li>
			<strong>Expressions in the <span class="inline-code">.seq</span> format.</strong> Component
			arguments, <span class="inline-code">style</span> blocks and the signal-backed attributes
			accept arithmetic, ternaries and built-ins. Slots such as
			<span class="inline-code">If(when:)</span>, <span class="inline-code">Width</span>,
			<span class="inline-code">Progress</span> or <span class="inline-code">Slider</span>
			re-evaluate their expression when the host signal changes.
		</li>
	</ul>

	<h2>One exporter, four outputs</h2>
	<table class="formats">
		<thead>
			<tr>
				<th>Output</th>
				<th>API</th>
				<th>Best for</th>
			</tr>
		</thead>
		<tbody>
			<tr>
				<td><strong>PNG</strong></td>
				<td><span class="inline-code">export_png</span></td>
				<td>Stills at any scale, including HiDPI</td>
			</tr>
			<tr>
				<td><strong>Animated GIF</strong></td>
				<td><span class="inline-code">export_gif</span></td>
				<td>Short loops, README demos, release notes</td>
			</tr>
			<tr>
				<td><strong>Motion JPEG AVI</strong></td>
				<td><span class="inline-code">export_video</span></td>
				<td>Longer clips without an ffmpeg dependency</td>
			</tr>
			<tr>
				<td><strong>PNG sequence</strong></td>
				<td><span class="inline-code">export_frames</span></td>
				<td>Feeding an external encoder or a compositor</td>
			</tr>
		</tbody>
	</table>
	<p class="note">
		GIF frames are quantized to a 256-color palette per frame; the AVI is Motion JPEG, not H.264. Both
		are intentionally self-contained, so export never shells out to another tool.
	</p>

	<h2>A block editor, not a text buffer</h2>
	<p>
		<span class="inline-code">BlockEditor</span> edits a <span class="inline-code">Document</span> made
		of typed blocks: paragraphs, headings, bullet and numbered lists, to-dos, quotes, code, callouts,
		tables and dividers. Each block keeps its own style, layout and inline content while the editor owns
		focus, selection and history.
	</p>

	<p>
		Inline runs carry their own style, and embeddable objects — mentions, math, images, statuses and
		icons — occupy one character in the document. Selection, deletion, copying and pasting therefore
		work across them without special cases, and pasting a fragment back into the editor preserves the
		object rather than turning it into plain text.
	</p>

	<CodeBlock lang="Rust" code={blockEditorHtml} />

	<ul>
		<li>
			<strong>Selection across blocks.</strong> Anchor and focus are document positions, so a drag can
			start in one block and end in another. Delete, cut, copy and paste operate on the whole range.
		</li>
		<li>
			<strong>Real editing semantics.</strong> Enter splits a block and keeps its type; Backspace
			merges backwards; undo and redo restore document and focus. Cut without a selection removes the
			current block and paste restores it with a new identifier.
		</li>
		<li>
			<strong>IME aware.</strong> Preedit text stays out of the document until commit, the composition
			caret follows the candidate cursor, and <span class="inline-code">ime_cursor_rect</span> is
			reported in window coordinates.
		</li>
		<li>
			<strong>Scrolling and soft wrap.</strong> The caret stays visible while typing or navigating,
			the wheel scrolls the block layout, and hit-testing accounts for the scroll offset. Wrapped
			lines keep caret affinity at line boundaries instead of jumping to the previous visual line.
		</li>
	</ul>

	<p class="note">
		The editor inherits the ambient <span class="inline-code">Theme</span> and accepts explicit
		background, font size, padding and text style overrides. Inline objects and block decorations are
		painted by the element, so the document remains the single source of truth.
	</p>

	<h2>Expressions in the .seq format</h2>
	<p>
		Attribute values already accepted arithmetic, ternaries and built-ins, but the expression
		stopped at the edges of the declarative format. 0.1.6 carries it into the three places that
		still demanded a literal: component arguments, <span class="inline-code">style</span> blocks
		and the attributes driven by a reactive signal.
	</p>
	<ul>
		<li>
			<strong>Component parameters.</strong> A parameter used inside an expression in the body is
			substituted with the call-site value, so <span class="inline-code">Badge(size: 10 + 2)</span>
			with <span class="inline-code">size * 2</span> in the body renders at 24 pt. Numbers, lengths
			(<span class="inline-code">fill</span>/<span class="inline-code">px</span>/<span
				class="inline-code">percent</span
			>/<span class="inline-code">fit</span>), <span class="inline-code">theme.*</span>,
			<span class="inline-code">env.*</span>, <span class="inline-code">item.*</span> and
			<span class="inline-code">.seq</span> function calls all round-trip through the expression.
		</li>
		<li>
			<strong>Style blocks.</strong> <span class="inline-code">radius: 4 * 2</span> builds a
			<span class="inline-code">BoxStyle</span> with <span class="inline-code">radius: 8.0</span>,
			and the same arithmetic works for text and button fields. Styles are still resolved at load
			time and cannot see host properties.
		</li>
		<li>
			<strong>Reactive attributes.</strong> Slots that already take a signal —
			<span class="inline-code">If(when:)</span>, <span class="inline-code">Opacity</span>,
			<span class="inline-code">Scale</span>, <span class="inline-code">Rotate</span>,
			<span class="inline-code">Width</span>, <span class="inline-code">Height</span>,
			<span class="inline-code">Translate</span>, <span class="inline-code">Progress</span>,
			<span class="inline-code">Slider</span>, <span class="inline-code">DragValue</span>,
			<span class="inline-code">NumberField</span> and <span class="inline-code">SplitPanel</span> —
			now evaluate their expression inside an effect, so changing the host property updates the
			view without a reload.
		</li>
	</ul>

	<CodeBlock lang="Seq" code={seqExpressionHtml} />

	<p class="note">
		Reactivity is opt-in by widget: an attribute that takes a plain
		<span class="inline-code">f32</span>, such as <span class="inline-code">Text(size:)</span>, still
		evaluates once at load. Content set with <span class="inline-code">dynamic()</span> was already
		live, and wrapping a subtree in <span class="inline-code">Width</span> or
		<span class="inline-code">Scale</span> makes its geometry react.
	</p>

	<h2>Render the scene</h2>
	<p>
		A scene is an ordinary <span class="inline-code">Canvas</span>: draw a rounded card and a sphere into
		an offscreen group, then draw the glass shader over it. Exporting is the same view tree you would
		hand to a window.
	</p>

	<CodeBlock lang="Rust" code={exportHtml} />

	<h2>Shaders without a window</h2>
	<p>
		The <span class="inline-code">shader!</span> macro still produces typed WGSL, but the headless
		rasterizer cannot execute it. A shader used during export needs a CPU implementation with the same
		uniforms and sampling. Registering it by name is all the exporter needs.
	</p>

	<CodeBlock lang="Rust" code={shaderHtml} />

	<div class="callout">
		<strong>Fallback behaviour</strong>
		A shader without a registered <span class="inline-code">CpuShader</span> is skipped with a warning
		instead of failing the whole export. File-backed textures are not available to CPU shaders; use
		<span class="inline-code">TexInput::Target</span> or RGBA data instead.
	</div>

	<h2>The glassmorphism demo</h2>
	<p>
		The example is deliberately small: a rounded card with a dot grid and three drifting colour lights,
		and one frosted panel in the middle. The background is drawn into a capture target; the
		<span class="inline-code">glass</span> shader samples a 12-tap blur of it, tints the result milky,
		then adds a soft border and a diagonal reflection. Because the lights orbit, the colour behind the
		panel keeps moving.
	</p>

	<CodeBlock lang="Terminal" code={commandHtml} />

	<h2>16-bit intermediates</h2>
	<p>
		The compositor can now keep an intermediate at 16-bit float precision. Twenty translucent
		gradients into an <span class="inline-code">Rgba8</span> target round after every blend; the same
		twenty into <span class="inline-code">Rgba16Float</span> stay smooth until the final tone-mapped
		pass. Each target carries its own format through <span class="inline-code">RenderList</span>, and
		the renderer builds the matching pipeline per format.
	</p>

	<figure class="shot">
		<img
			src={hdrLayers}
			alt="The same twenty gradient layers composited into an RGBA8 target (left, with visible shadow bands) and an RGBA16Float target (right, smooth)"
		/>
		<figcaption>
			<strong>hdr_layers</strong> — RGBA8 on the left, RGBA16Float on the right, both tone-mapped by
			the same shader. The banding on the left is the 8-bit intermediate rounding at each of the 20
			blend steps.
		</figcaption>
	</figure>

	<CodeBlock lang="Rust" code={hdrHtml} />

	<h2>Shaders on photos</h2>
	<p>
		A photo is just another texture input. <span class="inline-code">photo_curve</span> applies an
		S-curve around mid-grey, lifts the shadows slightly, then adds saturation and a warm tint. The
		same uniforms drive the window and the headless <span class="inline-code">Exporter</span>, so the
		figure below is the exported frame, not a screenshot.
	</p>

	<figure class="shot">
		<img
			src={photoShader}
			alt="A mountain landscape split in half: the original capture on the left, a richer contrast-graded version on the right"
		/>
		<figcaption>
			<strong>photo_shader</strong> — original on the left; contrast 0.78, lift 0.035, saturation 1.15
			and warmth 0.18 on the right. Photo: Aleksandra Boguslawska (CC0).
		</figcaption>
	</figure>

	<CodeBlock lang="Rust" code={photoHtml} />

	<p class="note">
		Run them with <span class="inline-code">cargo run --example hdr_layers -p sequoia-ui</span> and
		<span class="inline-code">cargo run --example photo_shader -p sequoia-ui</span>. The photo example
		accepts <span class="inline-code">--export &lt;path&gt;</span> to write the same scene headlessly.
	</p>

	<h2>3D LUTs on the GPU</h2>
	<p>
		<span class="inline-code">Sampler3D</span> binds an Adobe/IRIDAS
		<span class="inline-code">.cube</span> table as a <span class="inline-code">texture_3d</span>. The shader
		samples the photo, then looks each colour up in the LUT; the sampler interpolates the table
		trilinearly, so the result matches a photo editor applying the same table.
	</p>

	<figure class="shot">
		<img
			src={lutPhoto}
			alt="Van Gogh's The Starry Night split in half: the original painting on the left, the same pixels graded through a teal-orange 3D LUT on the right"
		/>
		<figcaption>
			<strong>lut_photo</strong> — Van Gogh's <em>The Starry Night</em> on the left; the right half is the
			same pixels looked up in a 33³ teal-orange <span class="inline-code">.cube</span> table. The PNG is the
			headless export of the same scene, not a screenshot. Painting: public domain (Wikimedia Commons).
		</figcaption>
	</figure>

	<CodeBlock lang="Rust" code={lutHtml} />

	<p class="note">
		Run it with <span class="inline-code">cargo run --example lut_photo -p sequoia-ui</span>, or add
		<span class="inline-code">-- --export lut-photo.png</span> to write the frame headlessly. Only 3D tables
		with the default <span class="inline-code">DOMAIN_MIN/DOMAIN_MAX</span> are accepted; 1D and 2D LUTs are
		rejected with a warning.
	</p>

	<h2>Performance notes</h2>
	<p>
		Export is CPU rasterization: every pixel of every frame is shaded and composited, then GIF frames are
		palette-quantized. On the 640×480, 24-frame demo this takes about 9.5 s in a release build, of which
		roughly 7 s is GIF quantization. A debug build is about 11× slower, so use
		<span class="inline-code">--release</span> for real exports.
	</p>
	<ul>
		<li>Lower <span class="inline-code">scale</span>, fps or frame count while iterating on a scene.</li>
		<li>Prefer <span class="inline-code">export_video</span> or <span class="inline-code">export_frames</span> for longer animations.</li>
		<li>GIF output size scales with pixels × frames, not with scene complexity.</li>
	</ul>

	<h2>Upgrade notes</h2>
	<ul>
		<li>
			<span class="inline-code">export_png</span>, <span class="inline-code">render_to_image</span>,
			<span class="inline-code">export_gif</span>, <span class="inline-code">render_animation</span> and
			<span class="inline-code">export_frames</span> keep their signatures.
		</li>
		<li>
			New entry points: <span class="inline-code">Exporter</span>,
			<span class="inline-code">CpuShader</span> and <span class="inline-code">export_video</span>.
		</li>
		<li>Animations still advance the frame clock between frames, so time-based scenes loop cleanly.</li>
		<li>
			<span class="inline-code">.seq</span> expressions now also resolve in component arguments and
			<span class="inline-code">style</span> blocks. Only signal-backed attributes are reactive;
			plain numeric attributes such as <span class="inline-code">Text(size:)</span> still evaluate
			once.
		</li>
	</ul>

	<div class="release-nav">
		<a
			class="release-nav-card"
			href="/"
			onclick={(e) => {
				e.preventDefault();
				nav.page = 'blog';
			}}
		>
			<span class="label">← Previous</span>
			<span class="title">All release notes</span>
		</a>
		<a class="release-nav-card next" href={`${BOOK}guides/export.html`} target="_blank" rel="noreferrer">
			<span class="label">Next →</span>
			<span class="title">Export guide in the book</span>
		</a>
	</div>
</article>

<DocsFooter />

<style>
	.release {
		max-width: 860px;
		margin: 0 auto;
		padding: var(--space-7) var(--space-6) var(--space-8);
	}

	.back {
		display: inline-block;
		margin-bottom: var(--space-6);
		font-size: var(--text-sm);
		font-weight: 600;
		color: #777;
		text-decoration: none;
		transition: color 0.2s;
	}
	.back:hover {
		color: var(--accent);
	}

	.meta {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--space-3);
		margin-bottom: var(--space-4);
	}
	.tag {
		font-size: var(--text-xs);
		font-weight: 700;
		color: var(--accent);
		background: rgba(230, 57, 30, 0.08);
		padding: var(--space-1) var(--space-3);
		border-radius: 20px;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.version {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		font-weight: 700;
		color: #555;
		background: #eee;
		padding: var(--space-1) var(--space-2);
		border-radius: 6px;
	}
	.date {
		font-size: var(--text-xs);
		font-weight: 500;
		color: #999;
	}

	.release-head h1 {
		font-size: var(--text-5xl);
		font-weight: 800;
		letter-spacing: var(--tracking-tighter);
		line-height: var(--leading-tight);
		margin-bottom: var(--space-4);
		text-wrap: balance;
	}
	.lede {
		font-size: var(--text-lg);
		line-height: var(--leading-normal);
		color: #555;
	}
	.lede strong {
		color: var(--fg);
	}
	.bench-link {
		display: inline-block;
		margin-top: var(--space-4);
		color: var(--accent);
		text-decoration: none;
		font-size: var(--text-sm);
		font-weight: 700;
	}
	.bench-link:hover {
		text-decoration: underline;
	}

	.perf-strip {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(320px, 1.05fr);
		gap: var(--space-5);
		align-items: center;
		margin: var(--space-7) 0;
		padding: var(--space-5);
		border: 1px solid #e5e5e7;
		border-radius: 16px;
		background: #fafafa;
	}
	.perf-tag {
		color: var(--accent);
		font-size: var(--text-2xs);
		font-weight: 700;
		letter-spacing: var(--tracking-wide);
		text-transform: uppercase;
	}
	.perf-copy h2 {
		margin: var(--space-1) 0 var(--space-2);
		font-size: var(--text-xl);
		line-height: var(--leading-tight);
	}
	.perf-copy p {
		margin: 0;
		color: #666;
		font-size: var(--text-sm);
		line-height: 1.6;
	}
	.perf-grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: var(--space-2);
	}
	.perf-grid div {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: var(--space-3);
		border: 1px solid #e9e9e9;
		border-radius: 10px;
		background: #fff;
	}
	.perf-grid strong {
		color: var(--accent);
		font-size: var(--text-lg);
		letter-spacing: var(--tracking-tight);
	}
	.perf-grid span {
		color: var(--fg);
		font-size: var(--text-xs);
		font-weight: 700;
	}
	.perf-grid small {
		color: #999;
		font-size: var(--text-2xs);
	}
	@media (max-width: 720px) {
		.perf-strip {
			grid-template-columns: 1fr;
		}
	}

	.shot {
		margin: var(--space-7) 0;
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 16px;
		overflow: hidden;
	}
	.shot img {
		display: block;
		width: 100%;
		height: auto;
		background: #262626;
	}
	.shot figcaption {
		padding: var(--space-4) var(--space-5);
		font-size: var(--text-sm);
		line-height: var(--leading-normal);
		color: #777;
		border-top: 1px solid #eee;
	}
	.shot figcaption strong {
		color: var(--fg);
	}

	.release h2 {
		font-size: var(--text-2xl);
		font-weight: 700;
		letter-spacing: var(--tracking-tight);
		line-height: var(--leading-snug);
		margin-top: var(--space-7);
		margin-bottom: var(--space-4);
		padding-bottom: var(--space-3);
		border-bottom: 1px solid #e5e5e7;
		text-wrap: balance;
	}
	.release p {
		font-size: var(--text-base);
		line-height: var(--leading-relaxed);
		color: #444;
		margin-bottom: var(--space-5);
	}
	.release ul {
		margin-bottom: var(--space-5);
		padding-left: var(--space-5);
	}
	.release li {
		font-size: var(--text-base);
		line-height: var(--leading-relaxed);
		color: #444;
		margin-bottom: var(--space-3);
	}
	.release li::marker {
		color: var(--accent);
	}
	.release li strong {
		color: var(--fg);
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

	.formats {
		width: 100%;
		border-collapse: collapse;
		margin: var(--space-5) 0;
		font-size: var(--text-sm);
	}
	.formats th {
		text-align: left;
		padding: var(--space-3) var(--space-4);
		font-weight: 700;
		font-size: var(--text-xs);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: #999;
		border-bottom: 2px solid #e5e5e7;
	}
	.formats td {
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid #eee;
		color: #444;
		vertical-align: top;
	}
	.formats tr:hover td {
		background: #fafafa;
	}
	.note {
		font-size: var(--text-sm) !important;
		color: #777 !important;
	}

	.callout {
		padding: var(--space-5);
		border-radius: 10px;
		border: 1px solid rgba(126, 200, 227, 0.3);
		background: rgba(126, 200, 227, 0.1);
		color: #2a7a8c;
		font-size: var(--text-sm);
		line-height: var(--leading-normal);
		margin: var(--space-6) 0;
	}
	.callout strong {
		display: block;
		margin-bottom: 4px;
		font-weight: 700;
		color: #1f5f6d;
	}

	.release-nav {
		display: flex;
		gap: var(--space-4);
		margin-top: var(--space-8);
		padding-top: var(--space-6);
		border-top: 1px solid #e5e5e7;
	}
	.release-nav-card {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-5);
		background: #fff;
		border: 1px solid #e5e5e7;
		border-radius: 12px;
		text-decoration: none;
		transition:
			border-color 0.2s,
			box-shadow 0.2s;
	}
	.release-nav-card:hover {
		border-color: var(--accent);
		box-shadow: 0 4px 12px rgba(230, 57, 30, 0.08);
	}
	.release-nav-card .label {
		font-size: var(--text-xs);
		font-weight: 500;
		color: #999;
	}
	.release-nav-card .title {
		font-size: var(--text-base);
		font-weight: 700;
		color: var(--fg);
	}
	.release-nav-card.next {
		text-align: right;
	}

	@media (max-width: 768px) {
		.release {
			padding: var(--space-6) var(--space-4) var(--space-7);
		}
		.release-head h1 {
			font-size: var(--text-3xl);
		}
		.release-nav {
			flex-direction: column;
		}
		.release-nav-card.next {
			text-align: left;
		}
	}
</style>
