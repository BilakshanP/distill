<script lang="ts">
	import { api, type SearchResult } from '$lib/api';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';

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

<div class="space-y-6">
	<h1 class="text-2xl font-bold tracking-tight">Search</h1>

	<form onsubmit={(e) => { e.preventDefault(); doSearch(); }} class="flex gap-2">
		<Input bind:value={query} placeholder="Search questions..." class="flex-1" />
		<Button type="submit" disabled={loading}>
			{loading ? '...' : 'Search'}
		</Button>
	</form>

	{#if searched && results.length === 0 && !loading}
		<p class="text-muted-foreground text-sm">No results found.</p>
	{/if}

	<div class="space-y-3">
		{#each results as r}
			<a href="/questions/{r.id}" class="block group">
				<Card.Root class="transition-colors group-hover:border-primary/30">
					<Card.Header>
						<div class="flex items-center gap-2">
							<span class="text-xs font-mono text-muted-foreground">{r.score.toFixed(3)}</span>
							<Card.Title class="text-base">{r.title}</Card.Title>
						</div>
						<Card.Description class="line-clamp-2">{r.body}</Card.Description>
					</Card.Header>
					{#if r.tags.length > 0}
						<Card.Content>
							<div class="flex gap-1.5 flex-wrap">
								{#each r.tags as tag}
									<Badge variant="secondary">{tag}</Badge>
								{/each}
							</div>
						</Card.Content>
					{/if}
				</Card.Root>
			</a>
		{/each}
	</div>
</div>
