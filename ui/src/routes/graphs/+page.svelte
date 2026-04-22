<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { theme } from '$lib/theme.svelte';
	import { colorTheme } from '$lib/color-theme.svelte';
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
		LineController, LineElement, PointElement, LinearScale,
		Tooltip, Legend, Filler, CategoryScale, BarController, BarElement
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
	}

	let canvasRequests = $state<HTMLCanvasElement | null>(null);
	let canvasDuration = $state<HTMLCanvasElement | null>(null);
	let canvasSize = $state<HTMLCanvasElement | null>(null);
	let canvasHosts = $state<HTMLCanvasElement | null>(null);

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let charts: (Chart<any> | null)[] = [null, null, null, null];
	let loading = $state(true);
	let error = $state('');
	let bucket = $state('hour');
	let buckets: Bucket[] = [];

	function colors() {
		const d = theme.dark;
		const ct = colorTheme.theme[d ? 'dark' : 'light'];
		return {
			grid: d ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.06)',
			tick: d ? 'rgba(255,255,255,0.4)' : 'rgba(0,0,0,0.4)',
			total: d ? 'rgba(255,255,255,0.7)' : 'rgba(0,0,0,0.6)',
			totalFill: d ? 'rgba(255,255,255,0.05)' : 'rgba(0,0,0,0.04)',
			green: ct.green,
			yellow: ct.yellow,
			red: ct.red,
			blue: ct.blue,
			purple: ct.purple,
			orange: ct.orange,
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
		if (n >= 1_000)  return `${(n / 1_000).toFixed(2)}s`;
		if (n >= 1)      return `${n.toFixed(1)}ms`;
		return `${(n * 1_000).toFixed(0)}µs`;
	}

	function fmtBytes(v: unknown): string {
		const n = Number(v);
		if (n >= 1_048_576) return `${(n / 1_048_576).toFixed(1)}MB`;
		if (n >= 1_024)     return `${(n / 1_024).toFixed(1)}KB`;
		return `${n}B`;
	}

	function formatLabel(ts: number): string {
		const d = new Date(ts * 1000);
		if (bucket === 'minute') return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
		if (bucket === 'day') return d.toLocaleDateString([], { month: 'short', day: 'numeric' });
		return d.toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
	}

	function buildOrUpdate(idx: number, canvas: HTMLCanvasElement, config: ConstructorParameters<typeof Chart>[1]) {
		if (charts[idx]) {
			const c = charts[idx]!;
			c.data = config.data as typeof c.data;
			if (config.options) c.options = config.options as typeof c.options;
			c.update('none'); // no animation = no lag
		} else {
			charts[idx] = new Chart(canvas, config);
		}
	}

	function renderAll() {
		if (!canvasRequests || !canvasDuration || !canvasSize || !canvasHosts) return;
		const c = colors();
		const labels = buckets.map((b) => formatLabel(b.ts));
		const scales = scaleOpts();
		const plugins = { legend: legendOpts() };
		const shared = { responsive: true, maintainAspectRatio: false,
			interaction: { mode: 'index' as const, intersect: false }, plugins, scales };

		buildOrUpdate(0, canvasRequests, {
			type: 'line',
			data: {
				labels,
				datasets: [
					{ label: 'Total', data: buckets.map((b) => b.total), borderColor: c.total,
					  backgroundColor: c.totalFill, fill: true, tension: 0.3, pointRadius: 2 },
					{ label: '2xx', data: buckets.map((b) => b.s2xx), borderColor: c.green,
					  backgroundColor: 'transparent', tension: 0.3, pointRadius: 0 },
					{ label: '4xx', data: buckets.map((b) => b.s4xx), borderColor: c.yellow,
					  backgroundColor: 'transparent', tension: 0.3, pointRadius: 0 },
					{ label: '5xx', data: buckets.map((b) => b.s5xx), borderColor: c.red,
					  backgroundColor: 'transparent', tension: 0.3, pointRadius: 0 }
				]
			},
			options: shared
		});

		buildOrUpdate(1, canvasDuration, {
			type: 'line',
			data: {
				labels,
				datasets: [
					{ label: 'Avg', data: buckets.map((b) => +b.avg_duration_ms.toFixed(2)),
					  borderColor: c.blue, backgroundColor: 'transparent', tension: 0.3, pointRadius: 0 },
					{ label: 'Median', data: buckets.map((b) => +b.median_duration_ms.toFixed(2)),
					  borderColor: c.green, backgroundColor: 'transparent', tension: 0.3, pointRadius: 0 },
					{ label: 'p99', data: buckets.map((b) => +b.p99_duration_ms.toFixed(2)),
					  borderColor: c.red, backgroundColor: 'transparent', tension: 0.3, pointRadius: 0 }
				]
			},
			options: { ...shared, scales: { ...scales,
				y: { ...scales.y, ticks: { ...scales.y.ticks, callback: fmtMs } } } }
		});

		buildOrUpdate(2, canvasSize, {
			type: 'line',
			data: {
				labels,
				datasets: [
					{ label: 'Avg', data: buckets.map((b) => +b.avg_size.toFixed(0)),
					  borderColor: c.purple, backgroundColor: 'transparent', tension: 0.3, pointRadius: 0 },
					{ label: 'Median', data: buckets.map((b) => +b.median_size.toFixed(0)),
					  borderColor: c.green, backgroundColor: 'transparent', tension: 0.3, pointRadius: 0 },
					{ label: 'p99', data: buckets.map((b) => +b.p99_size.toFixed(0)),
					  borderColor: c.orange, backgroundColor: 'transparent', tension: 0.3, pointRadius: 0 }
				]
			},
			options: { ...shared, scales: { ...scales,
				y: { ...scales.y, ticks: { ...scales.y.ticks, callback: fmtBytes } } } }
		});

		buildOrUpdate(3, canvasHosts, {
			type: 'bar',
			data: {
				labels,
				datasets: [
					{ label: 'Unique Clients', data: buckets.map((b) => b.unique_clients),
					  backgroundColor: c.blue + '80', borderColor: c.blue, borderWidth: 1 }
				]
			},
			options: shared
		});
	}

	async function fetchAndRender() {
		loading = true;
		error = '';
		try {
			const res = await fetch(`/api/timeline?bucket=${bucket}`);
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			const data = await res.json();
			buckets = data.buckets;
			renderAll();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to fetch timeline';
		} finally {
			loading = false;
		}
	}

	// Re-render on theme or color scheme change (no refetch)
	$effect(() => {
		void theme.dark;
		void colorTheme.id;
		if (buckets.length) renderAll();
	});

	onMount(fetchAndRender);
	onDestroy(() => charts.forEach((c) => c?.destroy()));
</script>

<div class="mx-auto max-w-6xl space-y-6">
	<div>
		<h1 class="text-3xl font-bold">Graphs</h1>
		<p class="mt-1 text-neutral-500 dark:text-white/50">Requests, latency, and size over time</p>
	</div>

	<div class="flex gap-2">
		{#each ['minute', 'hour', 'day'] as b}
			<button
				onclick={() => { bucket = b; fetchAndRender(); }}
				class="rounded-lg border px-4 py-1.5 text-sm transition-colors {bucket === b
					? 'border-neutral-400 bg-neutral-200 dark:border-white/30 dark:bg-white/10'
					: 'border-neutral-200 hover:bg-neutral-100 dark:border-white/10 dark:hover:bg-white/5'}"
			>
				{b}
			</button>
		{/each}
	</div>

	{#if error}
		<div class="rounded-lg border border-red-200 bg-red-50 p-4 text-red-600 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-400">{error}</div>
	{/if}

	{#if loading}
		<div class="text-neutral-500 dark:text-white/50">Loading...</div>
	{/if}

	{#each [
		{ label: 'Requests', ref: 'canvasRequests' },
		{ label: 'Duration (ms)', ref: 'canvasDuration' },
		{ label: 'Response Size (bytes)', ref: 'canvasSize' },
		{ label: 'Unique Clients', ref: 'canvasHosts' }
	] as chart, i}
		<div class="rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5" class:hidden={loading}>
			<h2 class="mb-4 text-sm font-semibold uppercase tracking-wide text-neutral-500 dark:text-white/50">{chart.label}</h2>
			<div class="h-64">
				{#if i === 0}<canvas bind:this={canvasRequests}></canvas>
				{:else if i === 1}<canvas bind:this={canvasDuration}></canvas>
				{:else if i === 2}<canvas bind:this={canvasSize}></canvas>
				{:else}<canvas bind:this={canvasHosts}></canvas>
				{/if}
			</div>
		</div>
	{/each}
</div>
