<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { api, isLoggedIn } from '$lib/api';
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Textarea } from '$lib/components/ui/textarea';
	import Markdown from '$lib/components/Markdown.svelte';

	const questionId = $derived($page.params.id!);
	let body = $state('');
	let editMessage = $state('');
	let saving = $state(false);
	let error = $state('');
	let preview = $state(false);

	onMount(async () => {
		if (!isLoggedIn()) { goto(`/questions/${questionId}`); return; }
		try {
			const answer = await api.getWikiAnswer(questionId);
			body = answer.body;
		} catch {
			// No existing answer, start fresh
		}
	});

	async function save() {
		if (!body.trim()) return;
		saving = true;
		error = '';
		try {
			await api.editWikiAnswer(questionId, body, editMessage || undefined);
			goto(`/questions/${questionId}`);
		} catch (e: any) {
			error = e.message;
		} finally {
			saving = false;
		}
	}
</script>

<svelte:head><title>Edit Answer - Distill</title></svelte:head>

<div class="space-y-4">
	<div class="flex items-center justify-between">
		<h1 class="text-xl font-bold tracking-tight">Edit Answer</h1>
		<Button variant="outline" size="sm" href="/questions/{questionId}">Cancel</Button>
	</div>

	{#if error}
		<p class="text-destructive text-sm">{error}</p>
	{/if}

	<div class="flex gap-2 text-sm">
		<button class="px-3 py-1 rounded {!preview ? 'bg-muted font-medium' : ''}" onclick={() => preview = false}>Write</button>
		<button class="px-3 py-1 rounded {preview ? 'bg-muted font-medium' : ''}" onclick={() => preview = true}>Preview</button>
	</div>

	{#if preview}
		<div class="border border-border rounded-lg p-4 min-h-[200px]">
			<Markdown content={body} />
		</div>
	{:else}
		<Textarea bind:value={body} rows={15} placeholder="Write your answer (markdown supported)..." class="font-mono text-sm" />
	{/if}

	<div class="flex items-center gap-3">
		<Input bind:value={editMessage} placeholder="Edit summary (e.g. 'Fixed code example')" class="flex-1" />
		<Button onclick={save} disabled={saving || !body.trim()}>
			{saving ? 'Saving...' : 'Save'}
		</Button>
	</div>
</div>
