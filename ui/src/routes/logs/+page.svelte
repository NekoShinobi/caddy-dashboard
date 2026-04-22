<script lang="ts">
	import { onMount } from 'svelte';
	import { anonymize, anonIP, anonHost } from '$lib/anonymize.svelte';
	import { colorTheme } from '$lib/color-theme.svelte';
	import { theme } from '$lib/theme.svelte';

	interface RequestInfo {
		remote_ip: string;
		client_ip: string;
		proto: string;
		method: string;
		host: string;
		uri: string;
	}

	interface LogEntry {
		ts: number;
		request: RequestInfo;
		duration: number;
		size: number;
		status: number;
		bytes_read: number;
	}

	interface LogsResponse {
		total: number;
		page: number;
		limit: number;
		entries: LogEntry[];
	}

	let data = $state<LogsResponse | null>(null);
	let loading = $state(true);
	let error = $state('');

	let filterStatus = $state('');
	let filterHost = $state('');
	let filterMethod = $state('');
	let page = $state(0);
	let limit = $state(50);
	let pageInput = $state('1');

	async function fetchLogs() {
		loading = true;
		error = '';
		try {
			const params = new URLSearchParams({ page: String(page), limit: String(limit) });
			if (filterStatus) params.set('status', filterStatus);
			if (filterHost) params.set('host', filterHost);
			if (filterMethod) params.set('method', filterMethod);
			const res = await fetch(`/api/logs?${params}`);
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			data = await res.json();
			pageInput = String(page + 1);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to fetch logs';
		} finally {
			loading = false;
		}
	}

	function applyFilters() {
		page = 0;
		fetchLogs();
	}

	function goToPage(n: number) {
		const maxPage = data ? Math.ceil(data.total / limit) - 1 : 0;
		page = Math.max(0, Math.min(n, maxPage));
		fetchLogs();
	}

	function onPageInputCommit() {
		const n = parseInt(pageInput, 10);
		if (!isNaN(n)) goToPage(n - 1);
	}

	function onLimitChange(e: Event) {
		limit = parseInt((e.target as HTMLSelectElement).value, 10);
		page = 0;
		fetchLogs();
	}

	function statusColor(code: number): string {
		const c = colorTheme.theme[theme.dark ? 'dark' : 'light'];
		if (code < 300) return c.green;
		if (code < 400) return c.blue;
		if (code < 500) return c.yellow;
		return c.red;
	}

	function formatTs(ts: number): string {
		return new Date(ts * 1000).toLocaleString();
	}

	function formatDuration(d: number): string {
		const ms = d * 1000;
		return ms < 1 ? `${(ms * 1000).toFixed(0)}µs` : `${ms.toFixed(1)}ms`;
	}

	onMount(fetchLogs);
</script>

<div class="max-w-full space-y-6">
	<div>
		<h1 class="text-3xl font-bold">Logs</h1>
		<p class="mt-1 text-neutral-500 dark:text-white/50">Browse and filter access log entries</p>
	</div>

	<div class="flex flex-wrap gap-3">
		<input
			bind:value={filterStatus}
			onkeydown={(e) => e.key === 'Enter' && applyFilters()}
			placeholder="Status (e.g. 404)"
			class="rounded-lg border border-neutral-200 bg-neutral-100 px-3 py-2 text-sm outline-none placeholder:text-neutral-400 focus:border-neutral-400 dark:border-white/10 dark:bg-white/5 dark:placeholder:text-white/30 dark:focus:border-white/30"
		/>
		<input
			bind:value={filterHost}
			onkeydown={(e) => e.key === 'Enter' && applyFilters()}
			placeholder="Host"
			class="rounded-lg border border-neutral-200 bg-neutral-100 px-3 py-2 text-sm outline-none placeholder:text-neutral-400 focus:border-neutral-400 dark:border-white/10 dark:bg-white/5 dark:placeholder:text-white/30 dark:focus:border-white/30"
		/>
		<input
			bind:value={filterMethod}
			onkeydown={(e) => e.key === 'Enter' && applyFilters()}
			placeholder="Method (e.g. GET)"
			class="rounded-lg border border-neutral-200 bg-neutral-100 px-3 py-2 text-sm outline-none placeholder:text-neutral-400 focus:border-neutral-400 dark:border-white/10 dark:bg-white/5 dark:placeholder:text-white/30 dark:focus:border-white/30"
		/>
		<button
			onclick={applyFilters}
			class="rounded-lg border border-neutral-300 bg-neutral-200 px-4 py-2 text-sm hover:bg-neutral-300 dark:border-white/20 dark:bg-white/10 dark:hover:bg-white/20"
		>
			Filter
		</button>
		<div class="ml-auto flex items-center gap-2">
			<label class="text-sm text-neutral-500 dark:text-white/50">Rows</label>
			<select
				onchange={onLimitChange}
				value={limit}
				class="rounded-lg border border-neutral-200 bg-neutral-100 px-2 py-2 text-sm outline-none dark:border-white/10 dark:bg-white/5"
			>
				{#each [25, 50, 100, 250, 500] as n}
					<option value={n}>{n}</option>
				{/each}
			</select>
		</div>
	</div>

	{#if error}
		<div class="rounded-lg border border-red-200 bg-red-50 p-4 text-red-600 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-400">{error}</div>
	{/if}

	{#if data}
		<p class="text-sm text-neutral-500 dark:text-white/50">{data.total.toLocaleString()} entries</p>

		<div class="overflow-x-auto rounded-lg border border-neutral-200 dark:border-white/10">
			<table class="w-full text-sm">
				<thead>
					<tr class="border-b border-neutral-200 text-left text-xs uppercase tracking-wide text-neutral-400 dark:border-white/10 dark:text-white/40">
						<th class="px-4 py-3">Time</th>
						<th class="px-4 py-3">Status</th>
						<th class="px-4 py-3">Method</th>
						<th class="px-4 py-3">Host</th>
						<th class="px-4 py-3">Path</th>
						<th class="px-4 py-3">Duration</th>
						<th class="px-4 py-3">Size</th>
						<th class="px-4 py-3">IP</th>
					</tr>
				</thead>
				<tbody>
					{#each data.entries as entry}
						<tr class="border-b border-neutral-100 hover:bg-neutral-50 dark:border-white/5 dark:hover:bg-white/5">
							<td class="px-4 py-2 font-mono text-neutral-500 dark:text-white/60">{formatTs(entry.ts)}</td>
							<td class="px-4 py-2 font-mono font-semibold" style="color: {statusColor(entry.status)}">{entry.status}</td>
							<td class="px-4 py-2 font-mono text-neutral-700 dark:text-white/80">{entry.request.method}</td>
							<td class="px-4 py-2 font-mono text-neutral-600 dark:text-white/70">{anonymize.on ? anonHost(entry.request.host) : entry.request.host}</td>
							<td class="max-w-xs truncate px-4 py-2 font-mono text-neutral-600 dark:text-white/70">{entry.request.uri}</td>
							<td class="px-4 py-2 font-mono text-neutral-500 dark:text-white/60">{formatDuration(entry.duration)}</td>
							<td class="px-4 py-2 font-mono text-neutral-500 dark:text-white/60">{entry.size}</td>
							<td class="px-4 py-2 font-mono text-neutral-400 dark:text-white/50">{anonymize.on ? anonIP(entry.request.client_ip) : entry.request.client_ip}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<div class="flex items-center gap-3">
			<button
				onclick={() => goToPage(page - 1)}
				disabled={page === 0}
				class="rounded-lg border border-neutral-200 px-4 py-2 text-sm disabled:opacity-30 hover:bg-neutral-50 dark:border-white/10 dark:hover:bg-white/5"
			>
				Previous
			</button>
			<span class="text-sm text-neutral-500 dark:text-white/50">Page</span>
			<input
				bind:value={pageInput}
				onkeydown={(e) => e.key === 'Enter' && onPageInputCommit()}
				onblur={onPageInputCommit}
				class="w-16 rounded-lg border border-neutral-200 bg-neutral-100 px-2 py-2 text-center text-sm outline-none dark:border-white/10 dark:bg-white/5"
			/>
			<span class="text-sm text-neutral-500 dark:text-white/50">of {Math.ceil(data.total / limit)}</span>
			<button
				onclick={() => goToPage(page + 1)}
				disabled={(page + 1) * limit >= data.total}
				class="rounded-lg border border-neutral-200 px-4 py-2 text-sm disabled:opacity-30 hover:bg-neutral-50 dark:border-white/10 dark:hover:bg-white/5"
			>
				Next
			</button>
		</div>
	{:else if loading}
		<div class="text-neutral-500 dark:text-white/50">Loading...</div>
	{/if}
</div>
