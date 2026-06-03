<script lang="ts">
	import { api, type Question } from '$lib/api';
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';

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

<div class="space-y-6">
	<div class="flex items-center justify-between">
		<h1 class="text-2xl font-bold tracking-tight">Questions</h1>
		<Button href="/ask" size="sm">Ask a question</Button>
	</div>

	{#if loading && questions.length === 0}
		<p class="text-muted-foreground text-sm">Loading...</p>
	{/if}

	<div class="space-y-3">
		{#each questions as q}
			<a href="/questions/{q.id}" class="block group">
				<Card.Root class="transition-colors group-hover:border-primary/30">
					<Card.Header>
						<Card.Title class="text-base">{q.title}</Card.Title>
						<Card.Description class="line-clamp-2">{q.body}</Card.Description>
					</Card.Header>
					{#if q.tags.length > 0}
						<Card.Content>
							<div class="flex gap-1.5 flex-wrap">
								{#each q.tags as tag}
									<Badge variant="secondary">{tag}</Badge>
								{/each}
							</div>
						</Card.Content>
					{/if}
				</Card.Root>
			</a>
		{/each}
	</div>

	{#if nextCursor}
		<div class="flex justify-center">
			<Button variant="outline" onclick={loadMore} disabled={loading}>
				{loading ? 'Loading...' : 'Load more'}
			</Button>
		</div>
	{/if}
</div>
