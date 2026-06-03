<script lang="ts">
	import { api, type SearchResult } from '$lib/api';

	let query = $state('');
	let results = $state<SearchResult[]>([]);
	let searched = $state(false);
	let loading = $state(false);

	async function doSearch() {
		if (!query.trim()) return;
		loading = true;
		searched = true;
		try {
			results = await api.search(query);
		} catch (e) {
			console.error(e);
			results = [];
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head><title>Search - Distill</title></svelte:head>

<div class="space-y-4">
	<h1 class="text-2xl font-bold text-gray-900">Search</h1>

	<form onsubmit={(e) => { e.preventDefault(); doSearch(); }} class="flex gap-2">
		<input bind:value={query} type="text" placeholder="Search questions..." class="flex-1 border border-gray-300 rounded-lg px-4 py-2 text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent" />
		<button type="submit" disabled={loading} class="px-4 py-2 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 disabled:bg-gray-300">
			{loading ? '...' : 'Search'}
		</button>
	</form>

	{#if searched && results.length === 0 && !loading}
		<p class="text-gray-500 text-sm">No results found.</p>
	{/if}

	{#each results as r}
		<a href="/questions/{r.id}" class="block p-4 bg-white rounded-lg border border-gray-200 hover:border-blue-300 transition-colors">
			<div class="flex items-center gap-2">
				<span class="text-xs text-gray-400 font-mono">{r.score.toFixed(3)}</span>
				<h2 class="text-lg font-medium text-gray-900">{r.title}</h2>
			</div>
			<p class="text-sm text-gray-600 mt-1 line-clamp-2">{r.body}</p>
			{#if r.tags.length > 0}
				<div class="flex gap-2 mt-2">
					{#each r.tags as tag}
						<span class="text-xs bg-blue-100 text-blue-700 px-2 py-0.5 rounded">{tag}</span>
					{/each}
				</div>
			{/if}
		</a>
	{/each}
</div>
