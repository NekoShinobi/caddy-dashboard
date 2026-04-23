<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { COUNTRY_COORDS } from '$lib/countries';
	import { timeRange } from '$lib/time-range.svelte';
	import TimeRangeSelector from '$lib/components/TimeRangeSelector.svelte';
	import type { Map, CircleMarker } from 'leaflet';

	interface CountryCount {
		country: string;
		count: number;
	}

	let L: typeof import('leaflet') | null = null;
	let mapEl = $state<HTMLDivElement | null>(null);
	let leafletMap: Map | null = null;
	let markers: CircleMarker[] = [];
	let data = $state<CountryCount[]>([]);
	let loading = $state(true);
	let error = $state('');

	async function fetchAndPlot() {
		if (!leafletMap || !L) return;
		loading = true;
		error = '';
		try {
			const since = timeRange.sinceParam();
			const url = since ? `/api/geo?since=${since}` : '/api/geo';
			const res = await fetch(url);
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			data = await res.json();

			markers.forEach((m) => m.remove());
			markers = [];

			const max = Math.max(...data.map((d) => d.count), 1);
			for (const { country, count } of data) {
				const coords = COUNTRY_COORDS[country];
				if (!coords) continue;
				const radius = 6 + (count / max) * 30;
				const marker = L!.circleMarker(coords, {
					radius,
					color: '#3b82f6',
					fillColor: '#3b82f6',
					fillOpacity: 0.5,
					weight: 1
				})
					.bindTooltip(`<strong>${country}</strong><br/>${count.toLocaleString()} requests`)
					.addTo(leafletMap!);
				markers.push(marker);
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to fetch geo data';
		} finally {
			loading = false;
		}
	}

	async function initMap() {
		if (!mapEl) return;
		L = (await import('leaflet')).default;
		await import('leaflet/dist/leaflet.css');

		leafletMap = L.map(mapEl, {
			zoomControl: true,
			scrollWheelZoom: true,
			minZoom: 2,
			maxBounds: [[-90, -180], [90, 180]],
			maxBoundsViscosity: 1.0
		}).setView([20, 0], 2);

		L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
			attribution: '© <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
		}).addTo(leafletMap);

		await fetchAndPlot();
	}

	onMount(initMap);

	onDestroy(() => {
		leafletMap?.remove();
		leafletMap = null;
	});
</script>

<svelte:head>
	<link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" />
</svelte:head>

<div class="mx-auto max-w-6xl space-y-6">
	<div class="flex flex-wrap items-end justify-between gap-4">
		<div>
			<h1 class="text-3xl font-bold">Map</h1>
			<p class="mt-1 text-neutral-500 dark:text-white/50">Request origins by country</p>
		</div>
		<TimeRangeSelector onchange={fetchAndPlot} />
	</div>

	{#if error}
		<div class="rounded-lg border border-red-200 bg-red-50 p-4 text-red-600 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-400">{error}</div>
	{/if}

	<div class="isolate overflow-hidden rounded-lg border border-neutral-200 dark:border-white/10">
		<div bind:this={mapEl} class="h-[500px] w-full"></div>
	</div>

	{#if !loading && data.length > 0}
		<div class="rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5">
			<h2 class="mb-4 text-sm font-semibold uppercase tracking-wide text-neutral-500 dark:text-white/50">
				Requests by Country
			</h2>
			<div class="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4">
				{#each data as { country, count }}
					<div class="flex items-center justify-between rounded border border-neutral-200 px-3 py-2 dark:border-white/5">
						<span class="font-mono text-sm font-semibold">{country}</span>
						<span class="text-sm text-neutral-500 dark:text-white/50">{count.toLocaleString()}</span>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
