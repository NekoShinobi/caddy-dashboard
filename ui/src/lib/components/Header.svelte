<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { Toggle } from 'bits-ui';
	import { theme } from '$lib/theme.svelte';
	import { anonymize } from '$lib/anonymize.svelte';
	import { auth } from '$lib/auth.svelte';
	import ThemePicker from './ThemePicker.svelte';

	const links = [
		{ href: '/', label: 'Overview' },
		{ href: '/logs', label: 'Logs' },
		{ href: '/graphs', label: 'Graphs' },
		{ href: '/map', label: 'Map' },
		{ href: '/reports', label: 'Reports' }
	] as const;

	let timeLabel = $state('—:—:—');

	onMount(() => {
		const update = () => {
			timeLabel = new Date().toLocaleTimeString([], {
				hour: '2-digit',
				minute: '2-digit',
				second: '2-digit'
			});
		};
		update();
		const id = setInterval(update, 1000);
		return () => clearInterval(id);
	});
</script>

<header class="app-header">
	<div class="header-inner">
		<a class="brand-block" href={resolve('/')} aria-label="Caddy Dashboard overview">
			<span class="brand-mark" aria-hidden="true">C</span>
			<span>
				<strong>Caddy</strong>
				<small>Dashboard</small>
			</span>
		</a>

		<nav class="primary-nav" aria-label="Primary navigation">
			{#each links as link (link.href)}
				{@const active = page.url.pathname === link.href}
				<a href={resolve(link.href)} class:active aria-current={active ? 'page' : undefined}>
					{link.label}
				</a>
			{/each}
		</nav>

		<div class="header-actions">
			<span class="header-clock">{timeLabel}</span>
			<ThemePicker />

			<Toggle.Root
				pressed={anonymize.on}
				onPressedChange={(pressed) => {
					if (pressed !== anonymize.on) anonymize.toggle();
				}}
				aria-label={anonymize.on ? 'Disable anonymize mode' : 'Enable anonymize mode'}
				title={anonymize.on ? 'Anonymize on' : 'Anonymize off'}
				class="icon-button"
			>
				<svg
					aria-hidden="true"
					width="17"
					height="17"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="1.8"
				>
					<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
					<path d="M9.5 12.5 11 14l3.5-4" />
				</svg>
			</Toggle.Root>

			<Toggle.Root
				pressed={theme.dark}
				onPressedChange={(pressed) => {
					if (pressed !== theme.dark) theme.toggle();
				}}
				aria-label={theme.dark ? 'Use light appearance' : 'Use dark appearance'}
				title={theme.dark ? 'Dark appearance' : 'Light appearance'}
				class="icon-button"
			>
				{#if theme.dark}
					<svg
						aria-hidden="true"
						width="17"
						height="17"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="1.8"
					>
						<circle cx="12" cy="12" r="4" />
						<path
							d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"
						/>
					</svg>
				{:else}
					<svg
						aria-hidden="true"
						width="17"
						height="17"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="1.8"
					>
						<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
					</svg>
				{/if}
			</Toggle.Root>

			{#if auth.user}
				<span class="user-name">{auth.user.username}</span>
				<a
					href={resolve('/settings')}
					aria-label="Settings"
					title="Settings"
					class="icon-button"
					class:current-control={page.url.pathname === '/settings'}
				>
					<svg
						aria-hidden="true"
						width="17"
						height="17"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="1.8"
					>
						<circle cx="12" cy="12" r="3" />
						<path
							d="M19.4 15a1.7 1.7 0 0 0 .34 1.85l.05.05a2 2 0 1 1-2.83 2.83l-.05-.05a1.7 1.7 0 0 0-1.85-.34 1.7 1.7 0 0 0-1.06 1.57V21a2 2 0 1 1-4 0v-.09a1.7 1.7 0 0 0-1.06-1.57 1.7 1.7 0 0 0-1.85.34l-.05.05a2 2 0 1 1-2.83-2.83l.05-.05A1.7 1.7 0 0 0 4.6 15a1.7 1.7 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.7 1.7 0 0 0 4.6 9a1.7 1.7 0 0 0-.34-1.85l-.05-.05a2 2 0 1 1 2.83-2.83l.05.05A1.7 1.7 0 0 0 8.94 4.6 1.7 1.7 0 0 0 10 3.09V3a2 2 0 1 1 4 0v.09a1.7 1.7 0 0 0 1.06 1.51 1.7 1.7 0 0 0 1.85-.34l.05-.05a2 2 0 1 1 2.83 2.83l-.05.05A1.7 1.7 0 0 0 19.4 9c.2.62.8 1 1.51 1H21a2 2 0 1 1 0 4h-.09c-.7 0-1.3.38-1.51 1z"
						/>
					</svg>
				</a>
				<button
					onclick={() => auth.logout()}
					aria-label="Sign out"
					title="Sign out"
					class="icon-button"
				>
					<svg
						aria-hidden="true"
						width="17"
						height="17"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="1.8"
					>
						<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9" />
					</svg>
				</button>
			{/if}
		</div>
	</div>
</header>

<style>
	.app-header {
		position: sticky;
		top: 0;
		z-index: 40;
		border-bottom: 1px solid var(--app-border);
		background: color-mix(in oklch, var(--app-bg) 90%, transparent);
		backdrop-filter: blur(18px) saturate(1.2);
	}

	.header-inner {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		max-width: 1240px;
		min-height: 68px;
		margin-inline: auto;
		padding-inline: 28px;
		gap: 28px;
	}

	.brand-block {
		display: inline-flex;
		align-items: center;
		gap: 10px;
		color: var(--app-fg);
		text-decoration: none;
	}

	.brand-mark {
		display: grid;
		width: 34px;
		height: 34px;
		place-items: center;
		border-radius: 10px;
		background: var(--app-fg);
		color: var(--app-bg);
		font-family: var(--font-mono);
		font-size: 0.9rem;
		font-weight: 700;
	}

	.brand-block strong,
	.brand-block small {
		display: block;
		line-height: 1.05;
	}

	.brand-block strong {
		font-size: 0.9rem;
		font-weight: 720;
		letter-spacing: -0.02em;
	}

	.brand-block small {
		margin-top: 3px;
		color: var(--app-muted);
		font-family: var(--font-mono);
		font-size: 0.62rem;
		letter-spacing: 0.03em;
	}

	.primary-nav {
		display: flex;
		align-items: stretch;
		align-self: stretch;
		overflow-x: auto;
		gap: 2px;
		scrollbar-width: none;
	}

	.primary-nav::-webkit-scrollbar {
		display: none;
	}

	.primary-nav a {
		position: relative;
		display: inline-flex;
		min-width: max-content;
		align-items: center;
		padding-inline: 13px;
		color: var(--app-muted);
		font-size: 0.84rem;
		font-weight: 560;
		text-decoration: none;
		transition: color 150ms ease;
	}

	.primary-nav a::after {
		position: absolute;
		right: 13px;
		bottom: -1px;
		left: 13px;
		height: 2px;
		transform: scaleX(0);
		background: var(--app-accent);
		content: '';
		transition: transform 150ms ease;
	}

	.primary-nav a:hover,
	.primary-nav a.active {
		color: var(--app-fg);
	}

	.primary-nav a.active::after {
		transform: scaleX(1);
	}

	.header-actions {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.header-clock {
		margin-right: 4px;
		color: var(--app-muted);
		font-family: var(--font-mono);
		font-size: 0.68rem;
		font-variant-numeric: tabular-nums;
	}

	.user-name {
		max-width: 120px;
		overflow: hidden;
		margin-left: 3px;
		color: var(--app-muted);
		font-size: 0.74rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.current-control {
		border-color: var(--app-accent);
		background: var(--app-accent-soft);
		color: var(--app-accent-strong);
	}

	@media (max-width: 920px) {
		.header-inner {
			grid-template-columns: 1fr auto;
			padding: 10px 16px 0;
			gap: 8px 16px;
		}

		.primary-nav {
			grid-column: 1 / -1;
			order: 3;
			min-height: 46px;
		}

		.primary-nav a {
			padding-inline: 12px;
		}

		.header-clock,
		.user-name {
			display: none;
		}
	}

	@media (max-width: 520px) {
		.header-actions {
			gap: 5px;
		}

		.brand-block small {
			display: none;
		}

		.header-actions :global(.icon-button) {
			width: 44px;
			min-height: 44px;
		}
	}
</style>
