<script lang="ts">
	import { colorTheme } from '$lib/color-theme.svelte';
	import { theme } from '$lib/theme.svelte';
	import { THEMES } from '$lib/themes';

	let open = $state(false);
	let container = $state<HTMLDivElement | null>(null);

	function quad(id: string): [string, string, string, string] {
		const t = THEMES.find((t) => t.id === id) ?? THEMES[0];
		const c = t[theme.dark ? 'dark' : 'light'];
		return [c.blue, c.green, c.red, c.purple];
	}

	let current = $derived(quad(colorTheme.id));

	function select(id: string) {
		colorTheme.set(id);
		open = false;
	}

	function onWindowClick(e: MouseEvent) {
		if (container && !container.contains(e.target as Node)) open = false;
	}
</script>

<svelte:window onclick={onWindowClick} />

<div bind:this={container} class="relative">
	<button
		onclick={() => (open = !open)}
		aria-label="Color theme"
		aria-expanded={open}
		class="rounded-lg border border-neutral-200 p-2 transition-colors hover:bg-neutral-100 dark:border-white/10 dark:hover:bg-white/10"
	>
		<svg width="16" height="16" viewBox="0 0 16 16">
			<rect x="0" y="0" width="7" height="7" fill={current[0]} rx="1.5" />
			<rect x="9" y="0" width="7" height="7" fill={current[1]} rx="1.5" />
			<rect x="0" y="9" width="7" height="7" fill={current[2]} rx="1.5" />
			<rect x="9" y="9" width="7" height="7" fill={current[3]} rx="1.5" />
		</svg>
	</button>

	{#if open}
		<div class="absolute right-0 top-full z-50 mt-1.5 w-44 overflow-hidden rounded-lg border border-neutral-200 bg-white shadow-lg dark:border-white/10 dark:bg-neutral-900">
			{#each THEMES as t}
				{@const [c1, c2, c3, c4] = quad(t.id)}
				<button
					onclick={() => select(t.id)}
					class="flex w-full items-center gap-2.5 px-3 py-2.5 text-sm transition-colors hover:bg-neutral-100 dark:hover:bg-white/5 {colorTheme.id === t.id ? 'bg-neutral-50 dark:bg-white/5' : ''}"
				>
					<svg width="14" height="14" viewBox="0 0 16 16" class="shrink-0">
						<rect x="0" y="0" width="7" height="7" fill={c1} rx="1.5" />
						<rect x="9" y="0" width="7" height="7" fill={c2} rx="1.5" />
						<rect x="0" y="9" width="7" height="7" fill={c3} rx="1.5" />
						<rect x="9" y="9" width="7" height="7" fill={c4} rx="1.5" />
					</svg>
					<span>{t.name}</span>
					{#if colorTheme.id === t.id}
						<svg class="ml-auto shrink-0" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
							<polyline points="20 6 9 17 4 12" />
						</svg>
					{/if}
				</button>
			{/each}
		</div>
	{/if}
</div>
