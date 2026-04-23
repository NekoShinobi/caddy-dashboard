<script lang="ts">
	import { timeRange, TIME_RANGES } from '$lib/time-range.svelte';

	let { onchange }: { onchange?: () => void } = $props();
</script>

<div class="flex gap-1.5">
	{#each TIME_RANGES as opt}
		<button
			onclick={() => { timeRange.set(opt.seconds); onchange?.(); }}
			class="rounded-lg border px-3 py-1.5 text-sm transition-colors {timeRange.seconds === opt.seconds
				? 'border-neutral-400 bg-neutral-200 dark:border-white/30 dark:bg-white/10'
				: 'border-neutral-200 hover:bg-neutral-100 dark:border-white/10 dark:hover:bg-white/5'}"
		>
			{opt.label}
		</button>
	{/each}
</div>
