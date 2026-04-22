<script lang="ts">
	import { onMount } from 'svelte';

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
		slowest_paths: SlowPath[];
	}

	let stats = $state<Stats | null>(null);
	let loading = $state(true);
	let error = $state('');

	async function fetchStats() {
		loading = true;
		error = '';
		try {
			const res = await fetch('/api/stats');
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			stats = await res.json();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to fetch stats';
		} finally {
			loading = false;
		}
	}

	function statusColor(code: string): string {
		const n = Number(code);
		if (n < 300) return 'text-green-600 dark:text-green-400';
		if (n < 400) return 'text-blue-600 dark:text-blue-400';
		if (n < 500) return 'text-yellow-600 dark:text-yellow-400';
		return 'text-red-600 dark:text-red-400';
	}

	function formatBytes(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
		if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
		return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
	}

	onMount(() => {
		fetchStats();
		const interval = setInterval(fetchStats, 30000);
		return () => clearInterval(interval);
	});
</script>

<div class="mx-auto max-w-6xl space-y-8">
	<div>
		<h1 class="text-3xl font-bold">Overview</h1>
		<p class="mt-1 text-neutral-500 dark:text-white/50">Caddy access log analytics</p>
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
				<p class="text-xs uppercase tracking-wide text-neutral-500 dark:text-white/50">Unique Hosts</p>
				<p class="mt-2 text-3xl font-bold">{stats.top_hosts.length}</p>
			</div>
		</div>

		<div class="grid grid-cols-1 gap-6 lg:grid-cols-3">
			<div class="rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5">
				<h2 class="mb-4 text-sm font-semibold uppercase tracking-wide text-neutral-500 dark:text-white/50">Status Codes</h2>
				<ul class="space-y-2">
					{#each Object.entries(stats.status_codes).sort((a, b) => Number(a[0]) - Number(b[0])) as [code, count]}
						<li class="flex items-center justify-between">
							<span class="font-mono text-sm {statusColor(code)}">{code}</span>
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
							<span class="truncate font-mono text-sm text-neutral-700 dark:text-white/70">{host}</span>
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
							<span class="font-mono text-sm text-neutral-700 dark:text-white/70">{ip}</span>
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
						<span class="truncate font-mono text-sm text-neutral-700 dark:text-white/70">{path}</span>
						<span class="shrink-0 text-sm text-neutral-600 dark:text-white/70">{count.toLocaleString()}</span>
					</li>
				{/each}
			</ul>
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
									<td class="py-2 pr-4 font-mono text-neutral-700 dark:text-white/70 max-w-xs truncate">{row.path}</td>
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
