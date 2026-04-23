<script lang="ts">
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

	let { entry, onclose }: { entry: LogEntry; onclose: () => void } = $props();

	let copied = $state(false);

	function tlsVersion(v: number) {
		return { 769: 'TLS 1.0', 770: 'TLS 1.1', 771: 'TLS 1.2', 772: 'TLS 1.3' }[v] ?? `0x${v.toString(16)}`;
	}

	function statusColor(code: number) {
		if (code < 300) return 'text-green-600 dark:text-green-400';
		if (code < 400) return 'text-blue-600 dark:text-blue-400';
		if (code < 500) return 'text-amber-600 dark:text-amber-400';
		return 'text-red-600 dark:text-red-400';
	}

	function formatDuration(d: number) {
		const ms = d * 1000;
		return ms < 1 ? `${(ms * 1000).toFixed(0)} µs` : `${ms.toFixed(2)} ms`;
	}

	function formatBytes(b: number) {
		if (b < 1024) return `${b} B`;
		if (b < 1024 ** 2) return `${(b / 1024).toFixed(1)} KB`;
		return `${(b / 1024 ** 2).toFixed(2)} MB`;
	}

	function copyJson() {
		navigator.clipboard.writeText(JSON.stringify(entry, null, 2));
		copied = true;
		setTimeout(() => copied = false, 2000);
	}

	function onBackdrop(e: MouseEvent) {
		if (e.target === e.currentTarget) onclose();
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
	}
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div
	class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-sm"
	onclick={onBackdrop}
>
	<div class="flex max-h-[90vh] w-full max-w-3xl flex-col overflow-hidden rounded-xl border border-neutral-200 bg-white shadow-2xl dark:border-white/10 dark:bg-neutral-900">

		<!-- Header -->
		<div class="flex items-center justify-between border-b border-neutral-200 px-5 py-4 dark:border-white/10">
			<div class="flex items-center gap-3 min-w-0">
				<span class="shrink-0 font-mono font-semibold text-neutral-700 dark:text-white/80">{entry.request.method}</span>
				<span class="shrink-0 font-mono font-bold {statusColor(entry.status)}">{entry.status}</span>
				<span class="truncate font-mono text-sm text-neutral-500 dark:text-white/50">{entry.request.host}{entry.request.uri}</span>
			</div>
			<div class="ml-4 flex shrink-0 items-center gap-2">
				<button
					onclick={copyJson}
					class="rounded-lg border border-neutral-200 px-3 py-1.5 text-xs transition-colors hover:bg-neutral-100 dark:border-white/10 dark:hover:bg-white/5"
				>
					{copied ? 'Copied!' : 'Copy JSON'}
				</button>
				<button onclick={onclose} aria-label="Close" class="rounded-lg border border-neutral-200 p-1.5 text-neutral-500 transition-colors hover:bg-neutral-100 dark:border-white/10 dark:text-white/50 dark:hover:bg-white/5">
					<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M18 6L6 18M6 6l12 12"/></svg>
				</button>
			</div>
		</div>

		<!-- Body -->
		<div class="flex-1 overflow-y-auto p-5 space-y-5 text-sm">

			<!-- Request -->
			<section>
				<h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-neutral-400 dark:text-white/30">Request</h3>
				<div class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 rounded-lg border border-neutral-200 bg-neutral-50 p-3 font-mono text-xs dark:border-white/10 dark:bg-white/[0.03]">
					<span class="text-neutral-400 dark:text-white/30">Time</span>
					<span>{new Date(entry.ts * 1000).toLocaleString()} <span class="text-neutral-400 dark:text-white/30">({entry.ts})</span></span>

					<span class="text-neutral-400 dark:text-white/30">Client IP</span>
					<span>{entry.request.client_ip || '—'}</span>

					{#if entry.request.remote_ip && entry.request.remote_ip !== entry.request.client_ip}
						<span class="text-neutral-400 dark:text-white/30">Remote IP</span>
						<span>{entry.request.remote_ip}{entry.request.remote_port ? `:${entry.request.remote_port}` : ''}</span>
					{/if}

					<span class="text-neutral-400 dark:text-white/30">Protocol</span>
					<span>{entry.request.proto}</span>

					{#if entry.user_id}
						<span class="text-neutral-400 dark:text-white/30">User ID</span>
						<span>{entry.user_id}</span>
					{/if}
				</div>
			</section>

			<!-- TLS -->
			{#if entry.request.tls}
				{@const tls = entry.request.tls}
				<section>
					<h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-neutral-400 dark:text-white/30">TLS</h3>
					<div class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 rounded-lg border border-neutral-200 bg-neutral-50 p-3 font-mono text-xs dark:border-white/10 dark:bg-white/[0.03]">
						<span class="text-neutral-400 dark:text-white/30">Version</span>
						<span>{tlsVersion(tls.version)}</span>

						<span class="text-neutral-400 dark:text-white/30">Cipher</span>
						<span>0x{tls.cipher_suite.toString(16).toUpperCase()}</span>

						<span class="text-neutral-400 dark:text-white/30">ALPN</span>
						<span>{tls.proto || '—'}</span>

						<span class="text-neutral-400 dark:text-white/30">SNI</span>
						<span>{tls.server_name || '—'}</span>

						<span class="text-neutral-400 dark:text-white/30">Resumed</span>
						<span>{tls.resumed ? 'yes' : 'no'}</span>
					</div>
				</section>
			{/if}

			<!-- Request Headers -->
			{#if Object.keys(entry.request.headers).length > 0}
				<section>
					<h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-neutral-400 dark:text-white/30">Request Headers</h3>
					<div class="rounded-lg border border-neutral-200 dark:border-white/10 overflow-hidden">
						<table class="w-full font-mono text-xs">
							<tbody class="divide-y divide-neutral-100 dark:divide-white/5">
								{#each Object.entries(entry.request.headers).sort(([a], [b]) => a.localeCompare(b)) as [name, values]}
									<tr class="bg-neutral-50 dark:bg-white/[0.02]">
										<td class="w-48 shrink-0 px-3 py-1.5 text-neutral-400 dark:text-white/30 align-top">{name}</td>
										<td class="px-3 py-1.5 text-neutral-700 dark:text-white/70 break-all">{values.join(', ')}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				</section>
			{/if}

			<!-- Response -->
			<section>
				<h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-neutral-400 dark:text-white/30">Response</h3>
				<div class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 rounded-lg border border-neutral-200 bg-neutral-50 p-3 font-mono text-xs dark:border-white/10 dark:bg-white/[0.03]">
					<span class="text-neutral-400 dark:text-white/30">Status</span>
					<span class="font-semibold {statusColor(entry.status)}">{entry.status}</span>

					<span class="text-neutral-400 dark:text-white/30">Duration</span>
					<span>{formatDuration(entry.duration)}</span>

					<span class="text-neutral-400 dark:text-white/30">Response size</span>
					<span>{formatBytes(entry.size)}</span>

					{#if entry.bytes_read > 0}
						<span class="text-neutral-400 dark:text-white/30">Request body</span>
						<span>{formatBytes(entry.bytes_read)}</span>
					{/if}
				</div>
			</section>

			<!-- Response Headers -->
			{#if Object.keys(entry.resp_headers ?? {}).length > 0}
				<section>
					<h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-neutral-400 dark:text-white/30">Response Headers</h3>
					<div class="rounded-lg border border-neutral-200 dark:border-white/10 overflow-hidden">
						<table class="w-full font-mono text-xs">
							<tbody class="divide-y divide-neutral-100 dark:divide-white/5">
								{#each Object.entries(entry.resp_headers).sort(([a], [b]) => a.localeCompare(b)) as [name, values]}
									<tr class="bg-neutral-50 dark:bg-white/[0.02]">
										<td class="w-48 shrink-0 px-3 py-1.5 text-neutral-400 dark:text-white/30 align-top">{name}</td>
										<td class="px-3 py-1.5 text-neutral-700 dark:text-white/70 break-all">{values.join(', ')}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				</section>
			{/if}

		</div>
	</div>
</div>
