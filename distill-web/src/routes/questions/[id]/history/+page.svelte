<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import Markdown from '$lib/components/Markdown.svelte';

	type Revision = { id: string; editor_id: string; edit_message: string | null; created_at: string };
	type RevisionDetail = { id: string; editor_id: string; body: string; diff: string; edit_message: string | null; created_at: string };

	const questionId = $derived($page.params.id!);
	let revisions = $state<Revision[]>([]);
	let current = $state<RevisionDetail | null>(null);
	let currentIdx = $state(0);
	let loading = $state(true);

	onMount(async () => {
		try {
			revisions = await api.getWikiHistory(questionId);
			if (revisions.length > 0) {
				await loadRevision(0);
			}
		} catch (e) {
			console.error(e);
		} finally {
			loading = false;
		}
	});

	async function loadRevision(idx: number) {
		if (idx < 0 || idx >= revisions.length) return;
		currentIdx = idx;
		current = await api.getRevision(revisions[idx].id);
	}

	function formatDiff(diff: string): string {
		return diff
			.split('\n')
			.map(line => {
				if (line.startsWith('+') && !line.startsWith('+++')) return `<span class="text-green-600">${escape(line)}</span>`;
				if (line.startsWith('-') && !line.startsWith('---')) return `<span class="text-red-500">${escape(line)}</span>`;
				if (line.startsWith('@@')) return `<span class="text-blue-500">${escape(line)}</span>`;
				return escape(line);
			})
			.join('\n');
	}

	function escape(s: string) {
		return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
	}
</script>

<svelte:head><title>Edit History - Distill</title></svelte:head>

<div class="space-y-4">
	<div class="flex items-center justify-between">
		<h1 class="text-xl font-bold tracking-tight">Edit History</h1>
		<Button variant="outline" size="sm" href="/questions/{questionId}">← Back to question</Button>
	</div>

	{#if loading}
		<p class="text-muted-foreground text-sm">Loading...</p>
	{:else if revisions.length === 0}
		<p class="text-muted-foreground text-sm">No edit history yet.</p>
	{:else}
		<div class="grid grid-cols-[250px_1fr] gap-4">
			<!-- Revision list -->
			<div class="space-y-1 border-r border-border pr-4">
				{#each revisions as rev, i}
					<button
						class="w-full text-left px-3 py-2 rounded text-xs hover:bg-muted transition-colors {i === currentIdx ? 'bg-muted font-medium' : ''}"
						onclick={() => loadRevision(i)}
					>
						<div class="truncate">{rev.edit_message || 'Untitled edit'}</div>
						<div class="text-muted-foreground">{new Date(rev.created_at).toLocaleString()}</div>
					</button>
				{/each}
			</div>

			<!-- Revision detail -->
			<div class="space-y-4">
				{#if current}
					<!-- Nav -->
					<div class="flex items-center justify-between">
						<Button variant="outline" size="sm" disabled={currentIdx >= revisions.length - 1} onclick={() => loadRevision(currentIdx + 1)}>← Older</Button>
						<span class="text-xs text-muted-foreground">
							Revision {revisions.length - currentIdx} of {revisions.length}
						</span>
						<Button variant="outline" size="sm" disabled={currentIdx <= 0} onclick={() => loadRevision(currentIdx - 1)}>Newer →</Button>
					</div>

					<!-- Diff -->
					<Card.Root>
						<Card.Header>
							<Card.Title class="text-sm">{current.edit_message || 'Untitled edit'}</Card.Title>
							<Card.Description>{new Date(current.created_at).toLocaleString()}</Card.Description>
						</Card.Header>
						<Card.Content>
							<pre class="text-xs font-mono whitespace-pre-wrap bg-muted p-4 rounded overflow-x-auto">{@html formatDiff(current.diff)}</pre>
						</Card.Content>
					</Card.Root>

					<!-- Full content at this revision -->
					<Card.Root>
						<Card.Header>
							<Card.Title class="text-sm">Content at this revision</Card.Title>
						</Card.Header>
						<Card.Content>
							<Markdown content={current.body} />
						</Card.Content>
					</Card.Root>
				{/if}
			</div>
		</div>
	{/if}
</div>
