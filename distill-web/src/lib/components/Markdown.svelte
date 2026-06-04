<script lang="ts">
	import { marked } from 'marked';

	let { content }: { content: string } = $props();

	const html = $derived(marked.parse(content, { async: false }) as string);
</script>

<div class="prose prose-sm max-w-none dark:prose-invert">
	{@html html}
</div>
