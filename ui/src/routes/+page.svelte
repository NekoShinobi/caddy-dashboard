<script lang="ts">
	import { onMount } from 'svelte';
	import { fly } from 'svelte/transition';
	import { anonymize, anonIP, anonHost, anonPathKey } from '$lib/anonymize.svelte';
	import { colorTheme } from '$lib/color-theme.svelte';
	import { theme } from '$lib/theme.svelte';
	import { timeRange } from '$lib/time-range.svelte';
	import TimeRangeSelector from '$lib/components/TimeRangeSelector.svelte';
	import UAModal from '$lib/components/UAModal.svelte';

	let selectedUA = $state<string | null>(null);

	interface SlowPath {
		path: string;
		avg_ms: number;
		p99_ms: number;
		count: number;
	}

	interface Stats {
		total_requests: number;
		status_codes: Record<string, number>;
		top_paths: [string, number][];
		top_hosts: [string, number][];
		top_ips: [string, number][];
		avg_duration_ms: number;
		total_bytes: number;
		unique_clients: number;
		top_referrers: [string, number][];
		top_user_agents: [string, number][];
		slowest_paths: SlowPath[];
	}

	let stats = $state<Stats | null>(null);
	let loading = $state(true);
	let error = $state('');
	let autoRefresh = $state(true);

	async function fetchStats() {
		loading = true;
		error = '';
		try {
			const since = timeRange.sinceParam();
			const url = since ? `/api/stats?since=${since}` : '/api/stats';
			const res = await fetch(url);
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			stats = await res.json();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to fetch stats';
		} finally {
			loading = false;
		}
	}

	function statusColor(code: string | number): string {
		const n = Number(code);
		const c = colorTheme.theme[theme.dark ? 'dark' : 'light'];
		if (n < 300) return c.green;
		if (n < 400) return c.blue;
		if (n < 500) return c.yellow;
		return c.red;
	}

	function pathUri(key: string): string {
		const i = key.indexOf('/');
		return i >= 0 ? key.slice(i) : key;
	}

	function formatBytes(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
		if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
		return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
	}

	onMount(() => {
		fetchStats();
		const interval = setInterval(() => { if (autoRefresh) fetchStats(); }, 30000);
		return () => clearInterval(interval);
	});

</script>

<div class="mx-auto max-w-6xl space-y-8">
	<div class="flex flex-wrap items-end justify-between gap-4">
		<div>
			<h1 class="text-3xl font-bold">Overview</h1>
			<p class="mt-1 text-neutral-500 dark:text-white/50">Caddy access log analytics</p>
		</div>
		<div class="flex items-center gap-2">
			<TimeRangeSelector onchange={fetchStats} />
			<button
				onclick={() => { autoRefresh = !autoRefresh; if (autoRefresh) fetchStats(); }}
				title={autoRefresh ? 'Auto-refresh on (30s) — click to pause' : 'Auto-refresh paused — click to resume'}
				class="relative flex h-8 w-8 items-center justify-center overflow-hidden rounded-lg border transition-colors {autoRefresh
					? 'border-neutral-200 hover:bg-neutral-100 dark:border-white/10 dark:hover:bg-white/5'
					: 'border-amber-400/50 bg-amber-50 text-amber-700 dark:border-amber-400/30 dark:bg-amber-400/10 dark:text-amber-400'}"
			>
				{#if autoRefresh}
					<svg in:fly={{ x: 16, duration: 150 }} out:fly={{ x: -16, duration: 150 }} xmlns="http://www.w3.org/2000/svg" class="absolute h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
						<rect x="6" y="5" width="4" height="14" rx="1"/><rect x="14" y="5" width="4" height="14" rx="1"/>
					</svg>
				{:else}
					<svg in:fly={{ x: -16, duration: 150 }} out:fly={{ x: 16, duration: 150 }} xmlns="http://www.w3.org/2000/svg" class="absolute h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
						<polygon points="5,3 19,12 5,21"/>
					</svg>
				{/if}
			</button>
		</div>
	</div>

	{#if error}
		<div class="rounded-lg border border-red-200 bg-red-50 p-4 text-red-600 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-400">
			{error}
		</div>
	{/if}

	{#if loading && !stats}
		<div class="grid grid-cols-2 gap-4 lg:grid-cols-4">
			{#each Array(4) as _}
				<div class="animate-pulse rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5">
					<div class="mb-3 h-3 w-20 rounded bg-neutral-200 dark:bg-white/10"></div>
					<div class="h-8 w-16 rounded bg-neutral-200 dark:bg-white/10"></div>
				</div>
			{/each}
		</div>
	{:else if stats}
		<div class="grid grid-cols-2 gap-4 lg:grid-cols-4">
			<div class="rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5">
				<p class="text-xs uppercase tracking-wide text-neutral-500 dark:text-white/50">Total Requests</p>
				<p class="mt-2 text-3xl font-bold">{stats.total_requests.toLocaleString()}</p>
			</div>
			<div class="rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5">
				<p class="text-xs uppercase tracking-wide text-neutral-500 dark:text-white/50">Avg Duration</p>
				<p class="mt-2 text-3xl font-bold">
					{stats.avg_duration_ms.toFixed(1)}<span class="text-sm font-normal text-neutral-500 dark:text-white/50"> ms</span>
				</p>
			</div>
			<div class="rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5">
				<p class="text-xs uppercase tracking-wide text-neutral-500 dark:text-white/50">Total Bytes</p>
				<p class="mt-2 text-3xl font-bold">{formatBytes(stats.total_bytes)}</p>
			</div>
			<div class="rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5">
				<p class="text-xs uppercase tracking-wide text-neutral-500 dark:text-white/50">Unique Clients</p>
				<p class="mt-2 text-3xl font-bold">{stats.unique_clients.toLocaleString()}</p>
			</div>
		</div>

		<div class="grid grid-cols-1 gap-6 lg:grid-cols-3">
			<div class="rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5">
				<h2 class="mb-4 text-sm font-semibold uppercase tracking-wide text-neutral-500 dark:text-white/50">Status Codes</h2>
				<ul class="space-y-2">
					{#each Object.entries(stats.status_codes).sort((a, b) => Number(a[0]) - Number(b[0])) as [code, count]}
						<li class="flex items-center justify-between">
							<a href="/logs?status={code}" class="font-mono text-sm hover:underline" style="color: {statusColor(code)}">{code}</a>
							<span class="text-sm text-neutral-600 dark:text-white/70">{count.toLocaleString()}</span>
						</li>
					{/each}
				</ul>
			</div>

			<div class="rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5">
				<h2 class="mb-4 text-sm font-semibold uppercase tracking-wide text-neutral-500 dark:text-white/50">Top Hosts</h2>
				<ul class="space-y-2">
					{#each stats.top_hosts as [host, count]}
						<li class="flex items-center justify-between gap-4">
							<a href="/logs?host={encodeURIComponent(host)}" class="truncate font-mono text-sm text-neutral-700 hover:underline dark:text-white/70">{anonymize.on ? anonHost(host) : host}</a>
							<span class="shrink-0 text-sm text-neutral-600 dark:text-white/70">{count.toLocaleString()}</span>
						</li>
					{/each}
				</ul>
			</div>

			<div class="rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5">
				<h2 class="mb-4 text-sm font-semibold uppercase tracking-wide text-neutral-500 dark:text-white/50">Top IPs</h2>
				<ul class="space-y-2">
					{#each stats.top_ips as [ip, count]}
						<li class="flex items-center justify-between gap-4">
							<a href="/logs?ip={encodeURIComponent(ip)}" class="font-mono text-sm text-neutral-700 hover:underline dark:text-white/70">{anonymize.on ? anonIP(ip) : ip}</a>
							<span class="shrink-0 text-sm text-neutral-600 dark:text-white/70">{count.toLocaleString()}</span>
						</li>
					{/each}
				</ul>
			</div>
		</div>

		<div class="rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5">
			<h2 class="mb-4 text-sm font-semibold uppercase tracking-wide text-neutral-500 dark:text-white/50">Top Paths</h2>
			<ul class="space-y-2">
				{#each stats.top_paths as [path, count]}
					<li class="flex items-center justify-between gap-4">
						<a href="/logs?path={encodeURIComponent(pathUri(path))}" class="truncate font-mono text-sm text-neutral-700 hover:underline dark:text-white/70">{anonymize.on ? anonPathKey(path) : path}</a>
						<span class="shrink-0 text-sm text-neutral-600 dark:text-white/70">{count.toLocaleString()}</span>
					</li>
				{/each}
			</ul>
		</div>

		<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
			{#if (stats.top_referrers ?? []).length > 0}
				<div class="rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5">
					<h2 class="mb-4 text-sm font-semibold uppercase tracking-wide text-neutral-500 dark:text-white/50">Top Referrers</h2>
					<ul class="space-y-2">
						{#each stats.top_referrers as [ref_, count]}
							<li class="flex items-center justify-between gap-4">
								<span class="truncate font-mono text-sm text-neutral-700 dark:text-white/70">{ref_}</span>
								<span class="shrink-0 text-sm text-neutral-600 dark:text-white/70">{count.toLocaleString()}</span>
							</li>
						{/each}
					</ul>
				</div>
			{/if}

			{#if (stats.top_user_agents ?? []).length > 0}
				<div class="rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5">
					<h2 class="mb-4 text-sm font-semibold uppercase tracking-wide text-neutral-500 dark:text-white/50">Top User Agents</h2>
					<ul class="space-y-2">
						{#each stats.top_user_agents as [ua, count]}
							<li class="flex items-center justify-between gap-4">
								<button
									onclick={() => selectedUA = ua}
									class="truncate font-mono text-sm text-neutral-700 hover:underline dark:text-white/70 text-left"
									title="Click to parse"
								>{ua}</button>
								<span class="shrink-0 text-sm text-neutral-600 dark:text-white/70">{count.toLocaleString()}</span>
							</li>
						{/each}
					</ul>
				</div>
			{/if}
		</div>

		{#if (stats.slowest_paths ?? []).length > 0}
			<div class="rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5">
				<h2 class="mb-4 text-sm font-semibold uppercase tracking-wide text-neutral-500 dark:text-white/50">Slowest Paths</h2>
				<div class="overflow-x-auto">
					<table class="w-full text-sm">
						<thead>
							<tr class="border-b border-neutral-200 text-left text-xs uppercase tracking-wide text-neutral-400 dark:border-white/10 dark:text-white/40">
								<th class="pb-3 pr-4">Path</th>
								<th class="pb-3 pr-4 text-right">Requests</th>
								<th class="pb-3 pr-4 text-right">Avg</th>
								<th class="pb-3 text-right">p99</th>
							</tr>
						</thead>
						<tbody>
							{#each stats.slowest_paths as row}
								<tr class="border-b border-neutral-200/60 last:border-0 dark:border-white/5">
									<td class="py-2 pr-4 max-w-xs truncate"><a href="/logs?path={encodeURIComponent(pathUri(row.path))}" class="font-mono text-neutral-700 hover:underline dark:text-white/70">{anonymize.on ? anonPathKey(row.path) : row.path}</a></td>
									<td class="py-2 pr-4 text-right text-neutral-500 dark:text-white/50">{row.count.toLocaleString()}</td>
									<td class="py-2 pr-4 text-right font-mono text-neutral-600 dark:text-white/60">{row.avg_ms.toFixed(1)}ms</td>
									<td class="py-2 text-right font-mono font-semibold text-orange-600 dark:text-orange-400">{row.p99_ms.toFixed(1)}ms</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</div>
		{/if}
	{/if}
</div>

{#if selectedUA}
	<UAModal ua={selectedUA} onclose={() => selectedUA = null} />
{/if}
