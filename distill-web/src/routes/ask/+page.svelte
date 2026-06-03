<script lang="ts">
	import { goto } from '$app/navigation';
	import { api, isLoggedIn } from '$lib/api';

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

<div class="space-y-4">
	<h1 class="text-2xl font-bold text-gray-900">Ask a Question</h1>

	{#if !isLoggedIn()}
		<p class="text-gray-600">Please <a href="/login" class="text-blue-600 hover:underline">login</a> to ask a question.</p>
	{:else}
		{#if error}
			<p class="text-red-600 text-sm">{error}</p>
		{/if}

		<form onsubmit={(e) => { e.preventDefault(); submit(); }} class="space-y-4">
			<div>
				<label for="title" class="block text-sm font-medium text-gray-700 mb-1">Title</label>
				<input id="title" bind:value={title} type="text" class="w-full border border-gray-300 rounded-lg px-4 py-2 text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent" placeholder="What's your question?" />
			</div>

			<div>
				<label for="body" class="block text-sm font-medium text-gray-700 mb-1">Body</label>
				<textarea id="body" bind:value={body} rows="6" class="w-full border border-gray-300 rounded-lg px-4 py-2 text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent" placeholder="Provide details..."></textarea>
			</div>

			<div>
				<label for="tags" class="block text-sm font-medium text-gray-700 mb-1">Tags (comma-separated)</label>
				<input id="tags" bind:value={tags} type="text" class="w-full border border-gray-300 rounded-lg px-4 py-2 text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent" placeholder="rust, axum, database" />
			</div>

			<label class="flex items-center gap-2 text-sm text-gray-700">
				<input type="checkbox" bind:checked={aiAnswer} class="rounded border-gray-300" />
				Generate AI answer
			</label>

			<button type="submit" disabled={submitting || !title.trim() || !body.trim()} class="px-4 py-2 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 disabled:bg-gray-300">
				{submitting ? 'Submitting...' : 'Submit Question'}
			</button>
		</form>
	{/if}
</div>
