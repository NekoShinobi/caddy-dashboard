<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { resolve } from '$app/paths';
	import { COUNTRY_COORDS } from '$lib/countries';
	import { timeRange } from '$lib/time-range.svelte';
	import TimeRangeSelector from '$lib/components/TimeRangeSelector.svelte';
	import SegmentedControl from '$lib/components/SegmentedControl.svelte';
	import AppDialog from '$lib/components/AppDialog.svelte';
	import { anonymize } from '$lib/anonymize.svelte';
	import { colorTheme } from '$lib/color-theme.svelte';
	import { theme } from '$lib/theme.svelte';
	import type { Map, CircleMarker } from 'leaflet';

	interface CountryCount {
		country: string;
		count: number;
		top_ips: string[];
	}
	interface PrecisePoint {
		lat: number;
		lng: number;
		count: number;
		top_ips: string[];
	}
	type GeoMode = 'country' | 'cluster';

	interface SelectedPoint {
		label: string;
		count: number;
		ips: string[];
	}

	const modeOptions = [
		{ label: 'Countries', value: 'country' },
		{ label: 'Clusters', value: 'cluster' }
	];

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

	function accent() {
		return colorTheme.theme[theme.dark ? 'dark' : 'light'].blue;
	}

	function clusterIcon(count: number) {
		const size = Math.round(Math.min(56, 28 + Math.log1p(count) * 4));
		const half = size / 2;
		const clusterAccent = accent();
		return L!.divIcon({
			html: `<div class="request-cluster" style="width:${size}px;height:${size}px;background:${clusterAccent};border-color:${clusterAccent}">${count >= 1000 ? (count / 1000).toFixed(1) + 'k' : count}</div>`,
			className: '',
			iconSize: [size, size],
			iconAnchor: [half, half]
		});
	}

	async function fetchAndPlot(nextMode: GeoMode = mode) {
		if (!leafletMap || !L) return;
		loading = true;
		error = '';
		noGeoip = false;

		try {
			const params = new URLSearchParams();
			const since = timeRange.sinceParam();
			if (since) params.set('since', since);
			if (nextMode === 'cluster') params.set('mode', 'precise');
			const res = await fetch(`/api/geo${params.size ? `?${params}` : ''}`);
			if (!res.ok) {
				const response = await res.json().catch(() => ({}));
				throw new Error(response.error ?? `HTTP ${res.status}`);
			}
			const json = await res.json();

			markers.forEach((marker) => marker.remove());
			markers = [];
			if (clusterGroup) {
				leafletMap.removeLayer(clusterGroup);
				clusterGroup = null;
			}

			if (json.mode === 'precise') {
				const raw: PrecisePoint[] = json.points;
				if (raw.length > 0) {
					clusterGroup = L.markerClusterGroup({
						maxClusterRadius: 60,
						iconCreateFunction: (cluster: any) => {
							const total: number = cluster
								.getAllChildMarkers()
								.reduce((sum: number, marker: any) => sum + (marker._weight ?? 1), 0);
							return clusterIcon(total);
						},
						spiderfyOnMaxZoom: true,
						showCoverageOnHover: false
					});

					for (const point of raw) {
						const radius = Math.min(18, 6 + Math.log1p(point.count) * 2);
						const marker: any = L.circleMarker([point.lat, point.lng], {
							radius,
							color: accent(),
							fillColor: accent(),
							fillOpacity: 0.7,
							weight: 1.5
						})
							.bindTooltip(`${point.count.toLocaleString()} requests`)
							.on('click', () => {
								selectedPoint = {
									label: `${point.lat.toFixed(2)}, ${point.lng.toFixed(2)}`,
									count: point.count,
									ips: point.top_ips
								};
							});
						marker._weight = point.count;
						clusterGroup.addLayer(marker);
					}

					leafletMap.addLayer(clusterGroup);
				}
			} else {
				data = json.data ?? [];
				const maxCount = Math.max(...data.map((entry) => entry.count), 1);
				for (const { country, count, top_ips } of data) {
					const coords = COUNTRY_COORDS[country];
					if (!coords) continue;
					const marker = L.circleMarker(coords, {
						radius: 6 + (count / maxCount) * 30,
						color: accent(),
						fillColor: accent(),
						fillOpacity: 0.48,
						weight: 1.5
					})
						.bindTooltip(`<strong>${country}</strong><br/>${count.toLocaleString()} requests`)
						.on('click', () => {
							selectedPoint = { label: country, count, ips: top_ips };
						})
						.addTo(leafletMap);
					markers.push(marker);
				}
				if (nextMode === 'cluster') noGeoip = true;
			}
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Failed to fetch geo data';
		} finally {
			loading = false;
		}
	}

	function changeMode(value: string) {
		mode = value as GeoMode;
		void fetchAndPlot(mode);
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
			maxBounds: [
				[-90, -180],
				[90, 180]
			],
			maxBoundsViscosity: 1
		}).setView([20, 0], 2);

		L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
			attribution: '© <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
		}).addTo(leafletMap);

		await fetchAndPlot(mode);
	}

	onMount(initMap);
	onDestroy(() => {
		leafletMap?.remove();
		leafletMap = null;
	});
</script>

<div class="page-shell" data-od-id="map-page">
	<header class="page-header" data-od-id="map-header">
		<div>
			<p class="page-eyebrow">Geography</p>
			<h1 class="page-title">Request map</h1>
			<p class="page-description">
				Locate traffic concentration and investigate the clients behind each region.
			</p>
		</div>
		<div class="flex flex-wrap items-center gap-3">
			<SegmentedControl
				value={mode}
				options={modeOptions}
				onchange={changeMode}
				label="Map resolution"
			/>
			<TimeRangeSelector onchange={() => fetchAndPlot(mode)} />
		</div>
	</header>

	{#if error}
		<div class="status-alert status-alert-error" role="alert">{error}</div>
	{/if}

	{#if noGeoip}
		<div class="status-alert status-alert-warning" role="status">
			<div>
				<strong>Precise clustering is unavailable.</strong>
				<p>
					Set <code>GEOIP_DB</code> to a MaxMind GeoLite2-City or DB-IP Lite City <code>.mmdb</code> file.
					Country-level data is shown instead.
				</p>
			</div>
		</div>
	{/if}

	<section class="panel map-shell" data-od-id="request-map">
		<div
			bind:this={mapEl}
			class="h-[clamp(420px,62vh,680px)] w-full"
			aria-label="Interactive request origin map"
		></div>
		{#if loading}
			<div class="map-loading" role="status">
				<span class="skeleton h-2 w-24"></span>
				<span>Updating geography…</span>
			</div>
		{/if}
	</section>

	{#if mode === 'country' && !loading}
		<section class="panel panel-pad" data-od-id="country-ranking">
			<div class="section-heading">
				<div>
					<p class="page-eyebrow">Country ranking</p>
					<h2>Requests by origin</h2>
				</div>
				<span class="count-badge">{data.length} countries</span>
			</div>
			{#if data.length === 0}
				<div class="empty-state">
					<strong>No geographic data</strong>
					<span>Requests with a recognized country will appear here.</span>
				</div>
			{:else}
				<div class="country-grid">
					{#each data as entry (entry.country)}
						<button
							class="country-item"
							onclick={() =>
								(selectedPoint = { label: entry.country, count: entry.count, ips: entry.top_ips })}
						>
							<span class="font-mono font-semibold">{entry.country}</span>
							<span class="font-mono text-sm text-[var(--app-muted-fg)]"
								>{entry.count.toLocaleString()}</span
							>
						</button>
					{/each}
				</div>
			{/if}
		</section>
	{/if}
</div>

{#if selectedPoint}
	<AppDialog
		open={true}
		onOpenChange={(open) => {
			if (!open) selectedPoint = null;
		}}
		title={selectedPoint.label}
		description={`${selectedPoint.count.toLocaleString()} requests in the selected period`}
		size="sm"
	>
		{#if selectedPoint.ips.length === 0}
			<div class="empty-state">
				<strong>No client details</strong>
				<span>IP data is not available for this location.</span>
			</div>
		{:else}
			<div>
				<p
					class="mb-3 text-xs font-semibold tracking-[0.16em] text-[var(--app-muted-fg)] uppercase"
				>
					Top clients
				</p>
				<ul class="space-y-2">
					{#each selectedPoint.ips as ip (ip)}
						<li>
							<a href={resolve(`/logs?ip=${encodeURIComponent(ip)}`)} class="client-link">
								<span class="font-mono {anonymize.on ? 'blur-sm select-none' : ''}">{ip}</span>
								<span aria-hidden="true">→</span>
							</a>
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	</AppDialog>
{/if}

<style>
	.map-shell {
		position: relative;
		isolation: isolate;
		overflow: hidden;
		min-height: 420px;
	}

	.map-loading {
		position: absolute;
		inset: 1rem 1rem auto auto;
		z-index: 500;
		display: flex;
		align-items: center;
		gap: 0.65rem;
		padding: 0.65rem 0.8rem;
		border: 1px solid var(--app-border);
		border-radius: var(--radius-sm);
		background: color-mix(in oklch, var(--app-surface) 90%, transparent);
		box-shadow: var(--shadow-soft);
		font-size: 0.75rem;
		color: var(--app-muted-fg);
		backdrop-filter: blur(12px);
	}

	.country-grid {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: 0.5rem;
	}

	.country-item,
	.client-link {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		width: 100%;
		min-height: 44px;
		padding: 0.65rem 0.8rem;
		border: 1px solid var(--app-border);
		border-radius: var(--radius-sm);
		background: var(--app-surface);
		transition:
			border-color 140ms ease,
			background 140ms ease,
			transform 140ms ease;
	}

	.country-item:hover,
	.client-link:hover {
		border-color: var(--app-border-strong);
		background: var(--app-surface-muted);
	}

	.country-item:active,
	.client-link:active {
		transform: translateY(1px);
	}

	:global(.request-cluster) {
		display: flex;
		align-items: center;
		justify-content: center;
		border: 2px solid;
		border-radius: 999px;
		color: white;
		font:
			600 0.7rem/1 ui-monospace,
			SFMono-Regular,
			Menlo,
			monospace;
		box-shadow: 0 8px 18px oklch(0.12 0.02 230 / 0.22);
	}

	:global(.leaflet-container) {
		background: var(--app-surface-muted);
		font-family: inherit;
	}

	:global(.leaflet-control-zoom a),
	:global(.leaflet-control-attribution) {
		color: var(--app-fg);
		background: color-mix(in oklch, var(--app-surface) 94%, transparent);
	}

	:global(.dark .leaflet-tile-pane) {
		filter: grayscale(0.35) invert(0.9) hue-rotate(165deg) brightness(0.72) contrast(0.92);
	}

	@media (max-width: 820px) {
		.country-grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}

	@media (max-width: 520px) {
		.country-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
