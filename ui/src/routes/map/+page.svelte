<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { COUNTRY_COORDS } from '$lib/countries';
	import { timeRange } from '$lib/time-range.svelte';
	import TimeRangeSelector from '$lib/components/TimeRangeSelector.svelte';
	import { anonymize } from '$lib/anonymize.svelte';
	import type { Map, CircleMarker } from 'leaflet';

	interface CountryCount { country: string; count: number; top_ips: string[]; }
	interface PrecisePoint { lat: number; lng: number; count: number; top_ips: string[]; }
	type GeoMode = 'country' | 'cluster';

	interface SelectedPoint {
		label: string;
		count: number;
		ips: string[];
	}

	let L: typeof import('leaflet') | null = null;
	let mapEl = $state<HTMLDivElement | null>(null);
	let leafletMap: Map | null = null;
	let markers: CircleMarker[] = [];
	let clusterGroup: any = null;
	let data = $state<CountryCount[]>([]);
	let loading = $state(true);
	let error = $state('');
	let mode = $state<GeoMode>('country');
	let noGeoip = $state(false);
	let selectedPoint = $state<SelectedPoint | null>(null);

	function clusterIcon(count: number) {
		const size = Math.round(Math.min(56, 28 + Math.log1p(count) * 4));
		const half = size / 2;
		return L!.divIcon({
			html: `<div style="width:${size}px;height:${size}px;border-radius:50%;background:rgba(59,130,246,0.85);border:2px solid rgba(59,130,246,1);display:flex;align-items:center;justify-content:center;color:#fff;font-size:11px;font-weight:600;font-family:monospace">${count >= 1000 ? (count / 1000).toFixed(1) + 'k' : count}</div>`,
			className: '',
			iconSize: [size, size],
			iconAnchor: [half, half]
		});
	}

	async function fetchAndPlot(m: GeoMode = mode) {
		if (!leafletMap || !L) return;
		loading = true;
		error = '';
		noGeoip = false;

		try {
			const params = new URLSearchParams();
			const since = timeRange.sinceParam();
			if (since) params.set('since', since);
			if (m === 'cluster') params.set('mode', 'precise');
			const res = await fetch(`/api/geo${params.size ? `?${params}` : ''}`);
			if (!res.ok) {
				const d = await res.json().catch(() => ({}));
				throw new Error(d.error ?? `HTTP ${res.status}`);
			}
			const json = await res.json();

			markers.forEach((mk) => mk.remove());
			markers = [];
			if (clusterGroup) { leafletMap!.removeLayer(clusterGroup); clusterGroup = null; }

			if (json.mode === 'precise') {
				const raw: PrecisePoint[] = json.points;
				if (raw.length > 0) {
					clusterGroup = L.markerClusterGroup({
						maxClusterRadius: 60,
						iconCreateFunction: (cluster: any) => {
							const total: number = cluster.getAllChildMarkers()
								.reduce((s: number, mk: any) => s + (mk._weight ?? 1), 0);
							return clusterIcon(total);
						},
						spiderfyOnMaxZoom: true,
						showCoverageOnHover: false
					});

					for (const pt of raw) {
						const w = pt.count;
						const r = Math.min(18, 6 + Math.log1p(w) * 2);
						const mk: any = L.circleMarker([pt.lat, pt.lng], {
							radius: r,
							color: '#2563eb',
							fillColor: '#3b82f6',
							fillOpacity: 0.75,
							weight: 1.5
						})
							.bindTooltip(`${w.toLocaleString()} requests`)
							.on('click', () => {
								selectedPoint = {
									label: `${pt.lat.toFixed(2)}, ${pt.lng.toFixed(2)}`,
									count: w,
									ips: pt.top_ips
								};
							});
						mk._weight = w;
						clusterGroup.addLayer(mk);
					}

					leafletMap!.addLayer(clusterGroup);
				}
			} else {
				data = json.data ?? [];
				const maxCount = Math.max(...data.map((d) => d.count), 1);
				for (const { country, count, top_ips } of data) {
					const coords = COUNTRY_COORDS[country];
					if (!coords) continue;
					const marker = L!.circleMarker(coords, {
						radius: 6 + (count / maxCount) * 30,
						color: '#3b82f6', fillColor: '#3b82f6', fillOpacity: 0.5, weight: 1
					})
						.bindTooltip(`<strong>${country}</strong><br/>${count.toLocaleString()} requests`)
						.on('click', () => {
							selectedPoint = { label: country, count, ips: top_ips };
						})
						.addTo(leafletMap!);
					markers.push(marker);
				}
				if (m === 'cluster') noGeoip = true;
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to fetch geo data';
		} finally {
			loading = false;
		}
	}

	async function setMode(m: GeoMode) {
		mode = m;
		await fetchAndPlot(m);
	}

	async function initMap() {
		if (!mapEl) return;
		L = (await import('leaflet')).default;
		await import('leaflet/dist/leaflet.css');
		await import('leaflet.markercluster');
		await import('leaflet.markercluster/dist/MarkerCluster.css');
		await import('leaflet.markercluster/dist/MarkerCluster.Default.css');

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

		await fetchAndPlot(mode);
	}

	onMount(initMap);
	onDestroy(() => { leafletMap?.remove(); leafletMap = null; });
</script>

<svelte:head>
	<link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" />
</svelte:head>

<div class="mx-auto max-w-6xl space-y-6">
	<div class="flex flex-wrap items-end justify-between gap-4">
		<div>
			<h1 class="text-3xl font-bold">Map</h1>
			<p class="mt-1 text-neutral-500 dark:text-white/50">Request origins</p>
		</div>
		<div class="flex items-center gap-3">
			<div class="flex gap-1">
				{#each (['country', 'cluster'] as GeoMode[]) as m}
					<button
						onclick={() => setMode(m)}
						class="rounded-lg border px-3 py-1.5 text-sm capitalize transition-colors {mode === m
							? 'border-neutral-400 bg-neutral-200 dark:border-white/30 dark:bg-white/10'
							: 'border-neutral-200 hover:bg-neutral-100 dark:border-white/10 dark:hover:bg-white/5'}"
					>{m}</button>
				{/each}
			</div>
			<TimeRangeSelector onchange={() => fetchAndPlot(mode)} />
		</div>
	</div>

	{#if error}
		<div class="rounded-lg border border-red-200 bg-red-50 p-4 text-red-600 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-400">{error}</div>
	{/if}

	{#if noGeoip}
		<div class="rounded-lg border border-amber-200 bg-amber-50 p-4 text-sm text-amber-700 dark:border-amber-400/30 dark:bg-amber-400/10 dark:text-amber-400">
			Cluster view requires a GeoIP database. Set <code class="font-mono">GEOIP_DB</code> to the path of a MaxMind GeoLite2-City or DB-IP Lite City <code class="font-mono">.mmdb</code> file. Showing country view as fallback.
		</div>
	{/if}

	<div class="isolate overflow-hidden rounded-lg border border-neutral-200 dark:border-white/10">
		<div bind:this={mapEl} class="h-[500px] w-full"></div>
	</div>

	{#if mode === 'country' && !loading && data.length > 0}
		<div class="rounded-lg border border-neutral-200 bg-neutral-100 p-6 dark:border-white/10 dark:bg-white/5">
			<h2 class="mb-4 text-sm font-semibold uppercase tracking-wide text-neutral-500 dark:text-white/50">Requests by Country</h2>
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

<!-- Point / Country IP Modal -->
{#if selectedPoint}
	<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-sm"
		onclick={(e) => { if (e.target === e.currentTarget) selectedPoint = null; }}
	>
		<div class="flex max-h-[70vh] w-full max-w-sm flex-col overflow-hidden rounded-xl border border-neutral-200 bg-white shadow-2xl dark:border-white/10 dark:bg-neutral-900">
			<div class="flex items-center justify-between border-b border-neutral-200 px-5 py-4 dark:border-white/10">
				<div>
					<div class="font-semibold">{selectedPoint.label}</div>
					<div class="text-xs text-neutral-500 dark:text-white/40">{selectedPoint.count.toLocaleString()} requests</div>
				</div>
				<button onclick={() => selectedPoint = null} aria-label="Close" class="rounded-lg border border-neutral-200 p-1.5 text-neutral-500 transition-colors hover:bg-neutral-100 dark:border-white/10 dark:text-white/50 dark:hover:bg-white/5">
					<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M18 6L6 18M6 6l12 12"/></svg>
				</button>
			</div>
			<div class="flex-1 overflow-y-auto p-4">
				{#if selectedPoint.ips.length === 0}
					<p class="text-center text-sm text-neutral-400 dark:text-white/30">No IP data available</p>
				{:else}
					<p class="mb-2 text-xs text-neutral-400 dark:text-white/30">Top IPs (up to 10)</p>
					<ul class="space-y-1">
						{#each selectedPoint.ips as ip}
							<li>
								<a
									href="/logs?ip={encodeURIComponent(ip)}"
									class="flex items-center justify-between rounded-lg border border-neutral-100 px-3 py-2 font-mono text-sm hover:bg-neutral-50 dark:border-white/5 dark:hover:bg-white/5"
								>
									<span class="{anonymize.on ? 'blur-sm select-none' : ''}">{ip}</span>
									<svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-neutral-400 dark:text-white/30" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 12h14M12 5l7 7-7 7"/></svg>
								</a>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		</div>
	</div>
{/if}
