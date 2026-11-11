<script>
	import { softReveal } from '../actions/softReveal';
	import { nav } from '../navigation.svelte.js';
	import heroImage from '../../assets/vision-design.png';
</script>

<section class="section hero" use:softReveal>
	<div class="hero-content">
		<span class="section-number">01</span>
		<p class="section-label">Introduction</p>
		<h1 class="section-title">A native, reactive GUI framework for Rust</h1>
		<p class="section-description">Built on wgpu with a retained view tree and fine-grained signals. No virtual DOM, no web view, no JavaScript runtime.</p>
		<div class="hero-actions">
			<a
				href="/"
				class="btn btn-outline"
				onclick={(e) => {
					e.preventDefault();
					nav.page = 'docs';
				}}>Read the documentation</a
			>
			<a
				class="hero-book-link"
				href="https://nurt-studio.github.io/Sequoia-UI/"
				target="_blank"
				rel="noreferrer">Or read the full book →</a
			>
		</div>
	</div>

	<div class="hero-visual">
		<div class="hero-glow" aria-hidden="true"></div>
		<div class="hero-frame">
			<img src={heroImage} alt="The Vision Design editor, built with SequoiaUI" />
		</div>
	</div>
</section>

<style>
	.hero {
		display: grid;
		/* Obrazek dostaje teraz więcej miejsca niż tekst */
		grid-template-columns: minmax(0, 0.9fr) minmax(0, 1.35fr);
		gap: var(--section-gap);
		align-items: center;
		min-height: 78vh;
	}

	.hero-content {
		max-width: 480px;
	}

	.hero-actions {
		display: flex;
		align-items: center;
		gap: var(--space-5);
		flex-wrap: wrap;
	}

	.hero-book-link {
		font-size: var(--text-sm);
		font-weight: 600;
		color: #555;
		text-decoration: none;
		transition: color 0.2s ease;
	}
	.hero-book-link:hover {
		color: #e6391e;
	}

	.hero-visual {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		/* Delikatne wyjście poza prawą krawędź sekcji na dużych ekranach */
		margin-right: -40px;
	}


	.hero-frame {
		position: relative;
		z-index: 1;
		width: 100%;
		max-width: 760px;          /* ⬅ było 560px */
		border-radius: 12px;
		overflow: hidden;
		background: #fff;
		border: 1px solid rgba(0, 0, 0, 0.06);
		box-shadow:
			0 32px 80px -16px rgba(0, 0, 0, 0.22),
			0 12px 32px -12px rgba(0, 0, 0, 0.12);
	}

	.hero-frame img {
		display: block;
		width: 100%;
		height: auto;
	}

	@keyframes float {
		0%, 100% { transform: translateY(0); }
		50%      { transform: translateY(-14px); }
	}

	@keyframes pulse {
		0%, 100% { opacity: 0.7; transform: scale(1); }
		50%      { opacity: 1;   transform: scale(1.08); }
	}

	/* Tablety / wąskie desktopy — wracamy do wyśrodkowania */
	@media (max-width: 1100px) {
		.hero {
			grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		}
		.hero-visual {
			margin-right: 0;
		}
		.hero-frame {
			max-width: 100%;
		}
	}

	/* Mobile — stack + wyśrodkowanie */
	@media (max-width: 900px) {
		.hero {
			grid-template-columns: 1fr;
			gap: var(--space-7);
			min-height: auto;
			text-align: center;
		}

		.hero-content {
			max-width: 100%;
			margin: 0 auto;
		}

		.hero-actions {
			justify-content: center;
		}

		.hero :global(.section-number::after) {
			margin: 6px auto 0;
		}

		.hero :global(.section-title),
		.hero :global(.section-description) {
			margin-left: auto;
			margin-right: auto;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.hero-frame,
		.hero-glow {
			animation: none;
		}
	}
</style>