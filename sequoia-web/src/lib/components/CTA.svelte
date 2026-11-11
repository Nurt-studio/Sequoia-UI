<script>
	import { reveal } from '../actions/reveal';
	import { nav } from '../navigation.svelte.js';
</script>

<section class="cta-section" use:reveal>
	<span class="section-number">07</span>
	<h2 class="section-title">Read the book, run an example</h2>
	<p class="section-description">
		SequoiaUI is pre-1.0. The documentation and the runnable examples in the repository are the best place to start.
	</p>
	<div class="cta-buttons">
		<a
			href="/"
			class="btn btn-white"
			onclick={(e) => {
				e.preventDefault();
				nav.page = 'docs';
			}}>Get Started</a
		>
		<a href="https://github.com/Nurt-studio/Sequoia-UI" class="btn btn-ghost" target="_blank" rel="noreferrer">View on GitHub</a>
	</div>
</section>

<style>
	.cta-section {
		background: #1a1a1a;
		padding: var(--section-py) var(--section-px);
		text-align: center;
	}

	.cta-section :global(.section-number) {
		color: var(--accent);
	}
	.cta-section :global(.section-number::after) {
		background-color: var(--accent);
		margin: var(--space-2) auto 0;
	}

	.cta-section :global(.section-title) {
		color: #ffffff;
		margin: var(--space-4) auto;
		max-width: 700px;
	}

	.cta-section :global(.section-description) {
		color: #a0a0a0;
		margin: 0 auto var(--space-6);
		max-width: 500px;
	}

	.cta-buttons {
		display: flex;
		gap: var(--space-4);
		justify-content: center;
	}

	@media (max-width: 768px) {
		.cta-buttons {
			flex-direction: column;
			align-items: center;
		}
	}
</style>