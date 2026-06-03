<script lang="ts">
	import './layout.css';
	import { isLoggedIn, clearToken } from '$lib/api';
	import { Button } from '$lib/components/ui/button';
	import { Separator } from '$lib/components/ui/separator';

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

<div class="min-h-screen bg-background font-mono">
	<header class="sticky top-0 z-50 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
		<div class="max-w-5xl mx-auto flex h-14 items-center justify-between px-6">
			<div class="flex items-center gap-6">
				<a href="/" class="text-lg font-bold tracking-tight">distill</a>
				<nav class="flex items-center gap-4 text-sm">
					<a href="/search" class="text-muted-foreground hover:text-foreground transition-colors">Search</a>
					<a href="/ask" class="text-muted-foreground hover:text-foreground transition-colors">Ask</a>
					<a href="/tags" class="text-muted-foreground hover:text-foreground transition-colors">Tags</a>
				</nav>
			</div>
			<div>
				{#if loggedIn}
					<Button variant="ghost" size="sm" onclick={logout}>Logout</Button>
				{:else}
					<Button variant="outline" size="sm" href="/login">Login</Button>
				{/if}
			</div>
		</div>
	</header>

	<main class="max-w-5xl mx-auto px-6 py-8">
		{@render children()}
	</main>
</div>
