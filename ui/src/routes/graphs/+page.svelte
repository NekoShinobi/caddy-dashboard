<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { theme } from '$lib/theme.svelte';
	import { colorTheme } from '$lib/color-theme.svelte';
	import SegmentedControl from '$lib/components/SegmentedControl.svelte';
	import {
		Chart,
		LineController,
		LineElement,
		PointElement,
		LinearScale,
		Tooltip,
		Legend,
		Filler,
		CategoryScale,
		BarController,
		BarElement
	} from 'chart.js';

	Chart.register(
		LineController,
		LineElement,
		PointElement,
		LinearScale,
		Tooltip,
		Legend,
		Filler,
		CategoryScale,
		BarController,
		BarElement
	);

	interface Bucket {
		ts: number;
		total: number;
		s2xx: number;
		s3xx: number;
		s4xx: number;
		s5xx: number;
		avg_duration_ms: number;
		median_duration_ms: number;
		p99_duration_ms: number;
		avg_size: number;
		median_size: number;
		p99_size: number;
		unique_clients: number;
		methods: Record<string, number>;
	}

	let canvasRequests = $state<HTMLCanvasElement | null>(null);
	let canvasDuration = $state<HTMLCanvasElement | null>(null);
	let canvasSize = $state<HTMLCanvasElement | null>(null);
	let canvasHosts = $state<HTMLCanvasElement | null>(null);
	let canvasMethods = $state<HTMLCanvasElement | null>(null);

	let charts: (Chart<any> | null)[] = [null, null, null, null, null];
	let loading = $state(true);
	let error = $state('');
	let requestController: AbortController | null = null;
	let bucket = $state('hour');
	let buckets = $state<Bucket[]>([]);
	const bucketOptions = [
		{ label: 'Minute', value: 'minute' },
		{ label: 'Hour', value: 'hour' },
		{ label: 'Day', value: 'day' }
	];

	function colors() {
		const active = colorTheme.theme[theme.dark ? 'dark' : 'light'];
		const tokenSource = canvasRequests?.closest('.app-shell') ?? document.documentElement;
		const styles = getComputedStyle(tokenSource);
		const token = (name: string, fallback: string) =>
			styles.getPropertyValue(name).trim() || fallback;
		return {
			grid: token('--app-border', 'oklch(0.84 0.01 230)'),
			tick: token('--app-muted-fg', 'oklch(0.48 0.02 230)'),
			total: token('--app-fg', 'oklch(0.2 0.02 230)'),
			totalFill: token('--app-muted', 'oklch(0.94 0.01 230)'),
			green: active.green,
			yellow: active.yellow,
			red: active.red,
			blue: active.blue,
			purple: active.purple,
			orange: active.orange
		};
	}

	function scaleOpts() {
		const c = colors();
		return {
			x: { ticks: { color: c.tick, maxTicksLimit: 10, maxRotation: 0 }, grid: { color: c.grid } },
			y: { beginAtZero: true, ticks: { color: c.tick }, grid: { color: c.grid } }
		};
	}

	function legendOpts() {
		return { labels: { color: colors().tick } };
	}

	function fmtMs(v: unknown): string {
		const n = Number(v);
		if (n >= 60_000) return `${(n / 60_000).toFixed(1)}m`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(2)}s`;
		if (n >= 1) return `${n.toFixed(1)}ms`;
		return `${(n * 1_000).toFixed(0)}µs`;
	}

	function fmtBytes(v: unknown): string {
		const n = Number(v);
		if (n >= 1_048_576) return `${(n / 1_048_576).toFixed(1)}MB`;
		if (n >= 1_024) return `${(n / 1_024).toFixed(1)}KB`;
		return `${n}B`;
	}

	function formatLabel(ts: number): string {
		const d = new Date(ts * 1000);
		if (bucket === 'minute')
			return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
		if (bucket === 'day') return d.toLocaleDateString([], { month: 'short', day: 'numeric' });
		return d.toLocaleString([], {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function buildOrUpdate(
		idx: number,
		canvas: HTMLCanvasElement,
		config: ConstructorParameters<typeof Chart>[1]
	) {
		if (charts[idx]) {
			const c = charts[idx]!;
			c.data = config.data as typeof c.data;
			if (config.options) c.options = config.options as typeof c.options;
			c.update('none'); // no animation = no lag
		} else {
			charts[idx] = new Chart(canvas, config);
		}
	}

	function methodColor(method: string, c: ReturnType<typeof colors>): string {
		const map: Record<string, string> = {
			GET: c.green,
			POST: c.blue,
			PUT: c.yellow,
			DELETE: c.red,
			PATCH: c.orange,
			HEAD: c.purple
		};
		return map[method.toUpperCase()] ?? c.total;
	}

	function renderAll() {
		if (!canvasRequests || !canvasDuration || !canvasSize || !canvasHosts || !canvasMethods) return;
		const c = colors();
		const labels = buckets.map((b) => formatLabel(b.ts));
		const scales = scaleOpts();
		const plugins = { legend: legendOpts() };
		const shared = {
			responsive: true,
			maintainAspectRatio: false,
			interaction: { mode: 'index' as const, intersect: false },
			plugins,
			scales
		};

		buildOrUpdate(0, canvasRequests, {
			type: 'line',
			data: {
				labels,
				datasets: [
					{
						label: 'Total',
						data: buckets.map((b) => b.total),
						borderColor: c.total,
						backgroundColor: c.totalFill,
						fill: true,
						tension: 0.3,
						pointRadius: 2
					},
					{
						label: '2xx',
						data: buckets.map((b) => b.s2xx),
						borderColor: c.green,
						backgroundColor: 'transparent',
						tension: 0.3,
						pointRadius: 0
					},
					{
						label: '4xx',
						data: buckets.map((b) => b.s4xx),
						borderColor: c.yellow,
						backgroundColor: 'transparent',
						tension: 0.3,
						pointRadius: 0
					},
					{
						label: '5xx',
						data: buckets.map((b) => b.s5xx),
						borderColor: c.red,
						backgroundColor: 'transparent',
						tension: 0.3,
						pointRadius: 0
					}
				]
			},
			options: shared
		});

		buildOrUpdate(1, canvasDuration, {
			type: 'line',
			data: {
				labels,
				datasets: [
					{
						label: 'Avg',
						data: buckets.map((b) => +b.avg_duration_ms.toFixed(2)),
						borderColor: c.blue,
						backgroundColor: 'transparent',
						tension: 0.3,
						pointRadius: 0
					},
					{
						label: 'Median',
						data: buckets.map((b) => +b.median_duration_ms.toFixed(2)),
						borderColor: c.green,
						backgroundColor: 'transparent',
						tension: 0.3,
						pointRadius: 0
					},
					{
						label: 'p99',
						data: buckets.map((b) => +b.p99_duration_ms.toFixed(2)),
						borderColor: c.red,
						backgroundColor: 'transparent',
						tension: 0.3,
						pointRadius: 0
					}
				]
			},
			options: {
				...shared,
				scales: { ...scales, y: { ...scales.y, ticks: { ...scales.y.ticks, callback: fmtMs } } }
			}
		});

		buildOrUpdate(2, canvasSize, {
			type: 'line',
			data: {
				labels,
				datasets: [
					{
						label: 'Avg',
						data: buckets.map((b) => +b.avg_size.toFixed(0)),
						borderColor: c.purple,
						backgroundColor: 'transparent',
						tension: 0.3,
						pointRadius: 0
					},
					{
						label: 'Median',
						data: buckets.map((b) => +b.median_size.toFixed(0)),
						borderColor: c.green,
						backgroundColor: 'transparent',
						tension: 0.3,
						pointRadius: 0
					},
					{
						label: 'p99',
						data: buckets.map((b) => +b.p99_size.toFixed(0)),
						borderColor: c.orange,
						backgroundColor: 'transparent',
						tension: 0.3,
						pointRadius: 0
					}
				]
			},
			options: {
				...shared,
				scales: { ...scales, y: { ...scales.y, ticks: { ...scales.y.ticks, callback: fmtBytes } } }
			}
		});

		buildOrUpdate(3, canvasHosts, {
			type: 'bar',
			data: {
				labels,
				datasets: [
					{
						label: 'Unique Clients',
						data: buckets.map((b) => b.unique_clients),
						backgroundColor: c.blue + '80',
						borderColor: c.blue,
						borderWidth: 1
					}
				]
			},
			options: shared
		});

		const allMethods = [...new Set(buckets.flatMap((b) => Object.keys(b.methods ?? {})))].sort();
		buildOrUpdate(4, canvasMethods!, {
			type: 'bar',
			data: {
				labels,
				datasets: allMethods.map((m) => ({
					label: m,
					data: buckets.map((b) => b.methods?.[m] ?? 0),
					backgroundColor: methodColor(m, c) + '99',
					borderColor: methodColor(m, c),
					borderWidth: 1
				}))
			},
			options: {
				...shared,
				scales: { ...scales, x: { ...scales.x, stacked: true }, y: { ...scales.y, stacked: true } }
			}
		});
	}

	function changeBucket(value: string) {
		bucket = value;
		void fetchAndRender();
	}

	async function fetchAndRender() {
		requestController?.abort();
		const controller = new AbortController();
		requestController = controller;
		loading = true;
		error = '';
		try {
			const res = await fetch(`/api/timeline?bucket=${bucket}`, {
				signal: controller.signal
			});
			if (!res.ok) {
				const d = await res.json().catch(() => ({}));
				throw new Error(d.error ?? `HTTP ${res.status}`);
			}
			const data = await res.json();
			buckets = data.buckets;
			renderAll();
		} catch (e) {
			if (e instanceof DOMException && e.name === 'AbortError') return;
			error = e instanceof Error ? e.message : 'Failed to fetch timeline';
		} finally {
			if (requestController === controller) loading = false;
		}
	}

	// Re-render on theme or color scheme change (no refetch)
	$effect(() => {
		void theme.dark;
		void colorTheme.id;
		if (buckets.length) renderAll();
	});

	onMount(fetchAndRender);
	onDestroy(() => {
		requestController?.abort();
		charts.forEach((c) => c?.destroy());
	});
</script>

<div class="page-shell" data-od-id="graphs-page">
	<header class="page-header" data-od-id="graphs-header">
		<div>
			<p class="page-eyebrow">Traffic intelligence</p>
			<h1 class="page-title">Performance trends</h1>
			<p class="page-description">
				Compare volume, latency, payload size, and client behavior across the selected resolution.
			</p>
		</div>
		<SegmentedControl
			value={bucket}
			options={bucketOptions}
			onchange={changeBucket}
			label="Chart resolution"
		/>
	</header>

	{#if error}
		<div class="status-alert status-alert-error" role="alert">{error}</div>
	{/if}

	{#if loading}
		<div class="chart-grid" aria-label="Loading charts">
			<div class="panel panel-pad chart-card chart-card-primary">
				<span class="skeleton h-3 w-28"></span>
				<span class="skeleton mt-6 block h-64 w-full"></span>
			</div>
			<div class="panel panel-pad chart-card">
				<span class="skeleton h-3 w-24"></span>
				<span class="skeleton mt-6 block h-52 w-full"></span>
			</div>
			<div class="panel panel-pad chart-card">
				<span class="skeleton h-3 w-32"></span>
				<span class="skeleton mt-6 block h-52 w-full"></span>
			</div>
		</div>
	{:else if buckets.length === 0}
		<div class="empty-state panel">
			<strong>No timeline data</strong>
			<span>Traffic activity will appear when requests are recorded.</span>
		</div>
	{/if}

	<div class="chart-grid" class:hidden={loading || buckets.length === 0}>
		<section class="panel panel-pad chart-card chart-card-primary" data-od-id="requests-chart">
			<div class="section-heading">
				<div>
					<p class="page-eyebrow">Volume &amp; reliability</p>
					<h2>Requests by status family</h2>
				</div>
			</div>
			<div class="chart-canvas chart-canvas-primary">
				<canvas
					bind:this={canvasRequests}
					aria-label="Line chart of total, successful, client-error, and server-error requests over time"
				></canvas>
			</div>
		</section>

		<section class="panel panel-pad chart-card" data-od-id="duration-chart">
			<div class="section-heading">
				<div>
					<p class="page-eyebrow">Latency</p>
					<h2>Response duration</h2>
				</div>
			</div>
			<div class="chart-canvas">
				<canvas
					bind:this={canvasDuration}
					aria-label="Line chart of average, median, and p99 response duration"
				></canvas>
			</div>
		</section>

		<section class="panel panel-pad chart-card" data-od-id="size-chart">
			<div class="section-heading">
				<div>
					<p class="page-eyebrow">Payload</p>
					<h2>Response size</h2>
				</div>
			</div>
			<div class="chart-canvas">
				<canvas
					bind:this={canvasSize}
					aria-label="Line chart of average, median, and p99 response size"
				></canvas>
			</div>
		</section>

		<section class="panel panel-pad chart-card" data-od-id="clients-chart">
			<div class="section-heading">
				<div>
					<p class="page-eyebrow">Audience</p>
					<h2>Unique clients</h2>
				</div>
			</div>
			<div class="chart-canvas">
				<canvas bind:this={canvasHosts} aria-label="Bar chart of unique clients over time"></canvas>
			</div>
		</section>

		<section class="panel panel-pad chart-card" data-od-id="methods-chart">
			<div class="section-heading">
				<div>
					<p class="page-eyebrow">Request mix</p>
					<h2>HTTP methods</h2>
				</div>
			</div>
			<div class="chart-canvas">
				<canvas
					bind:this={canvasMethods}
					aria-label="Stacked bar chart of HTTP request methods over time"
				></canvas>
			</div>
		</section>
	</div>
</div>

<style>
	.chart-grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 1rem;
	}

	.chart-card {
		min-width: 0;
	}

	.chart-card-primary {
		grid-column: 1 / -1;
	}

	.chart-canvas {
		height: 17rem;
		margin-top: 1.2rem;
	}

	.chart-canvas-primary {
		height: 22rem;
	}

	@media (max-width: 820px) {
		.chart-grid {
			grid-template-columns: 1fr;
		}

		.chart-card-primary {
			grid-column: auto;
		}

		.chart-canvas,
		.chart-canvas-primary {
			height: 16rem;
		}
	}
</style>
