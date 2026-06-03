<script lang="ts">
	import { page } from '$app/stores';
	import { api, type Question, type Answer, isLoggedIn } from '$lib/api';
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Separator } from '$lib/components/ui/separator';
	import { Textarea } from '$lib/components/ui/textarea';

	let question = $state<Question | null>(null);
	let answers = $state<Answer[]>([]);
	let newAnswer = $state('');
	let submitting = $state(false);
	let error = $state('');

	const id = $derived($page.params.id!);

	onMount(async () => {
		if (!id) return;
		try {
			const [q, a] = await Promise.all([api.getQuestion(id), api.getAnswers(id)]);
			question = q;
			answers = a;
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
			await api.rateAnswer(answerId, score);
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
		<p class="text-muted-foreground text-sm whitespace-pre-wrap">{question.body}</p>
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
					<p class="text-sm whitespace-pre-wrap">{a.body}</p>
				</Card.Content>
				{#if isLoggedIn()}
					<Card.Footer>
						<div class="flex gap-1">
							{#each [1, 2, 3, 4, 5] as score}
								<Button variant="outline" size="sm" onclick={() => rate(a.id, score)}>
									{score}
								</Button>
							{/each}
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
