<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { Switch, Toggle } from 'bits-ui';
	import { resolve } from '$app/paths';
	import { anonymize, anonHost, anonIP, anonPathKey } from '$lib/anonymize.svelte';
	import { colorTheme } from '$lib/color-theme.svelte';
	import { theme } from '$lib/theme.svelte';
	import { timeRange } from '$lib/time-range.svelte';
	import TimeRangeSelector from './TimeRangeSelector.svelte';
	import UAModal from './UAModal.svelte';
	import {
		CategoryScale,
		Chart,
		Filler,
		LinearScale,
		LineController,
		LineElement,
		PointElement,
		Tooltip
	} from 'chart.js';

	Chart.register(LineController, LineElement, PointElement, LinearScale, CategoryScale, Filler, Tooltip);

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

	interface Bucket {
		ts: number;
		total: number;
	}

	let selectedUA = $state<string | null>(null);
	let stats = $state.raw<Stats | null>(null);
	let buckets = $state.raw<Bucket[]>([]);
	let loading = $state(true);
	let error = $state('');
	let autoRefresh = $state(true);
	let slowFilter = $state('');
	let canvasRequests = $state<HTMLCanvasElement | null>(null);
	let bucketKind = $state<'minute' | 'hour' | 'day'>('hour');
	let requestController: AbortController | null = null;
	let requestChart: Chart | null = null;

	let statusFamilies = $derived.by(() => {
		const total = stats?.total_requests ?? 0;
		const counts: Record<'2xx' | '3xx' | '4xx' | '5xx', number> = {
			'2xx': 0,
			'3xx': 0,
			'4xx': 0,
			'5xx': 0
		};
		for (const [code, count] of Object.entries(stats?.status_codes ?? {})) {
			const family = `${code.charAt(0)}xx` as keyof typeof counts;
			if (family in counts) counts[family] += count;
		}
		return Object.entries(counts).map(([family, count]) => ({
			family,
			count,
			percentage: total ? (count / total) * 100 : 0,
			color: statusColor(Number(family.charAt(0)) * 100),
			label: family === '2xx' ? 'Success' : family === '3xx' ? 'Redirect' : family === '4xx' ? 'Client error' : 'Server error'
		}));
	});

	let successfulShare = $derived(
		statusFamilies.slice(0, 2).reduce((sum, family) => sum + family.percentage, 0)
	);

	let filteredSlowPaths = $derived.by(() => {
		const query = slowFilter.trim().toLowerCase();
		const rows = stats?.slowest_paths ?? [];
		if (!query) return rows;
		return rows.filter((row) => row.path.toLowerCase().includes(query));
	});

	function statusColor(code: string | number): string {
		const n = Number(code);
		const colors = colorTheme.theme[theme.dark ? 'dark' : 'light'];
		if (n < 300) return colors.green;
		if (n < 400) return 'var(--app-muted)';
		if (n < 500) return colors.yellow;
		return colors.red;
	}

	function pathLogsLink(key: string): `/logs?${string}` {
		const i = key.indexOf('/');
		const path = i >= 0 ? key.slice(i) : key;
		const host = i >= 0 ? key.slice(0, i) : '';
		const params = new URLSearchParams();
		params.set('path', path);
		if (host) params.set('host', host);
		return `/logs?${params}`;
	}

	function formatDurationMs(value: number): string {
		if (value >= 1000) return `${(value / 1000).toFixed(2)} s`;
		if (value >= 1) return `${value.toFixed(1)} ms`;
		return `${(value * 1000).toFixed(0)} µs`;
	}

	function formatBytes(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
		if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
		return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
	}

	function timelineBucket(): 'minute' | 'hour' | 'day' {
		if (timeRange.seconds > 0 && timeRange.seconds <= 21_600) return 'minute';
		if (timeRange.seconds === 0 || timeRange.seconds > 86_400) return 'day';
		return 'hour';
	}

	function formatBucketLabel(ts: number): string {
		const date = new Date(ts * 1000);
		if (bucketKind === 'minute') return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
		if (bucketKind === 'day') return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
		return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}

	function chartTokens() {
		const source = canvasRequests?.closest('.app-shell') ?? document.documentElement;
		const styles = getComputedStyle(source);
		const token = (name: string, fallback: string) => styles.getPropertyValue(name).trim() || fallback;
		return {
			accent: token('--app-accent', 'oklch(0.72 0.13 205)'),
			accentSoft: token('--app-accent-soft', 'oklch(0.28 0.055 205)'),
			border: token('--app-border', 'oklch(0.31 0.02 230)'),
			muted: token('--app-muted', 'oklch(0.68 0.018 220)')
		};
	}

	function renderChart() {
		if (!canvasRequests || buckets.length === 0) return;
		const colors = chartTokens();
		requestChart?.destroy();
		requestChart = new Chart(canvasRequests, {
			type: 'line',
			data: {
				labels: buckets.map((bucket) => formatBucketLabel(bucket.ts)),
				datasets: [
					{
						label: 'Requests',
						data: buckets.map((bucket) => bucket.total),
						borderColor: colors.accent,
						backgroundColor: colors.accentSoft,
						borderWidth: 2,
						fill: true,
						tension: 0.32,
						pointRadius: 0,
						pointHitRadius: 10
					}
				]
			},
			options: {
				animation: false,
				responsive: true,
				maintainAspectRatio: false,
				interaction: { mode: 'index', intersect: false },
				plugins: {
					legend: { display: false },
					tooltip: { displayColors: false }
				},
				scales: {
					x: {
						grid: { display: false },
						border: { display: false },
						ticks: { color: colors.muted, maxTicksLimit: 6, maxRotation: 0, font: { size: 10 } }
					},
					y: {
						beginAtZero: true,
						grid: { color: colors.border },
						border: { display: false },
						ticks: { color: colors.muted, maxTicksLimit: 5, font: { size: 10 } }
					}
				}
			}
		});
	}

	async function fetchDashboard() {
		requestController?.abort();
		const controller = new AbortController();
		requestController = controller;
		loading = true;
		error = '';
		bucketKind = timelineBucket();

		try {
			const [statsResponse, timelineResponse] = await Promise.all([
				fetch(`/api/stats?range=${timeRange.seconds}`, { signal: controller.signal }),
				fetch(`/api/timeline?bucket=${bucketKind}`, { signal: controller.signal })
			]);
			if (!statsResponse.ok) {
				const data = await statsResponse.json().catch(() => ({}));
				throw new Error(data.error ?? `HTTP ${statsResponse.status}`);
			}
			stats = await statsResponse.json();

			if (timelineResponse.ok) {
				const timeline = await timelineResponse.json();
				const since = timeRange.sinceParam();
				buckets = (timeline.buckets as Bucket[])
					.filter((bucket) => !since || bucket.ts >= Number(since))
					.slice(-64);
			} else {
				buckets = [];
			}
		} catch (caught) {
			if (caught instanceof DOMException && caught.name === 'AbortError') return;
			error = caught instanceof Error ? caught.message : 'Failed to fetch dashboard data';
		} finally {
			if (requestController === controller) loading = false;
		}
	}

	$effect(() => {
		void theme.dark;
		void colorTheme.id;
		if (canvasRequests && buckets.length) renderChart();
	});

	onMount(() => {
		void fetchDashboard();
		const interval = setInterval(() => {
			if (autoRefresh) void fetchDashboard();
		}, 30_000);
		return () => clearInterval(interval);
	});

	onDestroy(() => {
		requestController?.abort();
		requestChart?.destroy();
	});
</script>

<div class="page-shell overview-v2" data-od-id="overview-page">
	<header class="page-header overview-header-v2" data-od-id="overview-header">
		<div>
			<p class="page-eyebrow">Live operations</p>
			<h1 class="page-title">Traffic overview</h1>
			<p class="page-description">Volume, reliability, routing, and response performance in one operational view.</p>
		</div>
		<div class="overview-actions-v2">
			<TimeRangeSelector onchange={fetchDashboard} />
			<div class="auto-refresh-option-v2" data-od-id="auto-refresh-control">
				<span>Auto refresh</span>
				<Switch.Root
					checked={autoRefresh}
					onCheckedChange={(checked) => {
						autoRefresh = checked;
						if (checked) void fetchDashboard();
					}}
					class="refresh-switch-v2"
					aria-label={autoRefresh ? 'Pause automatic refresh' : 'Resume automatic refresh'}
					title={autoRefresh ? 'Refreshing every 30 seconds' : 'Automatic refresh paused'}
				>
					<Switch.Thumb class="refresh-switch-thumb-v2" />
				</Switch.Root>
			</div>
		</div>
	</header>

	{#if error}
		<div class="status-alert status-alert-error" role="alert">
			<strong>Dashboard data unavailable.</strong> {error}
		</div>
	{/if}

	{#if loading && !stats}
		<section class="metric-board" aria-label="Loading metrics" data-od-id="loading-metrics">
			{#each ['Requests', 'Duration', 'Volume', 'Clients'] as label (label)}
				<div class="metric-cell">
					<span class="skeleton h-3 w-24"></span>
					<span class="skeleton mt-4 block h-9 w-28"></span>
				</div>
			{/each}
		</section>
		<div class="dashboard-grid-v2" aria-label="Loading dashboard panels">
			<div class="panel chart-panel-v2 dashboard-span-8"><div class="panel-pad"><span class="skeleton block h-80 w-full"></span></div></div>
			<div class="panel dashboard-span-4"><div class="panel-pad"><span class="skeleton block h-80 w-full"></span></div></div>
		</div>
	{:else if stats}
		<section class="metric-board" data-od-id="key-metrics">
			<article class="metric-cell" data-od-id="metric-requests">
				<span class="metric-label">Total requests</span>
				<strong class="metric-value">{stats.total_requests.toLocaleString()}</strong>
				<span class="metric-context">{timeRange.label} range</span>
			</article>
			<article class="metric-cell" data-od-id="metric-duration">
				<span class="metric-label">Average duration</span>
				<strong class="metric-value">{formatDurationMs(stats.avg_duration_ms)}</strong>
				<span class="metric-context">End-to-end response</span>
			</article>
			<article class="metric-cell" data-od-id="metric-volume">
				<span class="metric-label">Response volume</span>
				<strong class="metric-value">{formatBytes(stats.total_bytes)}</strong>
				<span class="metric-context">Bytes served</span>
			</article>
			<article class="metric-cell" data-od-id="metric-clients">
				<span class="metric-label">Unique clients</span>
				<strong class="metric-value">{stats.unique_clients.toLocaleString()}</strong>
				<span class="metric-context">Distinct addresses</span>
			</article>
		</section>

		{#if stats.total_requests === 0}
			<section class="empty-state panel" data-od-id="empty-traffic">
				<div>
					<strong>No traffic in this range</strong>
					<p>Choose a wider range or wait for Caddy to record new requests.</p>
				</div>
			</section>
		{:else}
			<div class="dashboard-grid-v2">
				<section class="panel chart-panel-v2 dashboard-span-8" data-od-id="request-volume-chart">
					<header class="panel-header-v2">
						<div><h2>Request volume</h2><p>Traffic across the selected range</p></div>
						<span class="panel-meta-v2">{bucketKind} buckets</span>
					</header>
					<div class="chart-frame-v2">
						{#if buckets.length}
							<canvas bind:this={canvasRequests} aria-label="Filled line chart of request volume over time"></canvas>
						{:else}
							<div class="chart-empty-v2"><strong>No timeline data</strong><span>Request totals are available, but no timeline buckets were returned.</span></div>
						{/if}
					</div>
				</section>

				<section class="panel dashboard-span-4" data-od-id="status-distribution">
					<header class="panel-header-v2">
						<div><h2>Status distribution</h2><p>Share by response family</p></div>
						<span class="panel-meta-v2">HTTP</span>
					</header>
					<div class="status-body-v2">
						<div class="success-rate-v2"><strong>{successfulShare.toFixed(1)}%</strong><span>successful or redirected</span></div>
						<div class="family-band-v2" aria-label="Status distribution">
							{#each statusFamilies as family (family.family)}
								<span
									class="family-segment-v2"
									style:--share={`${family.percentage}%`}
									style:--tone={family.color}
									title={`${family.family} ${family.percentage.toFixed(1)}%`}
								></span>
							{/each}
						</div>
						<ul class="family-list-v2">
							{#each statusFamilies as family (family.family)}
								<li>
									<span class="family-swatch-v2" style:--tone={family.color}></span>
									<span><strong>{family.family}</strong><small>{family.label}</small></span>
									<span class="family-count-v2">{family.count.toLocaleString()}</span>
								</li>
							{/each}
						</ul>
					</div>
				</section>

				<section class="panel dashboard-span-6" data-od-id="top-paths">
					<header class="panel-header-v2">
						<div><h2>Top routes</h2><p>Most requested paths</p></div>
						<a href={resolve('/logs')} class="panel-link-v2">Explore logs</a>
					</header>
					<ol class="rank-list-v2">
						{#each stats.top_paths.slice(0, 6) as [path, count], index (path)}
							<li><span class="rank-index-v2">{String(index + 1).padStart(2, '0')}</span><a href={resolve(pathLogsLink(path))}>{anonymize.on ? anonPathKey(path) : path}</a><span>{count.toLocaleString()}</span></li>
						{/each}
					</ol>
				</section>

				<section class="panel dashboard-span-6" data-od-id="top-clients">
					<header class="panel-header-v2">
						<div><h2>Top clients</h2><p>Highest request volume by address</p></div>
						<span class="panel-meta-v2">Requests</span>
					</header>
					<ol class="rank-list-v2">
						{#each stats.top_ips.slice(0, 6) as [ip, count], index (ip)}
							<li><span class="rank-index-v2">{String(index + 1).padStart(2, '0')}</span><a href={resolve(`/logs?ip=${encodeURIComponent(ip)}`)}>{anonymize.on ? anonIP(ip) : ip}</a><span>{count.toLocaleString()}</span></li>
						{/each}
					</ol>
				</section>

				{#if (stats.slowest_paths ?? []).length > 0}
					<section class="panel dashboard-span-12" data-od-id="slowest-paths">
						<header class="panel-header-v2 table-header-v2">
							<div><h2>Slowest paths</h2><p>Sorted by p99 latency</p></div>
							<label class="filter-field-v2" for="slow-path-filter"><span>Filter</span><input id="slow-path-filter" type="search" bind:value={slowFilter} placeholder="Filter path" /></label>
						</header>
						<div class="table-wrap-v2">
							<table>
								<thead><tr><th>Path</th><th>Requests</th><th>Average</th><th>p99</th></tr></thead>
								<tbody>
									{#each filteredSlowPaths as row (row.path)}
										<tr><td><a href={resolve(pathLogsLink(row.path))}>{anonymize.on ? anonPathKey(row.path) : row.path}</a></td><td>{row.count.toLocaleString()}</td><td>{formatDurationMs(row.avg_ms)}</td><td class="latency-v2">{formatDurationMs(row.p99_ms)}</td></tr>
									{:else}
										<tr><td class="table-empty-v2" colspan="4">No paths match this filter.</td></tr>
									{/each}
								</tbody>
							</table>
						</div>
					</section>
				{/if}
			</div>

			<div class="support-grid-v2">
				{#if stats.top_hosts.length > 0}
					<section class="panel" data-od-id="top-hosts">
						<header class="panel-header-v2"><div><h2>Top hosts</h2><p>Highest-volume destinations</p></div></header>
						<ul class="compact-list-v2">{#each stats.top_hosts.slice(0, 5) as [host, count] (host)}<li><a href={resolve(`/logs?host=${encodeURIComponent(host)}`)}>{anonymize.on ? anonHost(host) : host}</a><span>{count.toLocaleString()}</span></li>{/each}</ul>
					</section>
				{/if}
				{#if stats.top_referrers.length > 0}
					<section class="panel" data-od-id="top-referrers">
						<header class="panel-header-v2"><div><h2>Top referrers</h2><p>External request sources</p></div></header>
						<ul class="compact-list-v2">{#each stats.top_referrers.slice(0, 5) as [referrer, count] (referrer)}<li><span>{referrer}</span><span>{count.toLocaleString()}</span></li>{/each}</ul>
					</section>
				{/if}
				{#if stats.top_user_agents.length > 0}
					<section class="panel" data-od-id="top-user-agents">
						<header class="panel-header-v2"><div><h2>User agents</h2><p>Client software signatures</p></div></header>
						<ul class="compact-list-v2">{#each stats.top_user_agents.slice(0, 5) as [ua, count] (ua)}<li><button type="button" onclick={() => (selectedUA = ua)} title="Parse user agent">{ua}</button><span>{count.toLocaleString()}</span></li>{/each}</ul>
					</section>
				{/if}
			</div>
		{/if}
	{/if}
</div>

{#if selectedUA}
	<UAModal ua={selectedUA} onclose={() => (selectedUA = null)} />
{/if}

<style>
	.overview-v2 { gap: 22px; }
	.overview-header-v2 { margin-bottom: 2px; }
	.overview-actions-v2 { display: flex; max-width: 100%; align-items: center; gap: 8px; }

	.auto-refresh-option-v2 {
		display: inline-flex;
		min-height: 44px;
		align-items: center;
		gap: 6px;
		color: var(--app-fg);
		font-size: 0.72rem;
		font-weight: 620;
	}
	:global(.refresh-switch-v2) {
		position: relative;
		width: 48px;
		height: 44px;
		flex: 0 0 48px;
		border: 0;
		border-radius: 999px;
		background: transparent;
		padding: 0;
	}
	:global(.refresh-switch-v2::before) {
		position: absolute;
		inset: 10px 4px;
		border: 1px solid var(--app-border-strong);
		border-radius: 999px;
		background: var(--app-surface-muted);
		content: '';
		transition: background 150ms cubic-bezier(0.2, 0, 0, 1), border-color 150ms cubic-bezier(0.2, 0, 0, 1);
	}
	:global(.refresh-switch-v2[data-state='checked']::before) {
		border-color: var(--app-fg);
		background: var(--app-fg);
	}
	:global(.refresh-switch-thumb-v2) {
		position: absolute;
		top: 13px;
		left: 7px;
		z-index: 1;
		display: block;
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background: var(--app-surface);
		box-shadow: 0 1px 3px color-mix(in oklch, var(--app-fg) 24%, transparent);
		transform: translateX(0);
		transition: transform 150ms cubic-bezier(0.2, 0, 0, 1);
	}
	:global(.refresh-switch-thumb-v2[data-state='checked']) { transform: translateX(16px); }

	.dashboard-grid-v2 { display: grid; grid-template-columns: repeat(12, minmax(0, 1fr)); gap: 16px; }
	.dashboard-span-4 { grid-column: span 4; }
	.dashboard-span-6 { grid-column: span 6; }
	.dashboard-span-8 { grid-column: span 8; }
	.dashboard-span-12 { grid-column: 1 / -1; }

	.panel-header-v2 {
		display: flex;
		min-height: 64px;
		align-items: center;
		justify-content: space-between;
		gap: 14px;
		border-bottom: 1px solid var(--app-border);
		padding: 13px 18px;
	}
	.panel-header-v2 h2 { margin: 0; font-size: 0.92rem; font-weight: 640; letter-spacing: -0.01em; }
	.panel-header-v2 p { margin: 3px 0 0; color: var(--app-muted); font-size: 0.68rem; }
	.panel-meta-v2 { color: var(--app-muted); font-family: var(--font-mono); font-size: 0.62rem; text-transform: uppercase; }
	.panel-link-v2 { color: var(--app-fg); font-size: 0.7rem; font-weight: 620; text-decoration: underline; text-decoration-color: var(--app-border); text-underline-offset: 0.2em; }
	.panel-link-v2:hover { text-decoration-color: var(--app-fg); }

	.chart-frame-v2 { position: relative; height: clamp(260px, 32vw, 360px); padding: 18px 18px 12px; }
	.chart-frame-v2 canvas { width: 100% !important; height: 100% !important; }
	.chart-empty-v2 { display: grid; height: 100%; place-content: center; justify-items: center; gap: 5px; text-align: center; }
	.chart-empty-v2 strong { font-size: 0.9rem; }
	.chart-empty-v2 span { max-width: 42ch; color: var(--app-muted); font-size: 0.76rem; }

	.status-body-v2 { display: grid; gap: 16px; padding: 18px; }
	.success-rate-v2 { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; }
	.success-rate-v2 strong { font-family: var(--font-mono); font-size: 1.75rem; font-weight: 560; letter-spacing: -0.035em; }
	.success-rate-v2 span { max-width: 14ch; color: var(--app-muted); font-size: 0.68rem; text-align: right; }
	.family-band-v2 { display: flex; height: 13px; overflow: hidden; border-radius: 4px; background: var(--app-surface-muted); }
	.family-segment-v2 { width: var(--share); min-width: 0; background: var(--tone); }
	.family-list-v2 { display: grid; margin: 0; padding: 0; list-style: none; }
	.family-list-v2 li { display: grid; grid-template-columns: 8px minmax(0, 1fr) auto; min-height: 48px; align-items: center; gap: 10px; border-bottom: 1px solid var(--app-border); }
	.family-list-v2 li:last-child { border-bottom: 0; }
	.family-swatch-v2 { width: 7px; height: 7px; border-radius: 2px; background: var(--tone); }
	.family-list-v2 strong { display: block; font-family: var(--font-mono); font-size: 0.7rem; font-weight: 650; }
	.family-list-v2 small { display: block; margin-top: 2px; color: var(--app-muted); font-size: 0.65rem; }
	.family-count-v2 { color: var(--app-muted); font-family: var(--font-mono); font-size: 0.68rem; }

	.rank-list-v2 { margin: 0; padding: 6px 18px 8px; list-style: none; }
	.rank-list-v2 li { display: grid; grid-template-columns: 24px minmax(0, 1fr) auto; min-height: 44px; align-items: center; gap: 10px; border-bottom: 1px solid var(--app-border); }
	.rank-list-v2 li:last-child { border-bottom: 0; }
	.rank-list-v2 a { min-width: 0; overflow: hidden; color: var(--app-fg); font-family: var(--font-mono); font-size: 0.72rem; text-overflow: ellipsis; white-space: nowrap; }
	.rank-list-v2 a:hover { text-decoration: underline; text-underline-offset: 0.2em; }
	.rank-list-v2 li > span:last-child { color: var(--app-muted); font-family: var(--font-mono); font-size: 0.68rem; }
	.rank-index-v2 { color: var(--app-muted); font-family: var(--font-mono); font-size: 0.62rem; }

	.table-header-v2 { min-height: 72px; }
	.filter-field-v2 { display: flex; align-items: center; gap: 8px; color: var(--app-muted); font-family: var(--font-mono); font-size: 0.62rem; text-transform: uppercase; }
	.filter-field-v2 input { width: min(220px, 36vw); min-height: 40px; border: 1px solid var(--app-border); border-radius: var(--app-radius-sm); background: var(--app-bg); color: var(--app-fg); padding: 0 12px; font-family: var(--font-sans); font-size: 0.74rem; text-transform: none; }
	.filter-field-v2 input::placeholder { color: var(--app-muted); opacity: 1; }
	.table-wrap-v2 { overflow-x: auto; }
	.table-wrap-v2 table { width: 100%; min-width: 720px; border-collapse: collapse; }
	.table-wrap-v2 th,
	.table-wrap-v2 td { border-bottom: 1px solid var(--app-border); padding: 12px 18px; text-align: left; }
	.table-wrap-v2 th { background: var(--app-surface-muted); color: var(--app-muted); font-family: var(--font-mono); font-size: 0.6rem; font-weight: 650; letter-spacing: 0.06em; text-transform: uppercase; }
	.table-wrap-v2 tbody tr:last-child td { border-bottom: 0; }
	.table-wrap-v2 tbody tr:hover { background: color-mix(in oklch, var(--app-fg) 5%, transparent); }
	.table-wrap-v2 td { font-family: var(--font-mono); font-size: 0.7rem; }
	.table-wrap-v2 th:not(:first-child),
	.table-wrap-v2 td:not(:first-child) { text-align: right; }
	.table-wrap-v2 td:first-child { max-width: 380px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.table-wrap-v2 td:first-child a:hover { text-decoration: underline; text-underline-offset: 0.2em; }
	.latency-v2 { color: var(--app-warning); font-weight: 650; }
	.table-empty-v2 { color: var(--app-muted); padding-block: 30px !important; text-align: center !important; }

	.support-grid-v2 { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 16px; }
	.compact-list-v2 { margin: 0; padding: 6px 18px 8px; list-style: none; }
	.compact-list-v2 li { display: grid; grid-template-columns: minmax(0, 1fr) auto; min-height: 42px; align-items: center; gap: 10px; border-bottom: 1px solid var(--app-border); }
	.compact-list-v2 li:last-child { border-bottom: 0; }
	.compact-list-v2 a,
	.compact-list-v2 button,
	.compact-list-v2 li > span:first-child { min-width: 0; overflow: hidden; color: var(--app-fg); font-family: var(--font-mono); font-size: 0.68rem; text-align: left; text-overflow: ellipsis; white-space: nowrap; }
	.compact-list-v2 button { min-height: 36px; border: 0; background: transparent; padding: 0; }
	.compact-list-v2 a:hover,
	.compact-list-v2 button:hover { text-decoration: underline; text-underline-offset: 0.2em; }
	.compact-list-v2 li > span:last-child { color: var(--app-muted); font-family: var(--font-mono); font-size: 0.66rem; }

	@media (max-width: 1120px) {
		.dashboard-span-4,
		.dashboard-span-8 { grid-column: 1 / -1; }
		.support-grid-v2 { grid-template-columns: 1fr 1fr; }
	}

	@media (max-width: 800px) {
		.dashboard-span-6 { grid-column: 1 / -1; }
		.support-grid-v2 { grid-template-columns: 1fr; }
		.overview-actions-v2 { width: 100%; flex-wrap: wrap; }
	}

	@media (max-width: 560px) {
		.auto-refresh-option-v2 { width: 100%; justify-content: space-between; }
		.table-header-v2 { align-items: flex-start; flex-direction: column; }
		.filter-field-v2,
		.filter-field-v2 input { width: 100%; }
		.chart-frame-v2 { height: 250px; padding-inline: 10px; }
	}
</style>
