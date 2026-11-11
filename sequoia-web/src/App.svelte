<script>
	import './app.css';
	import { nav } from './lib/navigation.svelte.js';
	import HomePage from './lib/pages/HomePage.svelte';
	import DocsPage from './lib/pages/DocsPage.svelte';
	import BlogPage from './lib/pages/BlogPage.svelte';
	import BenchmarksPage from './lib/pages/BenchmarksPage.svelte';

	$effect(() => {
		void nav.page;
		if (typeof window !== 'undefined') window.scrollTo(0, 0);
	});
</script>

{#if nav.page === 'home'}
	<HomePage />
{:else if nav.page === 'docs'}
	<DocsPage />
{:else if nav.page === 'blog'}
	<BlogPage />
{:else if nav.page === 'benchmarks'}
	<BenchmarksPage />
{/if}