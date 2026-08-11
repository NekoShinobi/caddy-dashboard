<script lang="ts">
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { resolve } from '$app/paths';
	import { marked } from 'marked';
	import DOMPurify from 'dompurify';
	import AppDialog from '$lib/components/AppDialog.svelte';
	import { timeRange } from '$lib/time-range.svelte';
	import TimeRangeSelector from '$lib/components/TimeRangeSelector.svelte';
	import { anonymize } from '$lib/anonymize.svelte';
	import { colorTheme } from '$lib/color-theme.svelte';
	import { theme } from '$lib/theme.svelte';
	import { auth } from '$lib/auth.svelte';

	const renderer = new marked.Renderer();
	marked.setOptions({ renderer, gfm: true, breaks: true });

	function renderMarkdown(text: string) {
		const html = marked.parse(text) as string;
		return browser ? DOMPurify.sanitize(html, { USE_PROFILES: { html: true } }) : '';
	}

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

	// AI analysis
	let aiText = $state('');
	let aiRunning = $state(false);
	let aiError = $state('');
	let aiDone = $state(false);
	let aiSavedAt = $state<Date | null>(null);
	let sanitizedAiHtml = $derived(renderMarkdown(aiText));

	const AI_STORAGE_KEY = 'caddy-dashboard:ai-analysis';

	onMount(() => {
		try {
			const saved = localStorage.getItem(AI_STORAGE_KEY);
			if (saved) {
				const { text, ts } = JSON.parse(saved);
				if (text) {
					aiText = text;
					aiDone = true;
					aiSavedAt = new Date(ts);
				}
			}
		} catch {}
	});

	async function runAiAnalysis() {
		aiText = '';
		aiError = '';
		aiDone = false;
		aiSavedAt = null;
		aiRunning = true;
		try {
			const res = await fetch('/api/reports/ai-analysis');
			if (!res.ok || !res.body) {
				const data = await res.json().catch(() => ({}));
				throw new Error(data.error ?? `HTTP ${res.status}`);
			}
			const reader = res.body.getReader();
			const decoder = new TextDecoder();
			let buf = '';
			while (true) {
				const { done, value } = await reader.read();
				if (done) break;
				buf += decoder.decode(value, { stream: true });
				const lines = buf.split('\n');
				buf = lines.pop() ?? '';
				for (const line of lines) {
					if (!line.startsWith('data: ')) continue;
					try {
						const evt = JSON.parse(line.slice(6));
						if (evt.token) aiText += evt.token;
						if (evt.error) aiError = evt.error;
						if (evt.done) aiDone = true;
					} catch {}
				}
			}
		} catch (e) {
			aiError = e instanceof Error ? e.message : 'Analysis failed';
		} finally {
			aiRunning = false;
			aiDone = true;
			if (aiText && !aiError) {
				const now = new Date();
				aiSavedAt = now;
				try {
					localStorage.setItem(
						AI_STORAGE_KEY,
						JSON.stringify({ text: aiText, ts: now.toISOString() })
					);
				} catch {}
			}
		}
	}

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
			if (!errRes.ok) {
				const d = await errRes.json().catch(() => ({}));
				throw new Error(d.error ?? `HTTP ${errRes.status}`);
			}
			if (!payRes.ok) {
				const d = await payRes.json().catch(() => ({}));
				throw new Error(d.error ?? `HTTP ${payRes.status}`);
			}
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

	function tc() {
		return colorTheme.theme[theme.dark ? 'dark' : 'light'];
	}

	function statusColor(code: number): string {
		const t = tc();
		if (code >= 500) return t.red;
		if (code >= 400) return t.yellow;
		if (code >= 300) return t.blue;
		return t.green;
	}

	function methodColor(m: string): string {
		const t = tc();
		const map: Record<string, string> = {
			GET: t.green,
			POST: t.blue,
			PUT: t.yellow,
			DELETE: t.red,
			PATCH: t.orange
		};
		return map[m] ?? (theme.dark ? 'rgba(255,255,255,0.5)' : '#6b7280');
	}

	function rateColor(rate: number): string {
		const t = tc();
		if (rate >= 80) return t.red;
		if (rate >= 50) return t.orange;
		if (rate >= 20) return t.yellow;
		return theme.dark ? 'rgba(255,255,255,0.5)' : '#6b7280';
	}

	function rateBadgeStyle(rate: number): string {
		const col = rateColor(rate);
		if (rate < 20)
			return theme.dark
				? 'color:rgba(255,255,255,0.5);background:rgba(255,255,255,0.05)'
				: 'color:#6b7280;background:#f5f5f5';
		return `color:${col};background-color:${col}1a`;
	}

	function codeBadgeStyle(code: number): string {
		const t = tc();
		let col: string;
		if (code >= 500) col = t.red;
		else if (code >= 400) col = t.yellow;
		else
			return theme.dark
				? 'color:rgba(255,255,255,0.5);background:rgba(255,255,255,0.05)'
				: 'color:#6b7280;background:#f5f5f5';
		return `color:${col};background-color:${col}1a`;
	}

	let selectedIpReport = $state<IpReport | null>(null);
	let selectedPayload = $state<LargePayload | null>(null);

	function logsLink(ip: string): `/logs?${string}` {
		return `/logs?ip=${encodeURIComponent(ip)}&status=4xx,5xx`;
	}

	onMount(fetchReport);
</script>

<div class="page-shell" data-od-id="reports-page">
	<header class="page-header" data-od-id="reports-header">
		<div>
			<p class="page-eyebrow">Investigation</p>
			<h1 class="page-title">Traffic reports</h1>
			<p class="page-description">
				Surface suspicious clients, error-heavy endpoints, and unusually large responses.
			</p>
		</div>
		<TimeRangeSelector onchange={fetchReport} />
	</header>

	<!-- Section: AI Analysis (admin only) -->
	{#if auth.user?.is_admin}
		<div class="space-y-3">
			<div class="flex items-center gap-3">
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					class="text-violet-500"
				>
					<path d="M12 2a10 10 0 1 0 10 10" /><path d="M12 8v4l3 3" /><circle
						cx="18"
						cy="6"
						r="4"
						fill="currentColor"
						class="text-violet-500"
					/>
				</svg>
				<h2 class="font-semibold">AI Traffic Analysis</h2>
				<span class="text-xs text-neutral-400 dark:text-white/30">last 24 hours · via Ollama</span>
				{#if aiSavedAt && !aiRunning}
					<span class="text-xs text-neutral-400 dark:text-white/30"
						>· saved {aiSavedAt.toLocaleString()}</span
					>
				{/if}
				<button
					onclick={runAiAnalysis}
					disabled={aiRunning}
					class="ml-auto flex items-center gap-1.5 rounded-lg border px-3 py-1.5 text-sm transition-colors disabled:opacity-50
					{aiRunning
						? 'border-violet-300 bg-violet-50 text-violet-600 dark:border-violet-500/30 dark:bg-violet-500/10 dark:text-violet-400'
						: 'border-neutral-200 hover:bg-neutral-100 dark:border-white/10 dark:hover:bg-white/5'}"
				>
					{#if aiRunning}
						<svg
							class="h-3.5 w-3.5 animate-spin"
							xmlns="http://www.w3.org/2000/svg"
							fill="none"
							viewBox="0 0 24 24"
						>
							<circle
								class="opacity-25"
								cx="12"
								cy="12"
								r="10"
								stroke="currentColor"
								stroke-width="4"
							/>
							<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8v8z" />
						</svg>
						Analysing…
					{:else}
						<svg
							xmlns="http://www.w3.org/2000/svg"
							class="h-3.5 w-3.5"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"><polygon points="5 3 19 12 5 21 5 3" /></svg
						>
						{aiDone ? 'Re-run' : 'Run analysis'}
					{/if}
				</button>
			</div>

			{#if aiError}
				<div
					class="rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-600 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-400"
				>
					{aiError}
				</div>
			{/if}

			{#if aiText}
				<div
					class="ai-output rounded-lg border border-neutral-200 bg-neutral-50 p-5 dark:border-white/10 dark:bg-white/[0.03]"
				>
					{@html sanitizedAiHtml}{#if aiRunning}<span
							class="ml-0.5 inline-block h-4 w-0.5 animate-pulse bg-current align-middle opacity-70"
						></span>{/if}
				</div>
			{:else if !aiRunning && !aiDone}
				<div
					class="rounded-lg border border-dashed border-neutral-200 p-8 text-center text-sm text-neutral-400 dark:border-white/10 dark:text-white/30"
				>
					Click <strong>Run analysis</strong> to send the last 24 hours of traffic stats to your
					local Ollama instance for review.
					<div class="mt-1 text-xs">
						Requires <code class="rounded bg-neutral-100 px-1 dark:bg-white/10">OLLAMA_HOST</code>
						reachable and
						<code class="rounded bg-neutral-100 px-1 dark:bg-white/10">OLLAMA_MODEL</code> pulled.
					</div>
				</div>
			{/if}
		</div>
	{/if}

	<!-- Section: High Error Rate IPs -->
	<div class="space-y-3">
		<div class="flex items-center gap-2">
			<svg
				xmlns="http://www.w3.org/2000/svg"
				width="16"
				height="16"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
				class="text-red-500"
			>
				<path
					d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"
				/><line x1="12" y1="9" x2="12" y2="13" /><line x1="12" y1="17" x2="12.01" y2="17" />
			</svg>
			<h2 class="font-semibold">High Error Rate by IP</h2>
			{#if !loading}
				<span
					class="rounded-full bg-neutral-100 px-2 py-0.5 text-xs text-neutral-500 dark:bg-white/10 dark:text-white/50"
					>{data.length} IPs</span
				>
			{/if}
		</div>

		{#if error}
			<div
				class="rounded-lg border border-red-200 bg-red-50 p-4 text-red-600 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-400"
			>
				{error}
			</div>
		{:else if loading}
			<div
				class="rounded-lg border border-neutral-200 p-8 text-center text-sm text-neutral-400 dark:border-white/10 dark:text-white/30"
			>
				Loading…
			</div>
		{:else if data.length === 0}
			<div
				class="rounded-lg border border-neutral-200 p-8 text-center text-sm text-neutral-400 dark:border-white/10 dark:text-white/30"
			>
				No suspicious activity detected in this time range.
			</div>
		{:else}
			<div class="data-table-wrap">
				<table class="w-full text-sm">
					<thead>
						<tr
							class="border-b border-neutral-200 bg-neutral-50 text-left dark:border-white/10 dark:bg-white/5"
						>
							<th class="px-4 py-3 font-medium text-neutral-500 dark:text-white/50">IP Address</th>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50"
								>Requests</th
							>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50"
								>4xx</th
							>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50"
								>5xx</th
							>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50"
								>Error Rate</th
							>
							<th class="px-4 py-3 font-medium text-neutral-500 dark:text-white/50"
								>Top Error Endpoints</th
							>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50"
								>Actions</th
							>
						</tr>
					</thead>
					<tbody class="divide-y divide-neutral-100 dark:divide-white/5">
						{#each data as row (row.ip)}
							{@const rate = errorRate(row)}
							<tr class="interactive-row">
								<td class="px-4 py-3">
									<span class="font-mono text-xs {anonymize.on ? 'blur-sm select-none' : ''}"
										>{row.ip}</span
									>
								</td>
								<td
									class="px-4 py-3 text-right font-mono text-xs text-neutral-500 dark:text-white/50"
									>{row.total.toLocaleString()}</td
								>
								<td
									class="px-4 py-3 text-right font-mono text-xs font-semibold"
									style="color:{tc().yellow}">{row.errors_4xx.toLocaleString()}</td
								>
								<td
									class="px-4 py-3 text-right font-mono text-xs font-semibold"
									style="color:{tc().red}">{row.errors_5xx.toLocaleString()}</td
								>
								<td class="px-4 py-3 text-right">
									<span
										class="rounded-full px-2 py-0.5 text-xs font-semibold"
										style={rateBadgeStyle(rate)}
									>
										{rate.toFixed(1)}%
									</span>
								</td>
								<td class="px-4 py-3">
									<div class="flex flex-wrap gap-1">
										{#each row.top_endpoints.slice(0, 3) as ep (`${ep.method}-${ep.path}`)}
											<span
												class="inline-flex max-w-xs items-center gap-1 overflow-hidden rounded border border-neutral-200 bg-neutral-50 px-1.5 py-0.5 dark:border-white/10 dark:bg-white/5"
											>
												<span
													class="shrink-0 text-xs font-semibold"
													style="color:{methodColor(ep.method)}">{ep.method}</span
												>
												<span
													class="truncate font-mono text-xs text-neutral-600 dark:text-white/60 {anonymize.on
														? 'blur-sm select-none'
														: ''}"
													title={anonymize.on ? '' : ep.path}>{ep.path}</span
												>
												<span class="shrink-0 text-xs text-neutral-400 dark:text-white/30"
													>×{ep.errors}</span
												>
											</span>
										{/each}
										{#if row.top_endpoints.length > 3}
											<span
												class="rounded border border-neutral-200 px-1.5 py-0.5 text-xs text-neutral-400 dark:border-white/10 dark:text-white/30"
												>+{row.top_endpoints.length - 3} more</span
											>
										{/if}
									</div>
								</td>
								<td class="px-4 py-3 text-right">
									<div class="flex justify-end gap-2">
										<button
											type="button"
											class="button-secondary text-xs whitespace-nowrap"
											onclick={() => (selectedIpReport = row)}
										>
											Details
										</button>
										<a
											href={resolve(logsLink(row.ip))}
											class="button-secondary text-xs whitespace-nowrap"
										>
											Logs
										</a>
									</div>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</div>

	<!-- Section: Large Payloads -->
	<div class="space-y-3">
		<div class="flex items-center gap-2">
			<svg
				xmlns="http://www.w3.org/2000/svg"
				width="16"
				height="16"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
				class="text-amber-500"
			>
				<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline
					points="17 8 12 3 7 8"
				/><line x1="12" y1="3" x2="12" y2="15" />
			</svg>
			<h2 class="font-semibold">Largest Response Payloads</h2>
			<span class="text-xs text-neutral-400 dark:text-white/30">top 100</span>
		</div>

		{#if payloadsLoading}
			<div
				class="rounded-lg border border-neutral-200 p-8 text-center text-sm text-neutral-400 dark:border-white/10 dark:text-white/30"
			>
				Loading…
			</div>
		{:else if payloads.length === 0}
			<div
				class="rounded-lg border border-neutral-200 p-8 text-center text-sm text-neutral-400 dark:border-white/10 dark:text-white/30"
			>
				No data in this time range.
			</div>
		{:else}
			<div class="data-table-wrap">
				<table class="w-full text-sm">
					<thead>
						<tr
							class="border-b border-neutral-200 bg-neutral-50 text-left dark:border-white/10 dark:bg-white/5"
						>
							<th class="px-4 py-3 font-medium text-neutral-500 dark:text-white/50">Time</th>
							<th class="px-4 py-3 font-medium text-neutral-500 dark:text-white/50">Method</th>
							<th class="px-4 py-3 font-medium text-neutral-500 dark:text-white/50">Host + Path</th>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50"
								>Status</th
							>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50"
								>Size</th
							>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50"
								>Duration</th
							>
							<th class="px-4 py-3 font-medium text-neutral-500 dark:text-white/50">IP</th>
							<th class="px-4 py-3 text-right font-medium text-neutral-500 dark:text-white/50"
								>Action</th
							>
						</tr>
					</thead>
					<tbody class="divide-y divide-neutral-100 dark:divide-white/5">
						{#each payloads as row (`${row.ts}-${row.ip}-${row.uri}`)}
							<tr class="interactive-row">
								<td class="px-4 py-2 font-mono text-xs text-neutral-500 dark:text-white/50"
									>{new Date(row.ts * 1000).toLocaleString()}</td
								>
								<td
									class="px-4 py-2 font-mono text-xs font-semibold"
									style="color:{methodColor(row.method)}">{row.method}</td
								>
								<td
									class="max-w-xs truncate px-4 py-2 font-mono text-xs text-neutral-600 dark:text-white/70 {anonymize.on
										? 'blur-sm select-none'
										: ''}"
									title={anonymize.on ? '' : `${row.host}${row.uri}`}>{row.host}{row.uri}</td
								>
								<td
									class="px-4 py-2 text-right font-mono text-xs font-semibold"
									style="color:{statusColor(row.status)}">{row.status}</td
								>
								<td
									class="px-4 py-2 text-right font-mono text-xs font-semibold text-neutral-700 dark:text-white/80"
									>{formatBytes(row.size)}</td
								>
								<td
									class="px-4 py-2 text-right font-mono text-xs text-neutral-500 dark:text-white/50"
									>{formatDuration(row.duration)}</td
								>
								<td
									class="px-4 py-2 font-mono text-xs {anonymize.on
										? 'blur-sm select-none'
										: ''} text-neutral-400 dark:text-white/40">{row.ip}</td
								>
								<td class="px-4 py-2 text-right">
									<button
										type="button"
										class="button-secondary text-xs whitespace-nowrap"
										onclick={() => (selectedPayload = row)}>Details</button
									>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</div>
</div>

{#if selectedIpReport}
	{@const row = selectedIpReport}
	{@const rate = errorRate(row)}
	<AppDialog
		open={true}
		onOpenChange={(open) => {
			if (!open) selectedIpReport = null;
		}}
		title={anonymize.on ? 'Client report' : row.ip}
		description={`${rate.toFixed(1)}% error rate across ${row.total.toLocaleString()} requests`}
		size="lg"
	>
		<div class="mb-5 flex justify-end">
			<a href={resolve(logsLink(row.ip))} class="button-secondary">View matching logs →</a>
		</div>
		<div class="metric-board compact-metrics">
			<div class="metric-cell">
				<span class="metric-label">Total</span>
				<strong class="metric-value">{row.total.toLocaleString()}</strong>
			</div>
			<div class="metric-cell">
				<span class="metric-label">4xx errors</span>
				<strong class="metric-value" style="color:{tc().yellow}"
					>{row.errors_4xx.toLocaleString()}</strong
				>
			</div>
			<div class="metric-cell">
				<span class="metric-label">5xx errors</span>
				<strong class="metric-value" style="color:{tc().red}"
					>{row.errors_5xx.toLocaleString()}</strong
				>
			</div>
			<div class="metric-cell">
				<span class="metric-label">Error rate</span>
				<strong class="metric-value" style="color:{rateColor(rate)}">{rate.toFixed(1)}%</strong>
			</div>
		</div>
		<section class="mt-6">
			<div class="section-heading">
				<div>
					<p class="page-eyebrow">Endpoint breakdown</p>
					<h3>Error-producing routes</h3>
				</div>
			</div>
			<div class="data-table-wrap">
				<table class="w-full text-xs">
					<thead>
						<tr>
							<th class="px-3 py-2 text-left">Method</th>
							<th class="px-3 py-2 text-left">Endpoint</th>
							<th class="px-3 py-2 text-right">Errors</th>
							<th class="px-3 py-2 text-left">Status codes</th>
						</tr>
					</thead>
					<tbody>
						{#each row.top_endpoints as endpoint (`${endpoint.method}-${endpoint.path}`)}
							<tr>
								<td class="px-3 py-2 font-semibold" style="color:{methodColor(endpoint.method)}"
									>{endpoint.method}</td
								>
								<td class="px-3 py-2 font-mono {anonymize.on ? 'blur-sm select-none' : ''}"
									>{endpoint.path}</td
								>
								<td class="px-3 py-2 text-right font-mono">{endpoint.errors}</td>
								<td class="px-3 py-2">
									<div class="flex flex-wrap gap-1">
										{#each endpoint.codes as [code, count] (code)}
											<span class="rounded px-1.5 py-0.5 font-mono" style={codeBadgeStyle(code)}
												>{code} ×{count}</span
											>
										{/each}
									</div>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</section>
	</AppDialog>
{/if}

{#if selectedPayload}
	{@const row = selectedPayload}
	<AppDialog
		open={true}
		onOpenChange={(open) => {
			if (!open) selectedPayload = null;
		}}
		title="Payload details"
		description={`${row.method} · ${row.status} · ${formatBytes(row.size)}`}
		size="md"
	>
		<dl class="detail-grid">
			<dt>Time</dt>
			<dd>{new Date(row.ts * 1000).toLocaleString()}</dd>
			<dt>Host</dt>
			<dd class={anonymize.on ? 'blur-sm select-none' : ''}>{row.host}</dd>
			<dt>URI</dt>
			<dd class={anonymize.on ? 'blur-sm select-none' : ''}>{row.uri}</dd>
			<dt>Method</dt>
			<dd class="font-semibold" style="color:{methodColor(row.method)}">{row.method}</dd>
			<dt>Status</dt>
			<dd class="font-semibold" style="color:{statusColor(row.status)}">{row.status}</dd>
			<dt>Response size</dt>
			<dd class="font-semibold">{formatBytes(row.size)}</dd>
			<dt>Duration</dt>
			<dd>{formatDuration(row.duration)}</dd>
			<dt>Client IP</dt>
			<dd class={anonymize.on ? 'blur-sm select-none' : ''}>{row.ip}</dd>
		</dl>
	</AppDialog>
{/if}

<style>
	.compact-metrics {
		grid-template-columns: repeat(4, minmax(0, 1fr));
	}
	.detail-grid {
		display: grid;
		grid-template-columns: minmax(7rem, auto) minmax(0, 1fr);
		gap: 0.7rem 1.5rem;
		padding: 1rem;
		border: 1px solid var(--app-border);
		border-radius: var(--radius-sm);
		background: var(--app-surface-muted);
		font-family: var(--font-mono);
		font-size: 0.78rem;
	}
	.detail-grid dt {
		color: var(--app-muted-fg);
	}
	.detail-grid dd {
		min-width: 0;
		overflow-wrap: anywhere;
	}
	@media (max-width: 620px) {
		.compact-metrics {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
		.detail-grid {
			grid-template-columns: 1fr;
			gap: 0.2rem;
		}
		.detail-grid dd {
			margin-bottom: 0.7rem;
		}
	}
	:global(.ai-output h1, .ai-output h2, .ai-output h3) {
		font-weight: 600;
		margin-top: 1rem;
		margin-bottom: 0.25rem;
	}
	:global(.ai-output h1) {
		font-size: 1.1rem;
	}
	:global(.ai-output h2) {
		font-size: 1rem;
	}
	:global(.ai-output h3) {
		font-size: 0.9rem;
	}
	:global(.ai-output p) {
		margin-bottom: 0.5rem;
		font-size: 0.875rem;
		line-height: 1.6;
	}
	:global(.ai-output ul, .ai-output ol) {
		padding-left: 1.25rem;
		margin-bottom: 0.5rem;
	}
	:global(.ai-output li) {
		font-size: 0.875rem;
		line-height: 1.6;
		margin-bottom: 0.2rem;
	}
	:global(.ai-output ul) {
		list-style-type: disc;
	}
	:global(.ai-output ol) {
		list-style-type: decimal;
	}
	:global(.ai-output strong) {
		font-weight: 600;
	}
	:global(.ai-output em) {
		font-style: italic;
	}
	:global(.ai-output code) {
		font-family: monospace;
		font-size: 0.8rem;
		background: rgba(0, 0, 0, 0.06);
		border-radius: 0.25rem;
		padding: 0.1rem 0.3rem;
	}
	:global(.dark .ai-output code) {
		background: rgba(255, 255, 255, 0.08);
	}
	:global(.ai-output hr) {
		border: none;
		border-top: 1px solid rgba(0, 0, 0, 0.1);
		margin: 0.75rem 0;
	}
	:global(.dark .ai-output hr) {
		border-top-color: rgba(255, 255, 255, 0.1);
	}
</style>
