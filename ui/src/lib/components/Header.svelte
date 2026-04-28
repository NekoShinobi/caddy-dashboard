<script lang="ts">
	import { page } from '$app/state';
	import { theme } from '$lib/theme.svelte';
	import { anonymize } from '$lib/anonymize.svelte';
	import { auth } from '$lib/auth.svelte';
	import ThemePicker from './ThemePicker.svelte';

	const links = [
		{ href: '/', label: 'Overview' },
		{ href: '/logs', label: 'Logs' },
		{ href: '/graphs', label: 'Graphs' },
		{ href: '/map', label: 'Map' },
		{ href: '/reports', label: 'Reports' }
	];

	let now = $state(new Date());
	$effect(() => {
		const id = setInterval(() => { now = new Date(); }, 1000);
		return () => clearInterval(id);
	});
</script>

<header class="border-b border-neutral-200 px-6 py-4 dark:border-white/10">
	<nav class="container mx-auto flex items-center gap-8">
		<span class="font-bold">Caddy Dashboard</span>
		{#each links as link}
			<a
				href={link.href}
				class="text-sm transition-colors {page.url.pathname === link.href
					? ''
					: 'text-neutral-500 hover:text-neutral-700 dark:text-white/50 dark:hover:text-white/80'}"
			>
				{link.label}
			</a>
		{/each}
		<div class="ml-auto flex items-center gap-3">
			<span class="font-mono text-xs tabular-nums text-neutral-400 dark:text-white/30">{now.toLocaleTimeString()}</span>
			<ThemePicker />
			<!-- Anonymize toggle -->
			<button
				onclick={anonymize.toggle}
				aria-label="Toggle anonymize mode"
				title={anonymize.on ? 'Anonymize on' : 'Anonymize off'}
				class="rounded-lg border p-2 transition-colors {anonymize.on
					? 'border-amber-400 bg-amber-100 text-amber-600 dark:border-amber-500/60 dark:bg-amber-500/20 dark:text-amber-400'
					: 'border-neutral-200 text-neutral-500 hover:bg-neutral-100 dark:border-white/10 dark:text-white/50 dark:hover:bg-white/10'}"
			>
				<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
				</svg>
			</button>
			<!-- Theme toggle -->
			<button
				onclick={theme.toggle}
				class="rounded-lg border border-neutral-200 p-2 text-neutral-500 transition-colors hover:bg-neutral-100 dark:border-white/10 dark:text-white/50 dark:hover:bg-white/10"
				aria-label="Toggle theme"
			>
				{#if theme.dark}
					<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
						<circle cx="12" cy="12" r="4"/>
						<path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/>
					</svg>
				{:else}
					<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
						<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
					</svg>
				{/if}
			</button>
			<!-- User section -->
			{#if auth.user}
				<div class="flex items-center gap-2 border-l border-neutral-200 pl-3 dark:border-white/10">
					<span class="text-xs text-neutral-500 dark:text-white/50">{auth.user.username}</span>
					<a
						href="/settings"
						aria-label="Settings"
						title="Settings"
						class="rounded-lg border border-neutral-200 p-2 text-neutral-500 transition-colors hover:bg-neutral-100 dark:border-white/10 dark:text-white/50 dark:hover:bg-white/10 {page.url.pathname === '/settings' ? 'bg-neutral-100 dark:bg-white/10' : ''}"
					>
						<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
							<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
						</svg>
					</a>
					<button
						onclick={() => auth.logout()}
						aria-label="Sign out"
						title="Sign out"
						class="rounded-lg border border-neutral-200 p-2 text-neutral-500 transition-colors hover:bg-neutral-100 dark:border-white/10 dark:text-white/50 dark:hover:bg-white/10"
					>
						<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
							<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/>
						</svg>
					</button>
				</div>
			{/if}
		</div>
	</nav>
</header>
