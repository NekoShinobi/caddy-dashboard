<script lang="ts">
	import { UAParser } from 'ua-parser-js';
	import AppDialog from './AppDialog.svelte';

	let { ua, onclose }: { ua: string; onclose: () => void } = $props();
	let copied = $state(false);

	let result = $derived(UAParser(ua));
	let rows = $derived<[string, string][]>([
		['Browser', [result.browser.name, result.browser.version].filter(Boolean).join(' ') || '—'],
		['Engine', [result.engine.name, result.engine.version].filter(Boolean).join(' ') || '—'],
		['OS', [result.os.name, result.os.version].filter(Boolean).join(' ') || '—'],
		['Device', [result.device.vendor, result.device.model].filter(Boolean).join(' ') || '—'],
		['Type', result.device.type || 'desktop'],
		['CPU', result.cpu.architecture || '—']
	]);

	async function copy() {
		await navigator.clipboard.writeText(ua);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}
</script>

<AppDialog
	open={true}
	onOpenChange={(open) => {
		if (!open) onclose();
	}}
	title="User agent"
	description="Parsed browser, operating system, and device details"
	size="md"
>
	<div class="space-y-5">
		<p
			class="rounded-xl bg-neutral-50 p-4 font-mono text-xs leading-relaxed break-all text-neutral-700 dark:bg-white/[0.035] dark:text-white/75"
		>
			{ua}
		</p>

		<dl class="grid grid-cols-2 gap-x-8 gap-y-5 text-sm">
			{#each rows as [label, value] (label)}
				<div>
					<dt class="text-xs text-neutral-400 dark:text-white/35">{label}</dt>
					<dd class="mt-1 font-medium text-neutral-800 dark:text-white/85">{value}</dd>
				</div>
			{/each}
		</dl>
	</div>

	{#snippet actions()}
		<button class="button-secondary" onclick={copy}>
			{copied ? 'Copied' : 'Copy user agent'}
		</button>
	{/snippet}
</AppDialog>
