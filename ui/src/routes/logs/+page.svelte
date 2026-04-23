<script lang="ts">
	import { onMount } from 'svelte';
	import { page as appPage } from '$app/stores';
	import { get } from 'svelte/store';
	import { anonymize, anonIP, anonHost } from '$lib/anonymize.svelte';
	import { colorTheme } from '$lib/color-theme.svelte';
	import { theme } from '$lib/theme.svelte';
	import LogEntryModal from '$lib/components/LogEntryModal.svelte';

	interface TlsInfo {
		resumed: boolean;
		version: number;
		cipher_suite: number;
		proto: string;
		server_name: string;
	}

	interface RequestInfo {
		remote_ip: string;
		remote_port: string;
		client_ip: string;
		proto: string;
		method: string;
		host: string;
		uri: string;
		headers: Record<string, string[]>;
		tls?: TlsInfo;
	}

	interface LogEntry {
		ts: number;
		request: RequestInfo;
		duration: number;
		size: number;
		status: number;
		bytes_read: number;
		user_id: string;
		resp_headers: Record<string, string[]>;
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

	let query = $state('');
	let page = $state(0);
	let limit = $state(50);
	let pageInput = $state('1');
	let showHelp = $state(false);
	let selectedEntry = $state<LogEntry | null>(null);

	interface Filters {
		status?: string;
		host?: string;
		method?: string;
		ip?: string;
		path?: string;
		ua?: string;
		duration_gt?: string;
		size_gt?: string;
		size_lt?: string;
		text?: string;
		not_status?: string;
		not_host?: string;
		not_method?: string;
		not_ip?: string;
		not_path?: string;
	}

	const PRESETS = [
		{ label: 'Bot Traffic',   query: 'ua:bot' },
		{ label: 'Slow Requests', query: 'duration:>1000' },
		{ label: 'Auth Failures', query: 'status:401,403' },
		{ label: 'Errors',        query: 'status:5xx' },
		{ label: 'Not Found',     query: 'status:404' },
	];

	function parseQuery(q: string): Filters {
		const f: Filters = {};
		const bare: string[] = [];
		for (const token of q.trim().split(/\s+/)) {
			if (!token) continue;
			const neg = token.startsWith('-');
			const t = neg ? token.slice(1) : token;
			const colon = t.indexOf(':');
			if (colon === -1) { if (!neg) bare.push(t); continue; }
			const key = t.slice(0, colon).toLowerCase();
			const val = t.slice(colon + 1);
			if (!val) continue;
			if (neg) {
				if      (key === 'status')  f.not_status  = val;
				else if (key === 'host')    f.not_host    = val;
				else if (key === 'method')  f.not_method  = val;
				else if (key === 'ip')      f.not_ip      = val;
				else if (key === 'path')    f.not_path    = val;
			} else {
				if      (key === 'status')  f.status  = val;
				else if (key === 'host')    f.host    = val;
				else if (key === 'method')  f.method  = val;
				else if (key === 'ip')      f.ip      = val;
				else if (key === 'path')    f.path    = val;
				else if (key === 'ua')      f.ua      = val;
				else if (key === 'duration' && val.startsWith('>')) f.duration_gt = val.slice(1);
				else if (key === 'size' && val.startsWith('>')) f.size_gt = val.slice(1);
				else if (key === 'size' && val.startsWith('<')) f.size_lt = val.slice(1);
			}
		}
		if (bare.length) f.text = bare.join(' ');
		return f;
	}

	function buildFilterParams(): URLSearchParams {
		const f = parseQuery(query);
		const params = new URLSearchParams();
		if (f.status)      params.set('status',      f.status);
		if (f.host)        params.set('host',        f.host);
		if (f.method)      params.set('method',      f.method);
		if (f.ip)          params.set('ip',          f.ip);
		if (f.path)        params.set('path',        f.path);
		if (f.ua)          params.set('ua',          f.ua);
		if (f.duration_gt) params.set('duration_gt', f.duration_gt);
		if (f.size_gt)     params.set('size_gt',     f.size_gt);
		if (f.size_lt)     params.set('size_lt',     f.size_lt);
		if (f.text)        params.set('text',        f.text);
		if (f.not_status)  params.set('not_status',  f.not_status);
		if (f.not_host)    params.set('not_host',    f.not_host);
		if (f.not_method)  params.set('not_method',  f.not_method);
		if (f.not_ip)      params.set('not_ip',      f.not_ip);
		if (f.not_path)    params.set('not_path',    f.not_path);
		return params;
	}

	function exportCsv() {
		const params = buildFilterParams();
		const qs = params.size ? `?${params}` : '';
		const a = document.createElement('a');
		a.href = `/api/logs/export${qs}`;
		a.download = 'caddy-logs.csv';
		a.click();
	}

	async function fetchLogs() {
		loading = true;
		error = '';
		try {
			const params = buildFilterParams();
			params.set('page', String(page));
			params.set('limit', String(limit));
			const res = await fetch(`/api/logs?${params}`);
			if (res.status >= 400 && res.status < 500) {
				data = { total: 0, page: 0, limit, entries: [] };
				pageInput = '1';
				return;
			}
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

	function togglePreset(preset: string) {
		query = query.trim() === preset ? '' : preset;
		applyFilters();
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

	onMount(() => {
		const sp = get(appPage).url.searchParams;
		const parts: string[] = [];
		if (sp.get('status')) parts.push(`status:${sp.get('status')}`);
		if (sp.get('host'))   parts.push(`host:${sp.get('host')}`);
		if (sp.get('ip'))     parts.push(`ip:${sp.get('ip')}`);
		if (sp.get('path'))   parts.push(`path:${sp.get('path')}`);
		if (parts.length) query = parts.join(' ');
		fetchLogs();
	});
</script>

<div class="max-w-full space-y-6">
	<div>
		<h1 class="text-3xl font-bold">Logs</h1>
		<p class="mt-1 text-neutral-500 dark:text-white/50">Browse and filter access log entries</p>
	</div>

	<div class="space-y-2">
		<div class="flex gap-2">
			<div class="relative flex-1">
				<svg xmlns="http://www.w3.org/2000/svg" class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-neutral-400 dark:text-white/30" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
				</svg>
				<input
					bind:value={query}
					onkeydown={(e) => e.key === 'Enter' && applyFilters()}
					placeholder="status:404  path:/api/*  -ip:1.2.3.4  duration:>1000"
					class="w-full rounded-lg border border-neutral-200 bg-neutral-100 py-2 pl-9 pr-3 font-mono text-sm outline-none placeholder:font-sans placeholder:text-neutral-400 focus:border-neutral-400 dark:border-white/10 dark:bg-white/5 dark:placeholder:text-white/30 dark:focus:border-white/30"
				/>
				{#if query}
					<button onclick={() => { query = ''; applyFilters(); }} class="absolute right-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-neutral-400 hover:text-neutral-600 dark:text-white/30 dark:hover:text-white/60" aria-label="Clear">
						<svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M18 6L6 18M6 6l12 12"/></svg>
					</button>
				{/if}
			</div>
			<button
				onclick={applyFilters}
				class="rounded-lg border border-neutral-300 bg-neutral-200 px-4 py-2 text-sm hover:bg-neutral-300 dark:border-white/20 dark:bg-white/10 dark:hover:bg-white/20"
			>
				Search
			</button>
			<button
				onclick={exportCsv}
				title="Export filtered results as CSV"
				class="flex items-center gap-1.5 rounded-lg border border-neutral-200 px-3 py-2 text-sm text-neutral-600 transition-colors hover:bg-neutral-100 dark:border-white/10 dark:text-white/60 dark:hover:bg-white/5"
			>
				<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
				</svg>
				CSV
			</button>
			<button
				onclick={() => showHelp = !showHelp}
				title="Search syntax help"
				aria-label="Toggle search help"
				class="rounded-lg border px-2.5 py-2 text-sm transition-colors {showHelp
					? 'border-neutral-400 bg-neutral-200 dark:border-white/30 dark:bg-white/10'
					: 'border-neutral-200 text-neutral-500 hover:bg-neutral-100 dark:border-white/10 dark:text-white/50 dark:hover:bg-white/5'}"
			>?</button>
			<div class="flex items-center gap-2">
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
		<div class="flex flex-wrap gap-1.5">
			{#each PRESETS as preset}
				<button
					onclick={() => togglePreset(preset.query)}
					class="rounded-full border px-3 py-1 text-xs transition-colors {query.trim() === preset.query
						? 'border-neutral-400 bg-neutral-200 dark:border-white/30 dark:bg-white/10'
						: 'border-neutral-200 hover:bg-neutral-100 dark:border-white/10 dark:hover:bg-white/5'}"
				>
					{preset.label}
				</button>
			{/each}
		</div>

		{#if showHelp}
			<div class="rounded-lg border border-neutral-200 bg-neutral-50 p-4 text-xs dark:border-white/10 dark:bg-white/[0.03]">
				<div class="mb-3 font-semibold text-neutral-700 dark:text-white/70">Search syntax</div>
				<div class="grid gap-x-8 gap-y-3 sm:grid-cols-2">
					<div class="space-y-1.5">
						<div class="font-medium text-neutral-500 dark:text-white/40 uppercase tracking-wide text-[10px]">Filters</div>
						{#each [
							['status:404', 'exact status code'],
							['status:4xx', 'status class (4xx / 5xx)'],
							['status:401,403', 'multiple values'],
							['host:example.com', 'hostname (substring)'],
							['ip:1.2.3.4', 'client IP (substring)'],
							['method:POST', 'HTTP method'],
							['path:/exact/path', 'exact path match'],
							['path:/api/*', 'glob — anything under /api/'],
							['path:*login*', 'glob — path containing "login"'],
							['ua:bot', 'user-agent substring'],
							['duration:>500', 'response time > 500 ms'],
							['size:>10000', 'response size > 10 000 bytes'],
							['size:<1000', 'response size < 1 000 bytes'],
						] as [ex, desc]}
							<div class="flex items-baseline gap-2">
								<code class="shrink-0 rounded bg-neutral-200 px-1.5 py-0.5 font-mono text-neutral-700 dark:bg-white/10 dark:text-white/70">{ex}</code>
								<span class="text-neutral-500 dark:text-white/40">{desc}</span>
							</div>
						{/each}
					</div>
					<div class="space-y-1.5">
						<div class="font-medium text-neutral-500 dark:text-white/40 uppercase tracking-wide text-[10px]">Negation — prefix any filter with <code class="rounded bg-neutral-200 px-1 dark:bg-white/10">-</code></div>
						{#each [
							['-status:200', 'exclude status 200'],
							['-ip:1.2.3.4', 'exclude an IP'],
							['-host:cdn.example.com', 'exclude a host'],
							['-method:GET', 'exclude GET requests'],
							['-path:/healthz', 'exclude a path'],
							['-path:/static/*', 'exclude a path glob'],
						] as [ex, desc]}
							<div class="flex items-baseline gap-2">
								<code class="shrink-0 rounded bg-neutral-200 px-1.5 py-0.5 font-mono text-neutral-700 dark:bg-white/10 dark:text-white/70">{ex}</code>
								<span class="text-neutral-500 dark:text-white/40">{desc}</span>
							</div>
						{/each}
						<div class="mt-3 space-y-1.5">
							<div class="font-medium text-neutral-500 dark:text-white/40 uppercase tracking-wide text-[10px]">Bare text</div>
							<div class="flex items-baseline gap-2">
								<code class="shrink-0 rounded bg-neutral-200 px-1.5 py-0.5 font-mono text-neutral-700 dark:bg-white/10 dark:text-white/70">login</code>
								<span class="text-neutral-500 dark:text-white/40">searches path, host, and IP</span>
							</div>
							<div class="pt-1 text-neutral-400 dark:text-white/30">Combine tokens: <code class="rounded bg-neutral-200 px-1 dark:bg-white/10">status:5xx -path:/healthz method:GET</code></div>
						</div>
					</div>
				</div>
			</div>
		{/if}
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
					{#if data.entries.length === 0}
						<tr>
							<td colspan="8" class="px-4 py-10 text-center text-sm text-neutral-400 dark:text-white/30">No entries found</td>
						</tr>
					{/if}
					{#each data.entries as entry}
						<tr
							onclick={() => selectedEntry = entry}
							class="cursor-pointer border-b border-neutral-100 hover:bg-neutral-50 dark:border-white/5 dark:hover:bg-white/5"
						>
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

{#if selectedEntry}
	<LogEntryModal entry={selectedEntry} onclose={() => selectedEntry = null} />
{/if}
