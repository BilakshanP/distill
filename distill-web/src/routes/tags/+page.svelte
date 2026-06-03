<script lang="ts">
	import { api } from '$lib/api';
	import { onMount } from 'svelte';

	let tags = $state<{ tag: string; count: number }[]>([]);
	let loading = $state(true);

	onMount(async () => {
		try {
			tags = await api.listTags();
		} catch (e) {
			console.error(e);
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head><title>Tags - Distill</title></svelte:head>

<div class="space-y-4">
	<h1 class="text-2xl font-bold text-gray-900">Tags</h1>

	{#if loading}
		<p class="text-gray-500">Loading...</p>
	{:else if tags.length === 0}
		<p class="text-gray-500 text-sm">No tags yet.</p>
	{:else}
		<div class="flex flex-wrap gap-3">
			{#each tags as t}
				<span class="inline-flex items-center gap-1 px-3 py-1.5 bg-blue-100 text-blue-700 rounded-lg text-sm">
					{t.tag}
					<span class="text-xs text-blue-500">({t.count})</span>
				</span>
			{/each}
		</div>
	{/if}
</div>
