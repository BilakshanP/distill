<script lang="ts">
	import { page } from '$app/stores';
	import { api, type Question, type Answer, type Discussion, isLoggedIn, getUserId } from '$lib/api';
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Separator } from '$lib/components/ui/separator';
	import { Textarea } from '$lib/components/ui/textarea';
	import Markdown from '$lib/components/Markdown.svelte';

	let question = $state<Question | null>(null);
	let wikiAnswer = $state<Answer | null>(null);
	let discussions = $state<Discussion[]>([]);
	let newComment = $state('');
	let replyTo = $state<string | null>(null);
	let replyBody = $state('');
	let myRating = $state<number | null>(null);
	let error = $state('');

	const id = $derived($page.params.id!);

	onMount(async () => {
		if (!id) return;
		try {
			question = await api.getQuestion(id);
			try { wikiAnswer = await api.getWikiAnswer(id); } catch {}
			discussions = await api.listDiscussions(id);

			// Fetch own rating if logged in
			if (isLoggedIn() && wikiAnswer) {
				try {
					const ratings = await api.getWikiRatings(wikiAnswer.id);
					const uid = getUserId();
					const mine = ratings.find(r => r.rater_id === uid);
					if (mine) myRating = mine.score;
				} catch {}
			}
		} catch (e: any) {
			error = e.message;
		}
	});

	async function rateAnswer(score: number) {
		if (!wikiAnswer) return;
		try {
			if (myRating === score) {
				await api.deleteWikiRating(wikiAnswer.id);
				myRating = null;
			} else {
				await api.rateWikiAnswer(wikiAnswer.id, score);
				myRating = score;
			}
			// Refresh answer to get updated stats
			wikiAnswer = await api.getWikiAnswer(id);
		} catch (e: any) { error = e.message; }
	}

	async function submitComment() {
		if (!newComment.trim()) return;
		try {
			const d = await api.createDiscussion(id, newComment);
			discussions = [...discussions, d];
			newComment = '';
		} catch (e: any) { error = e.message; }
	}

	async function submitReply() {
		if (!replyBody.trim() || !replyTo) return;
		try {
			const d = await api.createDiscussion(id, replyBody, replyTo);
			discussions = [...discussions, d];
			replyBody = '';
			replyTo = null;
		} catch (e: any) { error = e.message; }
	}

	async function vote(discussionId: string, direction: number) {
		try {
			const result = await api.voteDiscussion(discussionId, direction);
			discussions = discussions.map(d =>
				d.id === discussionId ? { ...d, score: result.score, user_vote: result.user_vote } : d
			);
		} catch (e: any) { error = e.message; }
	}

	// Build tree from flat list
	function buildTree(items: Discussion[]): (Discussion & { children: Discussion[] })[] {
		const map = new Map<string, Discussion & { children: Discussion[] }>();
		const roots: (Discussion & { children: Discussion[] })[] = [];
		for (const item of items) {
			map.set(item.id, { ...item, children: [] });
		}
		for (const item of items) {
			const node = map.get(item.id)!;
			if (item.parent_id && map.has(item.parent_id)) {
				map.get(item.parent_id)!.children.push(node);
			} else {
				roots.push(node);
			}
		}
		const sortByScore = (a: any, b: any) => b.score - a.score;
		const sortTree = (nodes: any[]) => { nodes.sort(sortByScore); nodes.forEach(n => sortTree(n.children)); };
		sortTree(roots);
		return roots;
	}

	const tree = $derived(buildTree(discussions));
</script>

<svelte:head><title>{question?.title || 'Question'} - Distill</title></svelte:head>

{#if error}
	<p class="text-destructive text-sm mb-4">{error}</p>
{/if}

{#if question}
	<article class="space-y-4">
		<h1 class="text-2xl font-bold tracking-tight">{question.title}</h1>
		<Markdown content={question.body} />
		{#if question.tags.length > 0}
			<div class="flex gap-1.5 flex-wrap">
				{#each question.tags as tag}
					<Badge variant="secondary">{tag}</Badge>
				{/each}
			</div>
		{/if}
	</article>

	<Separator class="my-8" />

	<!-- Wiki Answer -->
	<section class="space-y-4">
		<div class="flex items-center justify-between">
			<h2 class="text-lg font-semibold">Answer</h2>
			{#if isLoggedIn()}
				<Button variant="outline" size="sm" href="/questions/{id}/edit">
					{wikiAnswer ? 'Edit' : 'Write Answer'}
				</Button>
			{/if}
		</div>

		{#if wikiAnswer}
			<Card.Root>
				<Card.Content class="pt-6">
					<Markdown content={wikiAnswer.body} />
				</Card.Content>
				<Card.Footer class="text-xs text-muted-foreground flex flex-col gap-2">
					<div class="flex justify-between w-full">
						<span>
							{#if wikiAnswer.last_editor_name}
								Edited by
								<span class={wikiAnswer.last_editor_role === 'admin' ? 'text-amber-600 font-medium' : 'font-medium'}>{wikiAnswer.last_editor_name}</span>
								{#if wikiAnswer.last_editor_role === 'admin'}<span class="text-amber-600">[admin]</span>{/if}
							{/if}
							 — {new Date(wikiAnswer.updated_at).toLocaleDateString()}
						</span>
						<a href="/questions/{id}/history" class="hover:text-foreground">View history</a>
					</div>
					{#if wikiAnswer.rating_count > 0}
						<div class="flex gap-4 w-full">
							<span>Lifetime: ★ {wikiAnswer.rating_avg?.toFixed(1)} ({wikiAnswer.rating_count} ratings)</span>
							{#if wikiAnswer.rating_count_since_edit > 0}
								<span>Since last edit: ★ {wikiAnswer.rating_avg_since_edit?.toFixed(1)} ({wikiAnswer.rating_count_since_edit})</span>
							{/if}
						</div>
					{/if}
					{#if isLoggedIn()}
						<div class="flex items-center gap-1 w-full pt-1">
							<span class="text-xs mr-1">Rate:</span>
							{#each [1, 2, 3, 4, 5] as score}
								<button
									class="px-2 py-0.5 text-xs border rounded transition-colors {myRating === score ? 'bg-primary text-primary-foreground border-primary' : 'border-border hover:border-primary/50'}"
									onclick={() => rateAnswer(score)}
								>{score}</button>
							{/each}
							{#if myRating}
								<span class="text-xs ml-2">Your rating: {myRating}/5</span>
							{/if}
						</div>
					{/if}
				</Card.Footer>
			</Card.Root>
		{:else}
			<p class="text-muted-foreground text-sm">No answer yet. Be the first to contribute!</p>
		{/if}
	</section>

	<Separator class="my-8" />

	<!-- Threaded Discussion -->
	<section class="space-y-4">
		<h2 class="text-lg font-semibold">Discussion ({discussions.length})</h2>

		{#snippet threadNode(node: any)}
			{@const isOwn = node.author_id === getUserId()}
			<div class="border-l-2 {isOwn ? 'border-primary/50 bg-primary/5' : 'border-border'} pl-4 py-2 rounded-r" style="margin-left: {node.depth * 16}px">
				<div class="flex items-center gap-2 text-xs text-muted-foreground mb-1">
					<div class="flex items-center gap-1">
						{#if isLoggedIn()}
							<button
								class="hover:text-foreground {node.user_vote === 1 ? 'text-green-500' : ''}"
								onclick={() => vote(node.id, 1)}>▲</button>
						{/if}
						<span class="font-mono font-medium {node.score > 0 ? 'text-green-600' : node.score < 0 ? 'text-red-500' : ''}">{node.score}</span>
						{#if isLoggedIn()}
							<button
								class="hover:text-foreground {node.user_vote === -1 ? 'text-red-500' : ''}"
								onclick={() => vote(node.id, -1)}>▼</button>
						{/if}
					</div>
					<span class="font-medium {node.author_role === 'admin' ? 'text-amber-600' : 'text-foreground'}">{node.author_name}</span>
					{#if node.author_role === 'admin'}<span class="text-amber-600 text-[10px]">[admin]</span>{/if}
					{#if isOwn}<span class="text-primary text-[10px]">(you)</span>{/if}
					<span>{new Date(node.created_at).toLocaleDateString()}</span>
				</div>
				<Markdown content={node.body} />
				{#if isLoggedIn()}
					<button
						class="text-xs text-muted-foreground hover:text-foreground mt-1"
						onclick={() => { replyTo = replyTo === node.id ? null : node.id; replyBody = ''; }}
					>reply</button>
				{/if}
				{#if replyTo === node.id}
					<div class="mt-2 space-y-2">
						<Textarea bind:value={replyBody} rows={2} placeholder="Reply..." />
						<div class="flex gap-2">
							<Button size="sm" onclick={submitReply} disabled={!replyBody.trim()}>Reply</Button>
							<Button size="sm" variant="outline" onclick={() => replyTo = null}>Cancel</Button>
						</div>
					</div>
				{/if}
				{#each node.children as child}
					{@render threadNode(child)}
				{/each}
			</div>
		{/snippet}

		{#each tree as node}
			{@render threadNode(node)}
		{/each}

		{#if isLoggedIn()}
			<div class="mt-4 space-y-2">
				<Textarea bind:value={newComment} rows={3} placeholder="Add to the discussion..." />
				<Button onclick={submitComment} disabled={!newComment.trim()}>Post</Button>
			</div>
		{/if}
	</section>
{:else if !error}
	<p class="text-muted-foreground text-sm">Loading...</p>
{/if}
