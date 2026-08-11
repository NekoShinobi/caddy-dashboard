<script lang="ts">
	import { page } from '$app/state';
	import { fade } from 'svelte/transition';
	import './layout.css';
	import './layout-v2.css';
	import AuthGate from '$lib/components/AuthGate.svelte';
	import AppShell from '$lib/components/AppShell-v2.svelte';
	import { theme } from '$lib/theme.svelte';

	let { children } = $props();

	$effect(() => {
		document.documentElement.classList.toggle('dark', theme.dark);
		document.documentElement.style.colorScheme = theme.dark ? 'dark' : 'light';
		document
			.querySelector('meta[name="theme-color"]')
			?.setAttribute('content', theme.dark ? 'oklch(0.145 0.015 230)' : 'oklch(0.976 0.006 220)');
	});
</script>

<div class="app-shell" class:dark={theme.dark}>
	<a class="skip-link" href="#main-content">Skip to content</a>
	<AuthGate>
		<AppShell />
		<main id="main-content" class="app-main" tabindex="-1">
			{#key page.url.pathname}
				<div class="route-frame" in:fade={{ duration: 150 }}>{@render children()}</div>
			{/key}
		</main>
	</AuthGate>
</div>
