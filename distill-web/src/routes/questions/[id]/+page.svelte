<script lang="ts">
	import { page } from '$app/stores';
	import { api, type Question, type Answer, isLoggedIn } from '$lib/api';
	import { onMount } from 'svelte';

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
	<p class="text-red-600 text-sm mb-4">{error}</p>
{/if}

{#if question}
	<article class="space-y-4">
		<h1 class="text-2xl font-bold text-gray-900">{question.title}</h1>
		<div class="prose prose-sm max-w-none text-gray-700">{question.body}</div>
		{#if question.tags.length > 0}
			<div class="flex gap-2">
				{#each question.tags as tag}
					<span class="text-xs bg-blue-100 text-blue-700 px-2 py-0.5 rounded">{tag}</span>
				{/each}
			</div>
		{/if}
		<p class="text-xs text-gray-400">{new Date(question.created_at).toLocaleDateString()}</p>
	</article>

	<hr class="my-6 border-gray-200" />

	<section class="space-y-4">
		<h2 class="text-lg font-semibold text-gray-900">{answers.length} Answers</h2>

		{#each answers as a}
			<div class="p-4 bg-white rounded-lg border border-gray-200 space-y-2">
				<div class="flex items-center gap-2 text-xs text-gray-500">
					<span class="font-medium text-gray-700">[{a.author_type}]</span>
					{#if a.is_stale}<span class="text-orange-600">[STALE]</span>{/if}
					{#if a.rating_avg}
						<span>★ {a.rating_avg.toFixed(1)} ({a.rating_count})</span>
					{:else if a.rating_positive_pct !== null}
						<span>↑{Math.round(a.rating_positive_pct)}% ({a.rating_count})</span>
					{/if}
					{#if a.comment_count > 0}
						<span>{a.comment_count} comments</span>
					{/if}
				</div>
				<div class="prose prose-sm max-w-none text-gray-700 whitespace-pre-wrap">{a.body}</div>
				{#if isLoggedIn()}
					<div class="flex gap-1 pt-2">
						{#each [1, 2, 3, 4, 5] as score}
							<button onclick={() => rate(a.id, score)} class="text-xs px-2 py-1 border border-gray-200 rounded hover:bg-blue-50 hover:border-blue-300">
								{score}
							</button>
						{/each}
					</div>
				{/if}
			</div>
		{/each}
	</section>

	{#if isLoggedIn()}
		<div class="mt-6 space-y-2">
			<h3 class="text-sm font-medium text-gray-700">Your Answer</h3>
			<textarea bind:value={newAnswer} rows="4" class="w-full border border-gray-300 rounded-lg p-3 text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent" placeholder="Write your answer..."></textarea>
			<button onclick={submitAnswer} disabled={submitting || !newAnswer.trim()} class="px-4 py-2 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 disabled:bg-gray-300">
				{submitting ? 'Submitting...' : 'Submit Answer'}
			</button>
		</div>
	{/if}
{:else if !error}
	<p class="text-gray-500">Loading...</p>
{/if}
