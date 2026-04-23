<script lang="ts">
	import { onMount } from 'svelte';
	import { timeRange } from '$lib/time-range.svelte';
	import TimeRangeSelector from '$lib/components/TimeRangeSelector.svelte';
	import { anonymize } from '$lib/anonymize.svelte';

	interface EndpointStat {
		method: string;
		path: string;
		errors: number;
		codes: [number, number][];
	}

	interface IpReport {
		ip: string;
		total: number;
		errors_4xx: number;
		errors_5xx: number;
		top_endpoints: EndpointStat[];
	}

	interface LargePayload {
		ts: number;
		method: string;
		host: string;
		uri: string;
		status: number;
		size: number;
		ip: string;
		duration: number;
	}

	let data = $state<IpReport[]>([]);
	let payloads = $state<LargePayload[]>([]);
	let loading = $state(true);
	let payloadsLoading = $state(true);
	let error = $state('');
	let expandedIp = $state<string | null>(null);

	function formatBytes(b: number) {
		if (b < 1024) return `${b} B`;
		if (b < 1024 ** 2) return `${(b / 1024).toFixed(1)} KB`;
		if (b < 1024 ** 3) return `${(b / 1024 ** 2).toFixed(2)} MB`;
		return `${(b / 1024 ** 3).toFixed(2)} GB`;
	}

	function formatDuration(d: number) {
		const ms = d * 1000;
		return ms < 1 ? `${(ms * 1000).toFixed(0)}µs` : `${ms.toFixed(1)}ms`;
	}

	async function fetchReport() {
		loading = true;
		payloadsLoading = true;
		error = '';
		try {
			const params = new URLSearchParams();
			const since = timeRange.sinceParam();
			if (since) params.set('since', since);
			const qs = params.size ? `?${params}` : '';
			const [errRes, payRes] = await Promise.all([
				fetch(`/api/reports/error-rates${qs}`),
				fetch(`/api/reports/large-payloads${qs}`)
			]);
			if (!errRes.ok) throw new Error(`HTTP ${errRes.status}`);
			if (!payRes.ok) throw new Error(`HTTP ${payRes.status}`);
			data = await errRes.json();
			payloads = await payRes.json();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to fetch report';
		} finally {
			loading = false;
			payloadsLoading = false;
		}
	}

	function errorRate(row: IpReport) {
		return ((row.errors_4xx + row.errors_5xx) / row.total) * 100;
	}

	function rateColor(rate: number) {
		if (rate >= 80) return 'text-red-500 dark:text-red-400';
		if (rate >= 50) return 'text-orange-500 dark:text-orange-400';
		if (rate >= 20) return 'text-amber-500 dark:text-amber-400';
		return 'text-neutral-500 dark:text-white/50';
	}

	function rateBadgeBg(rate: number) {
		if (rate >= 80) return 'bg-red-100 text-red-700 dark:bg-red-500/15 dark:text-red-400';
		if (rate >= 50) return 'bg-orange-100 text-orange-700 dark:bg-orange-500/15 dark:text-orange-400';
		if (rate >= 20) return 'bg-amber-100 text-amber-700 dark:bg-amber-500/15 dark:text-amber-400';
		return 'bg-neutral-100 text-neutral-600 dark:bg-white/5 dark:text-white/50';
	}

	function methodColor(m: string) {
		const map: Record<string, string> = {
			GET: 'text-green-600 dark:text-green-400',
			POST: 'text-blue-600 dark:text-blue-400',
			PUT: 'text-yellow-600 dark:text-yellow-400',
			DELETE: 'text-red-600 dark:text-red-400',
			PATCH: 'text-orange-600 dark:text-orange-400',
		};
		return map[m] ?? 'text-neutral-500';
	}

	function codeBadge(code: number) {
		if (code >= 500) return 'bg-red-100 text-red-700 dark:bg-red-500/15 dark:text-red-400';
		if (code >= 400) return 'bg-amber-100 text-amber-700 dark:bg-amber-500/15 dark:text-amber-400';
		return 'bg-neutral-100 text-neutral-600 dark:bg-white/5 dark:text-white/50';
	}

	function statusColor(code: number) {
		if (code >= 500) return 'text-red-600 dark:text-red-400';
		if (code >= 400) return 'text-amber-600 dark:text-amber-400';
		if (code >= 300) return 'text-blue-600 dark:text-blue-400';
		return 'text-green-600 dark:text-green-400';
	}

	function logsLink(ip: string) {
		return `/logs?ip=${encodeURIComponent(ip)}&status=4xx,5xx`;
	}

	onMount(fetchReport);
</script>

<div class="mx-auto max-w-6xl space-y-6">
	<div class="flex flex-wrap items-end justify-between gap-4">
		<div>
			<h1 class="text-3xl font-bold">Reports</h1>
			<p class="mt-1 text-neutral-500 dark:text-white/50">Suspicious &amp; problematic activity</p>
		</div>
		<TimeRangeSelector onchange={fetchReport} />
	</div>

	<!-- Section: High Error Rate IPs -->
	<div class="space-y-3">
		<div class="flex items-center gap-2">
			<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-red-500">
				<path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>
			</svg>
			<h2 class="font-semibold">High Error Rate by IP</h2>
			{#if !loading}
				<span class="rounded-full bg-neutral-100 px-2 py-0.5 text-xs text-neutral-500 dark:bg-white/10 dark:text-white/50">{data.length} IPs</span>
			{/if}
		</div>

		{#if error}
			<div class="rounded-lg border border-red-200 bg-red-50 p-4 text-red-600 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-400">{error}</div>
		{:else if loading}
			<div class="rounded-lg border border-neutral-200 p-8 text-center text-sm text-neutral-400 dark:border-white/10 dark:text-white/30">Loading…</div>
		{:else if data.length === 0}
			<div class="rounded-lg border border-neutral-200 p-8 text-center text-sm text-neutral-400 dark:border-white/10 dark:text-white/30">No suspicious activity detected in this time range.</div>
		{:else}
			<div class="overflow-hidden rounded-lg border border-neutral-200 dark:border-white/10">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-neutral-200 bg-neutral-50 text-left dark:border-white/10 dark:bg-white/5">
							<th class="px-4 py-3 font-medium text-neutral-500 dark:text-white/50">IP Address</th>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50">Requests</th>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50">4xx</th>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50">5xx</th>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50">Error Rate</th>
							<th class="px-4 py-3 font-medium text-neutral-500 dark:text-white/50">Top Error Endpoints</th>
							<th class="px-4 py-3"></th>
						</tr>
					</thead>
					<tbody class="divide-y divide-neutral-100 dark:divide-white/5">
						{#each data as row (row.ip)}
							{@const rate = errorRate(row)}
							<tr class="transition-colors hover:bg-neutral-50 dark:hover:bg-white/[0.03]">
								<td class="px-4 py-3">
									<span class="font-mono text-xs {anonymize.on ? 'blur-sm select-none' : ''}">{row.ip}</span>
								</td>
								<td class="px-4 py-3 text-right font-mono text-xs text-neutral-500 dark:text-white/50">{row.total.toLocaleString()}</td>
								<td class="px-4 py-3 text-right font-mono text-xs text-amber-600 dark:text-amber-400">{row.errors_4xx.toLocaleString()}</td>
								<td class="px-4 py-3 text-right font-mono text-xs text-red-600 dark:text-red-400">{row.errors_5xx.toLocaleString()}</td>
								<td class="px-4 py-3 text-right">
									<span class="rounded-full px-2 py-0.5 text-xs font-semibold {rateBadgeBg(rate)}">
										{rate.toFixed(1)}%
									</span>
								</td>
								<td class="px-4 py-3">
									<div class="flex flex-wrap gap-1">
										{#each row.top_endpoints.slice(0, 3) as ep}
											<span class="inline-flex max-w-xs items-center gap-1 overflow-hidden rounded border border-neutral-200 bg-neutral-50 px-1.5 py-0.5 dark:border-white/10 dark:bg-white/5">
												<span class="shrink-0 text-xs font-semibold {methodColor(ep.method)}">{ep.method}</span>
												<span class="truncate font-mono text-xs text-neutral-600 dark:text-white/60" title={ep.path}>{ep.path}</span>
												<span class="shrink-0 text-xs text-neutral-400 dark:text-white/30">×{ep.errors}</span>
											</span>
										{/each}
										{#if row.top_endpoints.length > 3}
											<button
												onclick={() => expandedIp = expandedIp === row.ip ? null : row.ip}
												class="rounded border border-neutral-200 px-1.5 py-0.5 text-xs text-neutral-400 hover:bg-neutral-100 dark:border-white/10 dark:text-white/30 dark:hover:bg-white/5"
											>+{row.top_endpoints.length - 3} more</button>
										{/if}
									</div>
								</td>
								<td class="px-4 py-3 text-right">
									<a
										href={logsLink(row.ip)}
										class="rounded border border-neutral-200 px-2 py-1 text-xs text-neutral-500 transition-colors hover:bg-neutral-100 dark:border-white/10 dark:text-white/50 dark:hover:bg-white/5"
									>Logs →</a>
								</td>
							</tr>
							{#if expandedIp === row.ip}
								<tr class="bg-neutral-50 dark:bg-white/[0.02]">
									<td colspan="7" class="px-4 pb-4 pt-1">
										<table class="w-full text-xs">
											<thead>
												<tr class="text-left text-neutral-400 dark:text-white/30">
													<th class="pb-1 pr-4 font-medium">Method</th>
													<th class="pb-1 pr-4 font-medium">Endpoint</th>
													<th class="pb-1 pr-4 text-right font-medium">Errors</th>
													<th class="pb-1 font-medium">Status Codes</th>
												</tr>
											</thead>
											<tbody class="divide-y divide-neutral-100 dark:divide-white/5">
												{#each row.top_endpoints as ep}
													<tr>
														<td class="py-1.5 pr-4 font-semibold {methodColor(ep.method)}">{ep.method}</td>
														<td class="py-1.5 pr-4 font-mono text-neutral-600 dark:text-white/60">{ep.path}</td>
														<td class="py-1.5 pr-4 text-right font-mono text-neutral-500 dark:text-white/50">{ep.errors}</td>
														<td class="py-1.5">
															<div class="flex flex-wrap gap-1">
																{#each ep.codes as [code, count]}
																	<span class="rounded px-1.5 py-0.5 font-mono text-xs {codeBadge(code)}">{code} ×{count}</span>
																{/each}
															</div>
														</td>
													</tr>
												{/each}
											</tbody>
										</table>
									</td>
								</tr>
							{/if}
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</div>

	<!-- Section: Large Payloads -->
	<div class="space-y-3">
		<div class="flex items-center gap-2">
			<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-amber-500">
				<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/>
			</svg>
			<h2 class="font-semibold">Largest Response Payloads</h2>
			<span class="text-xs text-neutral-400 dark:text-white/30">top 100</span>
		</div>

		{#if payloadsLoading}
			<div class="rounded-lg border border-neutral-200 p-8 text-center text-sm text-neutral-400 dark:border-white/10 dark:text-white/30">Loading…</div>
		{:else if payloads.length === 0}
			<div class="rounded-lg border border-neutral-200 p-8 text-center text-sm text-neutral-400 dark:border-white/10 dark:text-white/30">No data in this time range.</div>
		{:else}
			<div class="overflow-hidden rounded-lg border border-neutral-200 dark:border-white/10">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-neutral-200 bg-neutral-50 text-left dark:border-white/10 dark:bg-white/5">
							<th class="px-4 py-3 font-medium text-neutral-500 dark:text-white/50">Time</th>
							<th class="px-4 py-3 font-medium text-neutral-500 dark:text-white/50">Method</th>
							<th class="px-4 py-3 font-medium text-neutral-500 dark:text-white/50">Host + Path</th>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50">Status</th>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50">Size</th>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50">Duration</th>
							<th class="px-4 py-3 font-medium text-neutral-500 dark:text-white/50">IP</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-neutral-100 dark:divide-white/5">
						{#each payloads as row}
							<tr class="transition-colors hover:bg-neutral-50 dark:hover:bg-white/[0.03]">
								<td class="px-4 py-2 font-mono text-xs text-neutral-500 dark:text-white/50">{new Date(row.ts * 1000).toLocaleString()}</td>
								<td class="px-4 py-2 font-mono text-xs font-semibold {methodColor(row.method)}">{row.method}</td>
								<td class="max-w-xs truncate px-4 py-2 font-mono text-xs text-neutral-600 dark:text-white/70" title="{row.host}{row.uri}">{row.host}{row.uri}</td>
								<td class="px-4 py-2 text-right font-mono text-xs font-semibold {statusColor(row.status)}">{row.status}</td>
								<td class="px-4 py-2 text-right font-mono text-xs font-semibold text-neutral-700 dark:text-white/80">{formatBytes(row.size)}</td>
								<td class="px-4 py-2 text-right font-mono text-xs text-neutral-500 dark:text-white/50">{formatDuration(row.duration)}</td>
								<td class="px-4 py-2 font-mono text-xs {anonymize.on ? 'blur-sm select-none' : ''} text-neutral-400 dark:text-white/40">{row.ip}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</div>
</div>
