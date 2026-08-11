<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { Popover, Toggle } from 'bits-ui';
	import { theme } from '$lib/theme.svelte';
	import { anonymize } from '$lib/anonymize.svelte';
	import { auth } from '$lib/auth.svelte';
	import ThemePicker from './ThemePicker.svelte';

	const workspaceLinks = [
		{ href: '/', label: 'Overview', key: 'G O' },
		{ href: '/logs', label: 'Access logs', key: 'G L' },
		{ href: '/graphs', label: 'Performance', key: 'G P' },
		{ href: '/map', label: 'Request map', key: 'G M' },
		{ href: '/reports', label: 'Reports', key: 'G R' }
	] as const;

	let menuOpen = $state(false);
	let timeLabel = $state('00:00:00');
	let currentLabel = $derived(
		workspaceLinks.find((link) => page.url.pathname === link.href)?.label ??
			(page.url.pathname === '/settings' ? 'Settings' : 'Dashboard')
	);

	function closeMenu() {
		menuOpen = false;
	}

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

<svelte:window
	onkeydown={(event) => {
		if (event.key === 'Escape') closeMenu();
	}}
/>

{#if menuOpen}
	<button class="nav-backdrop" type="button" aria-label="Close navigation" onclick={closeMenu}></button>
{/if}

<aside class={['app-sidebar-v2', { open: menuOpen }]} data-od-id="primary-sidebar">
	<a class="brand-v2" href={resolve('/')} aria-label="Caddy Dashboard overview" onclick={closeMenu}>
		<span class="brand-mark-v2" aria-hidden="true">C</span>
		<span class="brand-copy-v2">
			<strong>Caddy Dashboard</strong>
			<small>Private log analytics</small>
		</span>
	</a>

	<p class="nav-label-v2">Workspace</p>
	<nav id="primary-navigation" class="nav-list-v2" aria-label="Primary navigation">
		{#each workspaceLinks as link (link.href)}
			{@const active = page.url.pathname === link.href}
			<a
				href={resolve(link.href)}
				class={['nav-item-v2', { active }]}
				aria-current={active ? 'page' : undefined}
				onclick={closeMenu}
			>
				<span>{link.label}</span>
				<span class="nav-key-v2">{link.key}</span>
			</a>
		{/each}
	</nav>

	<p class="nav-label-v2">System</p>
	<nav class="nav-list-v2" aria-label="System navigation">
		<a
			href={resolve('/settings')}
			class={['nav-item-v2', { active: page.url.pathname === '/settings' }]}
			aria-current={page.url.pathname === '/settings' ? 'page' : undefined}
			onclick={closeMenu}
		>
			<span>Settings</span>
		</a>
	</nav>

	<div class="sidebar-footer-v2">
		<a
			class="github-star-v2"
			href="https://github.com/NekoShinobi/caddy-dashboard"
			target="_blank"
			rel="noopener noreferrer"
			data-od-id="github-star-link"
			onclick={closeMenu}
		>
			<svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
				<path d="m12 3 2.78 5.63 6.22.91-4.5 4.38 1.06 6.19L12 17.19l-5.56 2.92 1.06-6.19L3 9.54l6.22-.91L12 3z" />
			</svg>
			<span>
				<small>Enjoying the tool?</small>
				<strong>Star on GitHub</strong>
			</span>
		</a>

		<div class="sidebar-status-v2" data-od-id="source-status">
			<span class="source-indicator-v2" aria-hidden="true"></span>
			<span>
				<strong>Private instance</strong>
				<small>Access log source ready</small>
			</span>
		</div>
	</div>
</aside>

<header class="command-bar-v2" data-od-id="command-bar">
	<div class="command-context-v2">
		<button
			class="menu-control-v2"
			type="button"
			aria-controls="primary-navigation"
			aria-expanded={menuOpen}
			onclick={() => (menuOpen = !menuOpen)}
		>
			Menu
		</button>
		<p>Caddy Dashboard / <strong>{currentLabel}</strong></p>
	</div>

	<div class="command-actions-v2">
		<span class="header-clock-v2">{timeLabel}</span>
		<ThemePicker />
		<Toggle.Root
			pressed={anonymize.on}
			onPressedChange={(pressed) => {
				if (pressed !== anonymize.on) anonymize.toggle();
			}}
			aria-label={anonymize.on ? 'Disable privacy masking' : 'Enable privacy masking'}
			title={anonymize.on ? 'Privacy masking on' : 'Privacy masking off'}
			class="command-control-v2 command-toggle-v2"
		>
			<svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
				<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
				{#if anonymize.on}<path d="M9.5 12.5 11 14l3.5-4" />{/if}
			</svg>
		</Toggle.Root>
		<Toggle.Root
			pressed={theme.dark}
			onPressedChange={(pressed) => {
				if (pressed !== theme.dark) theme.toggle();
			}}
			aria-label={theme.dark ? 'Use light appearance' : 'Use dark appearance'}
			title={theme.dark ? 'Switch to light appearance' : 'Switch to dark appearance'}
			class="command-control-v2 command-toggle-v2"
		>
			{#if theme.dark}
				<svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
					<circle cx="12" cy="12" r="4" />
					<path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" />
				</svg>
			{:else}
				<svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
					<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
				</svg>
			{/if}
		</Toggle.Root>
		{#if auth.user}
			<Popover.Root>
				<Popover.Trigger
					class="avatar-trigger-v2"
					aria-label="Open user menu"
					title="User menu"
					data-od-id="user-menu-trigger"
					openOnHover
					openDelay={80}
					closeDelay={180}
				>
					<svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
						<circle cx="12" cy="8" r="3.5" />
						<path d="M5.5 20a6.5 6.5 0 0 1 13 0" />
					</svg>
				</Popover.Trigger>

				<Popover.Portal>
					<Popover.Content
						class="dropdown-content user-menu-v2"
						sideOffset={8}
						align="end"
					>
						<div class="user-menu-identity-v2">
							<span>Signed in as</span>
							<strong>{auth.user.username}</strong>
						</div>
						<div class="user-menu-separator-v2" aria-hidden="true"></div>
						<Popover.Close
							class="dropdown-item user-signout-v2"
							onclick={() => auth.logout()}
							data-od-id="user-signout"
						>
							<svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
								<path d="M10 17l5-5-5-5M15 12H3" />
								<path d="M14 4h4a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-4" />
							</svg>
							<span>Sign out</span>
						</Popover.Close>
					</Popover.Content>
				</Popover.Portal>
			</Popover.Root>
		{/if}
	</div>
</header>

<style>
	.app-sidebar-v2 {
		grid-area: sidebar;
		position: sticky;
		top: 0;
		display: flex;
		height: 100dvh;
		min-width: 0;
		flex-direction: column;
		border-right: 1px solid var(--app-border);
		background: var(--app-surface);
		padding: 20px 14px 16px;
	}

	.brand-v2 {
		display: flex;
		min-height: 52px;
		align-items: center;
		gap: 11px;
		border-radius: var(--app-radius-sm);
		padding: 0 8px;
		color: var(--app-fg);
		text-decoration: none;
	}

	.brand-v2:hover { background: var(--app-surface-muted); }

	.brand-mark-v2 {
		display: grid;
		width: 36px;
		height: 36px;
		place-items: center;
		flex: 0 0 auto;
		border-radius: 9px;
		background: var(--app-fg);
		color: var(--app-bg);
		font-family: var(--font-mono);
		font-size: 0.8rem;
		font-weight: 700;
	}

	.brand-copy-v2 { display: grid; line-height: 1.1; }
	.brand-copy-v2 strong { font-size: 0.86rem; font-weight: 650; letter-spacing: -0.01em; }
	.brand-copy-v2 small {
		margin-top: 4px;
		color: var(--app-muted);
		font-family: var(--font-mono);
		font-size: 0.61rem;
		letter-spacing: 0.02em;
	}

	.nav-label-v2 {
		margin: 22px 8px 8px;
		color: var(--app-muted);
		font-family: var(--font-mono);
		font-size: 0.62rem;
		font-weight: 620;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	.nav-list-v2 { display: grid; gap: 3px; }

	.nav-item-v2 {
		position: relative;
		display: flex;
		min-height: 44px;
		align-items: center;
		justify-content: space-between;
		border: 1px solid transparent;
		border-radius: var(--app-radius-sm);
		padding: 0 12px;
		color: var(--app-muted);
		font-size: 0.84rem;
		font-weight: 540;
		text-decoration: none;
		transition: background 100ms ease, border-color 100ms ease, color 100ms ease;
	}

	.nav-item-v2:hover { background: var(--app-surface-muted); color: var(--app-fg); }
	.nav-item-v2.active { border-color: var(--app-border); background: var(--app-accent-soft); color: var(--app-fg); }
	.nav-item-v2.active::before {
		position: absolute;
		top: 11px;
		bottom: 11px;
		left: -15px;
		width: 3px;
		border-radius: 0 3px 3px 0;
		background: var(--app-accent);
		content: '';
	}

	.nav-key-v2 { color: var(--app-muted); font-family: var(--font-mono); font-size: 0.6rem; }

	.sidebar-footer-v2 {
		display: grid;
		gap: 8px;
		margin-top: auto;
	}

	.github-star-v2 {
		display: flex;
		min-height: 48px;
		align-items: center;
		gap: 10px;
		border: 1px solid var(--app-border);
		border-radius: var(--app-radius-sm);
		background: transparent;
		color: var(--app-fg);
		padding: 8px 10px;
		text-decoration: none;
		transition: background 100ms ease, border-color 100ms ease, transform 80ms ease;
	}

	.github-star-v2:hover {
		border-color: var(--app-border-strong);
		background: var(--app-surface-muted);
	}

	.github-star-v2:active { transform: translateY(1px); }
	.github-star-v2 svg { width: 17px; height: 17px; flex: 0 0 17px; }
	.github-star-v2 span { display: grid; min-width: 0; gap: 2px; }
	.github-star-v2 strong { font-size: 0.72rem; font-weight: 620; }
	.github-star-v2 small { color: var(--app-muted); font-size: 0.62rem; }

	.sidebar-status-v2 {
		display: grid;
		grid-template-columns: 8px 1fr;
		align-items: start;
		gap: 10px;
		border-top: 1px solid var(--app-border);
		padding: 18px 8px 8px;
	}

	.source-indicator-v2 { width: 8px; height: 8px; margin-top: 5px; border-radius: 50%; background: var(--app-success); }
	.sidebar-status-v2 strong { display: block; font-size: 0.72rem; font-weight: 620; }
	.sidebar-status-v2 small { display: block; margin-top: 3px; color: var(--app-muted); font-family: var(--font-mono); font-size: 0.61rem; }

	.command-bar-v2 {
		grid-area: command;
		position: sticky;
		top: 0;
		z-index: 30;
		display: flex;
		min-width: 0;
		min-height: 68px;
		align-items: center;
		justify-content: space-between;
		gap: 18px;
		border-bottom: 1px solid var(--app-border);
		background: color-mix(in oklch, var(--app-bg) 92%, transparent);
		backdrop-filter: blur(14px);
		padding: 10px clamp(18px, 3vw, 36px);
	}

	.command-context-v2,
	.command-actions-v2 { display: flex; min-width: 0; align-items: center; gap: 8px; }
	.command-context-v2 p { overflow: hidden; color: var(--app-muted); font-size: 0.78rem; text-overflow: ellipsis; white-space: nowrap; }
	.command-context-v2 strong { color: var(--app-fg); font-weight: 620; }

	.menu-control-v2,
	:global(.command-control-v2) {
		display: inline-flex;
		min-height: 44px;
		align-items: center;
		justify-content: center;
		gap: 8px;
		border: 1px solid var(--app-border);
		border-radius: var(--app-radius-sm);
		background: var(--app-surface);
		color: var(--app-fg);
		padding: 0 12px;
		font-size: 0.72rem;
		font-weight: 620;
		transition: background 100ms ease, border-color 100ms ease, transform 80ms ease;
	}

	.menu-control-v2 { display: none; }
	.menu-control-v2:hover,
	:global(.command-control-v2:hover) { border-color: var(--app-border-strong); background: var(--app-surface-muted); }
	.menu-control-v2:active,
	:global(.command-control-v2:active) { transform: translateY(1px); }
	:global(.command-control-v2[aria-pressed='true']) { border-color: var(--app-fg); background: var(--app-fg); color: var(--app-bg); }
	:global(.command-control-v2[aria-pressed='true']:hover) {
		border-color: var(--app-fg);
		background: color-mix(in oklch, var(--app-fg) 88%, var(--app-bg));
		color: var(--app-bg);
	}
	:global(.command-toggle-v2) {
		width: 44px;
		min-width: 44px;
		flex: 0 0 44px;
		padding: 0;
	}
	:global(.command-toggle-v2 svg) { width: 18px; height: 18px; }
	:global(.avatar-trigger-v2) {
		display: inline-grid;
		width: 44px;
		height: 44px;
		flex: 0 0 44px;
		place-items: center;
		border: 1px solid var(--app-border);
		border-radius: 50%;
		background: var(--app-surface-muted);
		color: var(--app-fg);
		transition: background 100ms ease, border-color 100ms ease, transform 80ms ease;
	}
	:global(.avatar-trigger-v2:hover),
	:global(.avatar-trigger-v2[data-state='open']) {
		border-color: var(--app-border-strong);
		background: var(--app-surface);
	}
	:global(.avatar-trigger-v2:active) { transform: translateY(1px); }
	:global(.avatar-trigger-v2 svg) { width: 19px; height: 19px; }
	:global(.user-menu-v2) { min-width: 232px; }
	:global(.user-menu-identity-v2) {
		display: grid;
		gap: 3px;
		padding: 10px;
	}
	:global(.user-menu-identity-v2 span) {
		color: var(--app-muted);
		font-size: 0.68rem;
		font-weight: 620;
		letter-spacing: 0.06em;
		text-transform: uppercase;
	}
	:global(.user-menu-identity-v2 strong) {
		overflow-wrap: anywhere;
		font-size: 0.86rem;
		font-weight: 620;
		line-height: 1.4;
	}
	:global(.user-menu-separator-v2) {
		height: 1px;
		margin: 4px 5px;
		background: var(--app-border);
	}
	:global(.user-signout-v2) {
		width: 100%;
		border: 0;
		background: transparent;
		color: var(--app-danger);
		font-weight: 620;
		text-align: left;
	}
	:global(.user-signout-v2:hover),
	:global(.user-signout-v2:focus-visible) {
		background: color-mix(in oklch, var(--app-danger) 10%, var(--app-surface));
		color: color-mix(in oklch, var(--app-danger) 86%, var(--app-fg));
	}
	:global(.user-signout-v2 svg) { width: 17px; height: 17px; }
	.header-clock-v2 { margin-right: 3px; color: var(--app-muted); font-family: var(--font-mono); font-size: 0.66rem; font-variant-numeric: tabular-nums; }

	.nav-backdrop {
		position: fixed;
		inset: 0;
		z-index: 39;
		display: none;
		width: 100%;
		height: 100%;
		border: 0;
		background: color-mix(in oklch, var(--app-bg) 66%, transparent);
		padding: 0;
	}

	@media (max-width: 1080px) {
		.header-clock-v2 { display: none; }
	}

	@media (max-width: 920px) {
		.app-sidebar-v2 {
			position: fixed;
			inset: 0 auto 0 0;
			z-index: 40;
			width: min(86vw, 280px);
			transform: translateX(-105%);
			box-shadow: 18px 0 50px color-mix(in oklch, var(--app-bg) 68%, transparent);
			transition: transform 220ms cubic-bezier(0.2, 0, 0, 1);
		}
		.app-sidebar-v2.open { transform: translateX(0); }
		.nav-backdrop { display: block; }
		.menu-control-v2 { display: inline-flex; }
		.command-bar-v2 { padding-inline: 16px; }
	}

	@media (max-width: 620px) {
		.command-context-v2 p,
		.command-actions-v2 :global(.icon-button) { display: none; }
		:global(.command-toggle-v2) { min-width: 44px; padding: 0; }
		.command-bar-v2 { gap: 10px; }
	}
</style>
