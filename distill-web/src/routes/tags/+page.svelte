<script lang="ts">
	import { api } from '$lib/api';
	import { onMount } from 'svelte';
	import { Badge } from '$lib/components/ui/badge';

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

<div class="space-y-6">
	<h1 class="text-2xl font-bold tracking-tight">Tags</h1>

	{#if loading}
		<p class="text-muted-foreground text-sm">Loading...</p>
	{:else if tags.length === 0}
		<p class="text-muted-foreground text-sm">No tags yet.</p>
	{:else}
		<div class="flex flex-wrap gap-2">
			{#each tags as t}
				<Badge variant="outline" class="text-sm py-1 px-3">
					{t.tag} <span class="ml-1 text-muted-foreground">({t.count})</span>
				</Badge>
			{/each}
		</div>
	{/if}
</div>
