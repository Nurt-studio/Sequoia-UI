<script>
	let { lang = 'Rust', code = '' } = $props();
	let copied = $state(false);

	async function copy() {
		try {
			const tmp = document.createElement('div');
			tmp.innerHTML = code;
			await navigator.clipboard.writeText(tmp.textContent || '');
			copied = true;
			setTimeout(() => (copied = false), 2000);
		} catch {
			/* ignore */
		}
	}
</script>

<div class="code-block-wrapper">
	<div class="code-block-header">
		<span class="code-block-lang">{lang}</span>
		<button type="button" class="code-block-copy" onclick={copy}
			>{copied ? 'Copied!' : 'Copy'}</button
		>
	</div>
	<div class="code-block">
		<pre>{@html code}</pre>
	</div>
</div>

<style>
	.code-block-wrapper {
		background: #1a1a1a;
		border-radius: 12px;
		margin: var(--space-6) 0;
		overflow: hidden;
	}
	.code-block-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-3) var(--space-5);
		background: rgba(255, 255, 255, 0.04);
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}
	.code-block-lang {
		font-size: var(--text-xs);
		font-weight: 600;
		color: #888;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	.code-block-copy {
		font-size: var(--text-xs);
		font-weight: 500;
		color: #888;
		background: none;
		border: 1px solid rgba(255, 255, 255, 0.1);
		padding: var(--space-1) var(--space-3);
		border-radius: 6px;
		cursor: pointer;
		font-family: var(--font-sans);
		transition: all 0.2s;
	}
	.code-block-copy:hover {
		color: #fff;
		border-color: rgba(255, 255, 255, 0.3);
	}
	.code-block {
		padding: var(--space-5);
		overflow-x: auto;
	}
	.code-block pre {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		line-height: var(--leading-relaxed);
		color: #e5e5e7;
		margin: 0;
	}
	.code-block :global(.kw) {
		color: #e6391e;
		font-weight: 600;
	}
	.code-block :global(.fn) {
		color: #7ec8e3;
	}
	.code-block :global(.str) {
		color: #98c379;
	}
	.code-block :global(.cm) {
		color: #6a6a6a;
		font-style: italic;
	}
	.code-block :global(.num) {
		color: #d19a66;
	}
	.code-block :global(.ty) {
		color: #c678dd;
	}
	.code-block :global(.mac) {
		color: #e5c07b;
	}
</style>