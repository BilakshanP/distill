<script lang="ts">
	import { api, type Question } from '$lib/api';
	import { onMount } from 'svelte';

	let questions = $state<Question[]>([]);
	let nextCursor = $state<string | null>(null);
	let loading = $state(true);

	onMount(async () => {
		try {
			const page = await api.listQuestions();
			questions = page.data;
			nextCursor = page.next_cursor;
		} catch (e) {
			console.error(e);
		} finally {
			loading = false;
		}
	});

	async function loadMore() {
		if (!nextCursor) return;
		loading = true;
		try {
			const page = await api.listQuestions(20, nextCursor);
			questions = [...questions, ...page.data];
			nextCursor = page.next_cursor;
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head><title>Distill</title></svelte:head>

<div class="space-y-4">
	<h1 class="text-2xl font-bold text-gray-900">Questions</h1>

	{#if loading && questions.length === 0}
		<p class="text-gray-500">Loading...</p>
	{/if}

	{#each questions as q}
		<a href="/questions/{q.id}" class="block p-4 bg-white rounded-lg border border-gray-200 hover:border-blue-300 transition-colors">
			<h2 class="text-lg font-medium text-gray-900">{q.title}</h2>
			<p class="text-sm text-gray-600 mt-1 line-clamp-2">{q.body}</p>
			{#if q.tags.length > 0}
				<div class="flex gap-2 mt-2">
					{#each q.tags as tag}
						<span class="text-xs bg-blue-100 text-blue-700 px-2 py-0.5 rounded">{tag}</span>
					{/each}
				</div>
			{/if}
		</a>
	{/each}

	{#if nextCursor}
		<button onclick={loadMore} disabled={loading} class="w-full py-2 text-sm text-blue-600 hover:text-blue-800 disabled:text-gray-400">
			{loading ? 'Loading...' : 'Load more'}
		</button>
	{/if}
</div>
