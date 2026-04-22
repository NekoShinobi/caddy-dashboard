<script lang="ts">
	import './layout.css';
	import Header from '$lib/components/Header.svelte';
	import { theme } from '$lib/theme.svelte';

	let { children } = $props();
</script>

<div
	class="relative flex min-h-screen flex-col bg-neutral-50 text-neutral-900 dark:bg-neutral-950 dark:text-white"
	class:dark={theme.dark}
>
	<Header />
	<main class="container mx-auto flex-1 px-6 py-10">{@render children()}</main>
</div>
