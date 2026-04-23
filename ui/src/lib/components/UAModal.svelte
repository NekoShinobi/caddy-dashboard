<script lang="ts">
	import { UAParser } from 'ua-parser-js';

	let { ua, onclose }: { ua: string; onclose: () => void } = $props();

	let copied = $state(false);

	const result = UAParser(ua);

	const rows: [string, string][] = [
		['Browser',  [result.browser.name, result.browser.version].filter(Boolean).join(' ') || '—'],
		['Engine',   [result.engine.name,  result.engine.version ].filter(Boolean).join(' ') || '—'],
		['OS',       [result.os.name,      result.os.version     ].filter(Boolean).join(' ') || '—'],
		['Device',   [result.device.vendor, result.device.model  ].filter(Boolean).join(' ') || '—'],
		['Type',     result.device.type || 'desktop'],
		['CPU',      result.cpu.architecture || '—'],
	];

	async function copy() {
		await navigator.clipboard.writeText(ua);
		copied = true;
		setTimeout(() => { copied = false; }, 1500);
	}
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div
	role="presentation"
	class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
	onclick={(e) => e.target === e.currentTarget && onclose()}
>
	<div class="relative mx-4 w-full max-w-lg rounded-xl border border-neutral-200 bg-white shadow-xl dark:border-white/10 dark:bg-neutral-900">
		<div class="flex items-center justify-between border-b border-neutral-200 px-5 py-4 dark:border-white/10">
			<h2 class="text-sm font-semibold uppercase tracking-wide text-neutral-500 dark:text-white/50">User Agent</h2>
			<button onclick={onclose} aria-label="Close" class="rounded p-1 hover:bg-neutral-100 dark:hover:bg-white/10">
				<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M18 6L6 18M6 6l12 12"/>
				</svg>
			</button>
		</div>

		<div class="flex items-start gap-3 border-b border-neutral-200 px-5 py-4 dark:border-white/10">
			<p class="flex-1 break-all font-mono text-xs text-neutral-700 dark:text-white/70">{ua}</p>
			<button
				onclick={copy}
				title="Copy to clipboard"
				class="shrink-0 rounded border border-neutral-200 px-2 py-1 text-xs transition-colors hover:bg-neutral-100 dark:border-white/10 dark:hover:bg-white/10"
			>
				{copied ? 'Copied!' : 'Copy'}
			</button>
		</div>

		<div class="px-5 py-4">
			<dl class="grid grid-cols-2 gap-x-6 gap-y-3 text-sm">
				{#each rows as [label, value]}
					<div>
						<dt class="text-xs uppercase tracking-wide text-neutral-400 dark:text-white/30">{label}</dt>
						<dd class="mt-0.5 font-medium text-neutral-800 dark:text-white/80">{value}</dd>
					</div>
				{/each}
			</dl>
		</div>
	</div>
</div>
