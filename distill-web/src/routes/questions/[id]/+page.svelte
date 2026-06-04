<script lang="ts">
	import { page } from '$app/stores';
	import { api, type Question, type Answer, type Discussion, type IndividualAnswer, isLoggedIn, getUserId } from '$lib/api';
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Separator } from '$lib/components/ui/separator';
	import { Textarea } from '$lib/components/ui/textarea';
	import Markdown from '$lib/components/Markdown.svelte';

	let question = $state<Question | null>(null);
	let wikiAnswer = $state<Answer | null>(null);
	let answers = $state<IndividualAnswer[]>([]);
	let discussions = $state<Discussion[]>([]);
	let newComment = $state('');
	let replyTo = $state<string | null>(null);
	let replyBody = $state('');
	let myRating = $state<number | null>(null);
	let newAnswerBody = $state('');
	let submittingAnswer = $state(false);
	let answerDiscussions = $state<Record<string, Discussion[]>>({});
	let showAnswerThreads = $state<Record<string, boolean>>({});
	let answerReplyBody = $state<Record<string, string>>({});
	let myAnswerRatings = $state<Record<string, number>>({});
	let collapsedAnswers = $state(false);
	let collapsedDiscussion = $state(false);
	let collapsedThreads = $state<Record<string, boolean>>({});
	let error = $state('');

	const id = $derived($page.params.id!);

	onMount(async () => {
		if (!id) return;
		try {
			question = await api.getQuestion(id);
			try { wikiAnswer = await api.getWikiAnswer(id); } catch {}
			answers = await api.listAnswers(id);
			// Populate existing ratings
			for (const a of answers) {
				if (a.your_score) myAnswerRatings[a.id] = a.your_score;
			}
			myAnswerRatings = { ...myAnswerRatings };
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

	async function submitAnswer() {
		if (!newAnswerBody.trim()) return;
		submittingAnswer = true;
		try {
			const a = await api.createAnswer(id, newAnswerBody);
			answers = [...answers, a];
			newAnswerBody = '';
		} catch (e: any) { error = e.message; }
		finally { submittingAnswer = false; }
	}

	async function rateIndividual(answerId: string, score: number) {
		try {
			if (myAnswerRatings[answerId] === score) {
				await api.deleteAnswerRating(answerId);
				delete myAnswerRatings[answerId];
			} else {
				const result = await api.rateAnswer(answerId, score);
				myAnswerRatings[answerId] = score;
				answers = answers.map(a => a.id === answerId ? { ...a, rating_avg: result.rating_avg, rating_count: result.rating_count } : a);
			}
			myAnswerRatings = { ...myAnswerRatings };
		} catch (e: any) { error = e.message; }
	}

	async function toggleAnswerThread(answerId: string) {
		showAnswerThreads[answerId] = !showAnswerThreads[answerId];
		showAnswerThreads = { ...showAnswerThreads };
		if (showAnswerThreads[answerId] && !answerDiscussions[answerId]) {
			try {
				answerDiscussions[answerId] = await api.listDiscussions(id, undefined, answerId);
				answerDiscussions = { ...answerDiscussions };
			} catch {}
		}
	}

	async function submitAnswerReply(answerId: string) {
		const body = answerReplyBody[answerId]?.trim();
		if (!body) return;
		try {
			const d = await api.createDiscussion(id, body, undefined, answerId);
			answerDiscussions[answerId] = [...(answerDiscussions[answerId] || []), d];
			answerDiscussions = { ...answerDiscussions };
			answerReplyBody[answerId] = '';
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

	<!-- Individual Answers -->
	<section class="space-y-4">
		<button class="text-lg font-semibold flex items-center gap-2" onclick={() => collapsedAnswers = !collapsedAnswers}>
			<span class="text-xs">{collapsedAnswers ? '▶' : '▼'}</span>
			{answers.length} {answers.length === 1 ? 'Answer' : 'Answers'}
		</button>

		{#if !collapsedAnswers}

		{#each answers as a}
			<Card.Root class={a.is_accepted ? 'border-green-500/50' : ''}>
				<Card.Header>
					<div class="flex items-center gap-2 text-xs text-muted-foreground">
						<span class="font-medium {a.author_role === 'admin' ? 'text-amber-600' : 'text-foreground'}">{a.author_name}</span>
						{#if a.author_role === 'admin'}<span class="text-amber-600">[admin]</span>{/if}
						{#if a.is_accepted}<Badge variant="default" class="text-[10px]">Accepted</Badge>{/if}
						{#if a.rating_count > 0}
							<span>★ {a.rating_avg?.toFixed(1)} ({a.rating_count})</span>
						{/if}
						<span>{new Date(a.created_at).toLocaleDateString()}</span>
					</div>
				</Card.Header>
				<Card.Content>
					<Markdown content={a.body} />
				</Card.Content>
				{#if isLoggedIn()}
					<Card.Footer class="flex flex-col gap-2">
						<div class="flex items-center gap-1 w-full">
							<span class="text-xs mr-1">Rate:</span>
							{#each [1, 2, 3, 4, 5] as score}
								<button
									class="px-2 py-0.5 text-xs border rounded transition-colors {myAnswerRatings[a.id] === score ? 'bg-primary text-primary-foreground border-primary' : 'border-border hover:border-primary/50'}"
									onclick={() => rateIndividual(a.id, score)}
								>{score}</button>
							{/each}
							{#if myAnswerRatings[a.id]}
								<span class="text-xs text-muted-foreground ml-2">Your rating: {myAnswerRatings[a.id]}/5</span>
							{/if}
							<button class="ml-auto text-xs text-muted-foreground hover:text-foreground" onclick={() => toggleAnswerThread(a.id)}>
								{showAnswerThreads[a.id] ? 'Hide' : 'Discuss'}
							</button>
						</div>
						{#if showAnswerThreads[a.id]}
							<div class="w-full border-t pt-2 space-y-2">
								{#each answerDiscussions[a.id] || [] as d}
									<div class="text-xs pl-2 border-l border-border">
										<span class="font-medium {d.author_role === 'admin' ? 'text-amber-600' : ''}">{d.author_name}</span>: {d.body}
									</div>
								{/each}
								<div class="flex gap-2">
									<input
										type="text"
										class="flex-1 text-xs border border-border rounded px-2 py-1"
										placeholder="Reply..."
										value={answerReplyBody[a.id] || ''}
										oninput={(e) => { answerReplyBody[a.id] = (e.target as HTMLInputElement).value; answerReplyBody = {...answerReplyBody}; }}
										onkeydown={(e) => { if (e.key === 'Enter') submitAnswerReply(a.id); }}
									/>
									<button class="text-xs px-2 py-1 bg-primary text-primary-foreground rounded" onclick={() => submitAnswerReply(a.id)}>Post</button>
								</div>
							</div>
						{/if}
					</Card.Footer>
				{/if}
			</Card.Root>
		{/each}

		{#if isLoggedIn()}
			<div class="space-y-2 pt-2">
				<h3 class="text-sm font-medium">Your Answer</h3>
				<Textarea bind:value={newAnswerBody} rows={4} placeholder="Write your answer (markdown)..." />
				<Button onclick={submitAnswer} disabled={submittingAnswer || !newAnswerBody.trim()}>
					{submittingAnswer ? 'Submitting...' : 'Post Answer'}
				</Button>
			</div>
		{/if}
		{/if}
	</section>

	<Separator class="my-8" />

	<!-- Threaded Discussion -->
	<section class="space-y-4">
		<button class="text-lg font-semibold flex items-center gap-2" onclick={() => collapsedDiscussion = !collapsedDiscussion}>
			<span class="text-xs">{collapsedDiscussion ? '▶' : '▼'}</span>
			Discussion ({discussions.length})
		</button>

		{#if !collapsedDiscussion}

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
				{#if node.children.length > 0}
					<button
						class="text-[10px] text-muted-foreground hover:text-foreground"
						onclick={() => { collapsedThreads[node.id] = !collapsedThreads[node.id]; collapsedThreads = {...collapsedThreads}; }}
					>{collapsedThreads[node.id] ? `[+${node.children.length} hidden]` : '[-]'}</button>
				{/if}
				{#if !collapsedThreads[node.id]}
					{#each node.children as child}
						{@render threadNode(child)}
					{/each}
				{/if}
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
		{/if}
	</section>
{:else if !error}
	<p class="text-muted-foreground text-sm">Loading...</p>
{/if}
