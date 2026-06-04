<script lang="ts">
	import { page } from '$app/stores';
	import { api, type Question, type Answer, isLoggedIn, getUserId, setUserId } from '$lib/api';
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Separator } from '$lib/components/ui/separator';
	import { Textarea } from '$lib/components/ui/textarea';
	import Markdown from '$lib/components/Markdown.svelte';

	let question = $state<Question | null>(null);
	let answers = $state<Answer[]>([]);
	let newAnswer = $state('');
	let submitting = $state(false);
	let error = $state('');
	let myRatings = $state<Record<string, number>>({});

	const id = $derived($page.params.id!);

	onMount(async () => {
		if (!id) return;
		try {
			const [q, a] = await Promise.all([api.getQuestion(id), api.getAnswers(id)]);
			question = q;
			answers = a;

			// Fetch user info and existing ratings
			if (isLoggedIn()) {
				try {
					const me = await api.getMe();
					setUserId(me.id);
				} catch {}

				const userId = getUserId();
				if (userId) {
					for (const ans of a) {
						try {
							const r = await api.getRatings(ans.id);
							const mine = r.data.find(x => x.rater_id === userId);
							if (mine) myRatings[ans.id] = mine.score;
						} catch {}
					}
					myRatings = { ...myRatings };
				}
			}
		} catch (e: any) {
			error = e.message;
		}
	});

	async function submitAnswer() {
		if (!newAnswer.trim()) return;
		submitting = true;
		try {
			const a = await api.createAnswer(id, newAnswer);
			answers = [...answers, a];
			newAnswer = '';
		} catch (e: any) {
			error = e.message;
		} finally {
			submitting = false;
		}
	}

	async function rate(answerId: string, score: number) {
		try {
			if (myRatings[answerId] === score) {
				// Same score = unrate
				await api.deleteRating(answerId);
				delete myRatings[answerId];
				myRatings = { ...myRatings };
			} else {
				// New or different score = upsert
				await api.rateAnswer(answerId, score);
				myRatings[answerId] = score;
				myRatings = { ...myRatings };
			}
		} catch (e: any) {
			error = e.message;
		}
	}
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
		<p class="text-xs text-muted-foreground">{new Date(question.created_at).toLocaleDateString()}</p>
	</article>

	<Separator class="my-8" />

	<section class="space-y-4">
		<h2 class="text-lg font-semibold">{answers.length} {answers.length === 1 ? 'Answer' : 'Answers'}</h2>

		{#each answers as a}
			<Card.Root>
				<Card.Header>
					<div class="flex items-center gap-2 text-xs text-muted-foreground">
						<Badge variant={a.author_type === 'ai' ? 'default' : 'outline'}>{a.author_type}</Badge>
						{#if a.is_stale}<Badge variant="destructive">stale</Badge>{/if}
						{#if a.rating_avg}
							<span>★ {a.rating_avg.toFixed(1)} ({a.rating_count})</span>
						{:else if a.rating_positive_pct !== null}
							<span>↑{Math.round(a.rating_positive_pct)}% ({a.rating_count})</span>
						{/if}
						{#if a.comment_count > 0}
							<span>{a.comment_count} comments</span>
						{/if}
					</div>
				</Card.Header>
				<Card.Content>
					<Markdown content={a.body} />
				</Card.Content>
				{#if isLoggedIn()}
					<Card.Footer>
						<div class="flex gap-1 items-center">
							<span class="text-xs text-muted-foreground mr-1">Rate:</span>
							{#each [1, 2, 3, 4, 5] as score}
								<Button
									variant={myRatings[a.id] === score ? 'default' : 'outline'}
									size="sm"
									onclick={() => rate(a.id, score)}
								>
									{score}
								</Button>
							{/each}
							{#if myRatings[a.id]}
								<span class="text-xs text-muted-foreground ml-2">Your rating: {myRatings[a.id]}/5</span>
							{/if}
						</div>
					</Card.Footer>
				{/if}
			</Card.Root>
		{/each}
	</section>

	{#if isLoggedIn()}
		<Separator class="my-8" />
		<div class="space-y-3">
			<h3 class="text-sm font-medium">Your Answer</h3>
			<Textarea bind:value={newAnswer} rows={4} placeholder="Write your answer..." />
			<Button onclick={submitAnswer} disabled={submitting || !newAnswer.trim()}>
				{submitting ? 'Submitting...' : 'Submit Answer'}
			</Button>
		</div>
	{/if}
{:else if !error}
	<p class="text-muted-foreground text-sm">Loading...</p>
{/if}
