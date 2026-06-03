<script lang="ts">
	import { goto } from '$app/navigation';
	import { api, isLoggedIn } from '$lib/api';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Textarea } from '$lib/components/ui/textarea';
	import * as Card from '$lib/components/ui/card';

	let title = $state('');
	let body = $state('');
	let tags = $state('');
	let aiAnswer = $state(false);
	let submitting = $state(false);
	let error = $state('');

	async function submit() {
		if (!title.trim() || !body.trim()) return;
		submitting = true;
		error = '';
		try {
			const tagList = tags.split(',').map(t => t.trim()).filter(Boolean);
			const q = await api.createQuestion(title, body, tagList, aiAnswer);
			goto(`/questions/${q.id}`);
		} catch (e: any) {
			error = e.message;
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head><title>Ask a Question - Distill</title></svelte:head>

<div class="max-w-2xl mx-auto space-y-6">
	<h1 class="text-2xl font-bold tracking-tight">Ask a Question</h1>

	{#if !isLoggedIn()}
		<Card.Root>
			<Card.Content class="py-8 text-center">
				<p class="text-muted-foreground mb-4">You need to be logged in to ask a question.</p>
				<Button href="/login">Login</Button>
			</Card.Content>
		</Card.Root>
	{:else}
		{#if error}
			<p class="text-destructive text-sm">{error}</p>
		{/if}

		<form onsubmit={(e) => { e.preventDefault(); submit(); }} class="space-y-4">
			<div class="space-y-2">
				<label for="title" class="text-sm font-medium">Title</label>
				<Input id="title" bind:value={title} placeholder="What's your question?" />
			</div>

			<div class="space-y-2">
				<label for="body" class="text-sm font-medium">Body</label>
				<Textarea id="body" bind:value={body} rows={6} placeholder="Provide details..." />
			</div>

			<div class="space-y-2">
				<label for="tags" class="text-sm font-medium">Tags</label>
				<Input id="tags" bind:value={tags} placeholder="rust, axum, database (comma-separated)" />
			</div>

			<label class="flex items-center gap-2 text-sm">
				<input type="checkbox" bind:checked={aiAnswer} class="rounded border-border" />
				<span>Generate AI answer</span>
			</label>

			<Button type="submit" disabled={submitting || !title.trim() || !body.trim()}>
				{submitting ? 'Submitting...' : 'Submit Question'}
			</Button>
		</form>
	{/if}
</div>
