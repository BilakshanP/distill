<script lang="ts">
	import { api, isLoggedIn } from '$lib/api';
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Card from '$lib/components/ui/card';
	import * as Tabs from '$lib/components/ui/tabs';
	import { Separator } from '$lib/components/ui/separator';

	let config = $state<Record<string, string>>({});
	let jobs = $state<{ id: string; status: string; job_type: string; created_at: string }[]>([]);
	let loading = $state(true);
	let message = $state('');
	let error = $state('');

	// Config edit
	let editKey = $state('');
	let editValue = $state('');

	// Promote
	let promoteId = $state('');

	// Quota
	let quotaUserId = $state('');
	let quotaValue = $state('');

	onMount(async () => {
		if (!isLoggedIn()) return;
		try {
			const c = await api.getConfig();
			config = c.config;
			jobs = await api.listJobs();
		} catch (e: any) {
			error = e.message;
		} finally {
			loading = false;
		}
	});

	async function saveConfig() {
		if (!editKey.trim()) return;
		try {
			const c = await api.updateConfig({ [editKey]: editValue });
			config = c.config;
			message = `Updated ${editKey}`;
			editKey = '';
			editValue = '';
		} catch (e: any) { error = e.message; }
	}

	async function doReEmbed() {
		try {
			const r = await api.reEmbed();
			message = `Re-embed queued: ${r.enqueued} questions`;
		} catch (e: any) { error = e.message; }
	}

	async function doPromote() {
		if (!promoteId.trim()) return;
		try {
			await api.promoteUser(promoteId);
			message = `User ${promoteId} promoted to admin`;
			promoteId = '';
		} catch (e: any) { error = e.message; }
	}

	async function doSetQuota() {
		if (!quotaUserId.trim()) return;
		try {
			await api.setUserQuota(quotaUserId, parseInt(quotaValue) || 0);
			message = `Quota set for ${quotaUserId}`;
			quotaUserId = '';
			quotaValue = '';
		} catch (e: any) { error = e.message; }
	}

	async function refreshJobs() {
		try { jobs = await api.listJobs(); } catch {}
	}
</script>

<svelte:head><title>Admin - Distill</title></svelte:head>

<div class="space-y-6">
	<h1 class="text-2xl font-bold tracking-tight">Admin Panel</h1>

	{#if error}<p class="text-destructive text-sm">{error}</p>{/if}
	{#if message}<p class="text-green-600 text-sm">{message}</p>{/if}

	{#if loading}
		<p class="text-muted-foreground text-sm">Loading...</p>
	{:else}
		<Tabs.Root value="config">
			<Tabs.List>
				<Tabs.Trigger value="config">Config</Tabs.Trigger>
				<Tabs.Trigger value="users">Users</Tabs.Trigger>
				<Tabs.Trigger value="jobs">Jobs</Tabs.Trigger>
				<Tabs.Trigger value="actions">Actions</Tabs.Trigger>
			</Tabs.List>

			<Tabs.Content value="config" class="space-y-4 pt-4">
				<Card.Root>
					<Card.Header>
						<Card.Title class="text-sm">Runtime Configuration</Card.Title>
					</Card.Header>
					<Card.Content>
						<div class="space-y-2">
							{#each Object.entries(config) as [key, value]}
								<div class="flex justify-between items-center text-sm py-1 border-b border-border">
									<span class="font-mono text-muted-foreground">{key}</span>
									<span>{value}</span>
								</div>
							{/each}
						</div>
					</Card.Content>
					<Card.Footer class="flex gap-2">
						<Input bind:value={editKey} placeholder="Key" class="w-40" />
						<Input bind:value={editValue} placeholder="Value" class="flex-1" />
						<Button size="sm" onclick={saveConfig} disabled={!editKey.trim()}>Set</Button>
					</Card.Footer>
				</Card.Root>
			</Tabs.Content>

			<Tabs.Content value="users" class="space-y-4 pt-4">
				<Card.Root>
					<Card.Header><Card.Title class="text-sm">Promote User to Admin</Card.Title></Card.Header>
					<Card.Footer class="flex gap-2">
						<Input bind:value={promoteId} placeholder="User UUID" class="flex-1" />
						<Button size="sm" onclick={doPromote} disabled={!promoteId.trim()}>Promote</Button>
					</Card.Footer>
				</Card.Root>

				<Card.Root>
					<Card.Header><Card.Title class="text-sm">Set User Token Quota</Card.Title></Card.Header>
					<Card.Footer class="flex gap-2">
						<Input bind:value={quotaUserId} placeholder="User UUID" class="flex-1" />
						<Input bind:value={quotaValue} placeholder="Monthly quota" class="w-32" />
						<Button size="sm" onclick={doSetQuota} disabled={!quotaUserId.trim()}>Set</Button>
					</Card.Footer>
				</Card.Root>
			</Tabs.Content>

			<Tabs.Content value="jobs" class="space-y-4 pt-4">
				<div class="flex justify-between items-center">
					<h3 class="text-sm font-medium">Background Jobs</h3>
					<Button variant="outline" size="sm" onclick={refreshJobs}>Refresh</Button>
				</div>
				{#if jobs.length === 0}
					<p class="text-muted-foreground text-sm">No jobs.</p>
				{:else}
					<div class="space-y-1">
						{#each jobs as job}
							<div class="flex justify-between items-center text-xs py-1 border-b border-border">
								<span class="font-mono">{job.id.slice(0, 8)}</span>
								<span>{job.job_type}</span>
								<span class="px-2 py-0.5 rounded {job.status === 'completed' ? 'bg-green-100 text-green-700' : job.status === 'failed' ? 'bg-red-100 text-red-700' : 'bg-yellow-100 text-yellow-700'}">{job.status}</span>
								<span class="text-muted-foreground">{new Date(job.created_at).toLocaleString()}</span>
							</div>
						{/each}
					</div>
				{/if}
			</Tabs.Content>

			<Tabs.Content value="actions" class="space-y-4 pt-4">
				<Card.Root>
					<Card.Header>
						<Card.Title class="text-sm">Re-embed All Questions</Card.Title>
						<Card.Description>Queue all questions for re-embedding with the current model.</Card.Description>
					</Card.Header>
					<Card.Footer>
						<Button variant="outline" onclick={doReEmbed}>Start Re-embed</Button>
					</Card.Footer>
				</Card.Root>
			</Tabs.Content>
		</Tabs.Root>
	{/if}
</div>
