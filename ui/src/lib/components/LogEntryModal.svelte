<script lang="ts">
	import AppDialog from './AppDialog.svelte';

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

	function tlsVersion(version: number) {
		return (
			{ 769: 'TLS 1.0', 770: 'TLS 1.1', 771: 'TLS 1.2', 772: 'TLS 1.3' }[version] ??
			`0x${version.toString(16)}`
		);
	}

	function statusColor(code: number) {
		if (code < 300) return 'text-green-600 dark:text-green-400';
		if (code < 400) return 'text-blue-600 dark:text-blue-400';
		if (code < 500) return 'text-amber-600 dark:text-amber-400';
		return 'text-red-600 dark:text-red-400';
	}

	function formatDuration(duration: number) {
		const milliseconds = duration * 1000;
		return milliseconds < 1
			? `${(milliseconds * 1000).toFixed(0)} µs`
			: `${milliseconds.toFixed(2)} ms`;
	}

	function formatBytes(bytes: number) {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / 1024 ** 2).toFixed(2)} MB`;
	}

	async function copyJson() {
		await navigator.clipboard.writeText(JSON.stringify(entry, null, 2));
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}
</script>

<AppDialog
	open={true}
	onOpenChange={(open) => {
		if (!open) onclose();
	}}
	title={`${entry.request.method} ${entry.status} · Request detail`}
	description={`${entry.request.host}${entry.request.uri}`}
	size="lg"
>
	<div class="space-y-5 text-sm">
		<section>
			<h3 class="mb-2 text-xs font-semibold tracking-wide text-neutral-400 dark:text-white/40">
				Request
			</h3>
			<dl
				class="grid grid-cols-[auto_1fr] gap-x-5 gap-y-2 rounded-xl bg-neutral-50 p-4 font-mono text-xs dark:bg-white/[0.035]"
			>
				<dt class="text-neutral-400 dark:text-white/35">Time</dt>
				<dd>
					{new Date(entry.ts * 1000).toLocaleString()}
					<span class="text-neutral-400 dark:text-white/35">({entry.ts})</span>
				</dd>
				<dt class="text-neutral-400 dark:text-white/35">Client IP</dt>
				<dd>{entry.request.client_ip || '—'}</dd>
				{#if entry.request.remote_ip && entry.request.remote_ip !== entry.request.client_ip}
					<dt class="text-neutral-400 dark:text-white/35">Remote IP</dt>
					<dd>
						{entry.request.remote_ip}{entry.request.remote_port
							? `:${entry.request.remote_port}`
							: ''}
					</dd>
				{/if}
				<dt class="text-neutral-400 dark:text-white/35">Protocol</dt>
				<dd>{entry.request.proto}</dd>
				{#if entry.user_id}
					<dt class="text-neutral-400 dark:text-white/35">User ID</dt>
					<dd>{entry.user_id}</dd>
				{/if}
			</dl>
		</section>

		{#if entry.request.tls}
			{@const tls = entry.request.tls}
			<section>
				<h3 class="mb-2 text-xs font-semibold tracking-wide text-neutral-400 dark:text-white/40">
					TLS
				</h3>
				<dl
					class="grid grid-cols-[auto_1fr] gap-x-5 gap-y-2 rounded-xl bg-neutral-50 p-4 font-mono text-xs dark:bg-white/[0.035]"
				>
					<dt class="text-neutral-400 dark:text-white/35">Version</dt>
					<dd>{tlsVersion(tls.version)}</dd>
					<dt class="text-neutral-400 dark:text-white/35">Cipher</dt>
					<dd>0x{tls.cipher_suite.toString(16).toUpperCase()}</dd>
					<dt class="text-neutral-400 dark:text-white/35">ALPN</dt>
					<dd>{tls.proto || '—'}</dd>
					<dt class="text-neutral-400 dark:text-white/35">SNI</dt>
					<dd>{tls.server_name || '—'}</dd>
					<dt class="text-neutral-400 dark:text-white/35">Resumed</dt>
					<dd>{tls.resumed ? 'yes' : 'no'}</dd>
				</dl>
			</section>
		{/if}

		{#if Object.keys(entry.request.headers).length > 0}
			<section>
				<h3 class="mb-2 text-xs font-semibold tracking-wide text-neutral-400 dark:text-white/40">
					Request headers
				</h3>
				<div class="data-table-wrap">
					<table class="w-full font-mono text-xs">
						<tbody class="divide-y divide-neutral-100 dark:divide-white/5">
							{#each Object.entries(entry.request.headers).sort( ([a], [b]) => a.localeCompare(b) ) as [name, values] (name)}
								<tr>
									<th
										class="w-48 px-3 py-2 text-left align-top font-normal text-neutral-400 dark:text-white/35"
										>{name}</th
									>
									<td class="px-3 py-2 break-all text-neutral-700 dark:text-white/75"
										>{values.join(', ')}</td
									>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</section>
		{/if}

		<section>
			<h3 class="mb-2 text-xs font-semibold tracking-wide text-neutral-400 dark:text-white/40">
				Response
			</h3>
			<dl
				class="grid grid-cols-[auto_1fr] gap-x-5 gap-y-2 rounded-xl bg-neutral-50 p-4 font-mono text-xs dark:bg-white/[0.035]"
			>
				<dt class="text-neutral-400 dark:text-white/35">Status</dt>
				<dd class="font-semibold {statusColor(entry.status)}">{entry.status}</dd>
				<dt class="text-neutral-400 dark:text-white/35">Duration</dt>
				<dd>{formatDuration(entry.duration)}</dd>
				<dt class="text-neutral-400 dark:text-white/35">Response size</dt>
				<dd>{formatBytes(entry.size)}</dd>
				{#if entry.bytes_read > 0}
					<dt class="text-neutral-400 dark:text-white/35">Request body</dt>
					<dd>{formatBytes(entry.bytes_read)}</dd>
				{/if}
			</dl>
		</section>

		{#if Object.keys(entry.resp_headers ?? {}).length > 0}
			<section>
				<h3 class="mb-2 text-xs font-semibold tracking-wide text-neutral-400 dark:text-white/40">
					Response headers
				</h3>
				<div class="data-table-wrap">
					<table class="w-full font-mono text-xs">
						<tbody class="divide-y divide-neutral-100 dark:divide-white/5">
							{#each Object.entries(entry.resp_headers).sort( ([a], [b]) => a.localeCompare(b) ) as [name, values] (name)}
								<tr>
									<th
										class="w-48 px-3 py-2 text-left align-top font-normal text-neutral-400 dark:text-white/35"
										>{name}</th
									>
									<td class="px-3 py-2 break-all text-neutral-700 dark:text-white/75"
										>{values.join(', ')}</td
									>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</section>
		{/if}
	</div>

	{#snippet actions()}
		<button class="button-secondary" onclick={copyJson}>
			{copied ? 'Copied' : 'Copy JSON'}
		</button>
	{/snippet}
</AppDialog>
