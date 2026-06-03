<script lang="ts">
	import './layout.css';
	import { isLoggedIn, clearToken } from '$lib/api';

	let { children } = $props();
	let loggedIn = $state(false);

	$effect(() => {
		loggedIn = isLoggedIn();
	});

	function logout() {
		clearToken();
		loggedIn = false;
	}
</script>

<div class="min-h-screen bg-gray-50">
	<nav class="bg-white border-b border-gray-200 px-6 py-3 flex items-center justify-between">
		<div class="flex items-center gap-6">
			<a href="/" class="text-xl font-bold text-gray-900">Distill</a>
			<a href="/search" class="text-sm text-gray-600 hover:text-gray-900">Search</a>
			<a href="/ask" class="text-sm text-gray-600 hover:text-gray-900">Ask</a>
			<a href="/tags" class="text-sm text-gray-600 hover:text-gray-900">Tags</a>
		</div>
		<div>
			{#if loggedIn}
				<button onclick={logout} class="text-sm text-gray-600 hover:text-gray-900">Logout</button>
			{:else}
				<a href="/login" class="text-sm text-blue-600 hover:text-blue-800">Login</a>
			{/if}
		</div>
	</nav>

	<main class="max-w-4xl mx-auto px-6 py-8">
		{@render children()}
	</main>
</div>
