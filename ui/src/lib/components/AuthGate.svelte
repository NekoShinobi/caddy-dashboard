<script lang="ts">
	import { onMount } from 'svelte';
	import { auth } from '$lib/auth.svelte';
	import { MIN_PASSWORD_LENGTH } from '$lib/crypto';

	let { children } = $props();

	let username = $state('');
	let email = $state('');
	let password = $state('');
	let error = $state('');
	let submitting = $state(false);

	// Map OIDC error codes to human-readable messages
	const OIDC_ERROR_MESSAGES: Record<string, string> = {
		state_mismatch: 'Authentication session expired or was tampered with. Please try again.',
		provider_error: 'The identity provider rejected the login request.',
		missing_code: 'No authorization code returned by the provider.',
		config_error: 'OIDC is misconfigured on the server. Contact your administrator.',
		token_exchange_failed: 'Failed to exchange the authorization code for tokens.',
		userinfo_failed: 'Failed to retrieve your profile from the identity provider.',
		no_email: 'Your identity provider account does not have an email address.',
		email_not_verified: 'Your identity provider account email is not verified.',
		create_user_failed: 'Failed to create your account. Contact your administrator.',
		session_failed: 'Failed to create a session after login.'
	};

	onMount(() => {
		auth.check();
		// Show OIDC error from redirect query param
		const params = new URLSearchParams(window.location.search);
		const oidcError = params.get('oidc_error');
		if (oidcError) {
			error = OIDC_ERROR_MESSAGES[oidcError] ?? `Sign-in failed (${oidcError}).`;
			// Clean the URL so a refresh doesn't re-show the error
			const clean = window.location.pathname;
			window.history.replaceState({}, '', clean);
		}
	});

	$effect(() => {
		if (!auth.user && !auth.loading) {
			username = '';
			email = '';
			password = '';
			if (!error) error = '';
		}
	});

	async function handleLogin(e: SubmitEvent) {
		e.preventDefault();
		submitting = true;
		error = '';
		error = (await auth.login(username, password)) ?? '';
		submitting = false;
	}

	async function handleSignup(e: SubmitEvent) {
		e.preventDefault();
		submitting = true;
		error = '';
		error = (await auth.signup(username, email, password)) ?? '';
		submitting = false;
	}

	const oidc = $derived(auth.oidcConfig);
	const showLocal = $derived(!oidc?.disable_login);
	const showOidc = $derived(oidc?.enabled ?? false);
</script>

{#if auth.loading}
	<div class="flex h-screen items-center justify-center">
		<div
			class="h-6 w-6 animate-spin rounded-full border-2 border-neutral-300 border-t-neutral-600 dark:border-white/20 dark:border-t-white/60"
		></div>
	</div>
{:else if auth.user}
	{@render children()}
{:else}
	<div class="flex min-h-screen items-center justify-center px-4">
		<div class="w-full max-w-sm">
			<div class="mb-8 text-center">
				<h1 class="text-2xl font-bold">Caddy Dashboard</h1>
				<p class="mt-1 text-sm text-neutral-500 dark:text-white/40">
					{#if showLocal && auth.needsSetup}
						Create your admin account to get started
					{:else if showOidc && !showLocal}
						Sign in with {oidc?.provider_name ?? 'SSO'} to continue
					{:else}
						Sign in to continue
					{/if}
				</p>
			</div>

			<div
				class="rounded-xl border border-neutral-200 bg-white p-6 shadow-sm dark:border-white/10 dark:bg-neutral-900"
			>
				{#if auth.needsSetup && showLocal}
					<div
						class="mb-5 rounded-lg border border-blue-200 bg-blue-50 px-4 py-3 text-sm text-blue-700 dark:border-blue-500/30 dark:bg-blue-500/10 dark:text-blue-400"
					>
						No users exist yet. The first account created will be the admin.
					</div>
				{/if}

				{#if error}
					<p
						class="mb-4 rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-600 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-400"
					>
						{error}
					</p>
				{/if}

				<!-- OIDC login button -->
				{#if showOidc}
					<a
						href="/api/auth/oidc/login"
						class="flex w-full items-center justify-center gap-2.5 rounded-lg border border-neutral-200 bg-neutral-50 px-4 py-2.5 text-sm font-medium transition-colors hover:bg-neutral-100 dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10"
					>
						{#if oidc?.logo_url}
							<img src={oidc.logo_url} alt="" class="h-5 w-5 shrink-0 object-contain" />
						{:else}
							<svg
								xmlns="http://www.w3.org/2000/svg"
								class="h-4 w-4 shrink-0"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
								stroke-linecap="round"
								stroke-linejoin="round"
							>
								<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
							</svg>
						{/if}
						Sign in with {oidc?.provider_name ?? 'SSO'}
					</a>

					{#if showLocal}
						<div class="my-5 flex items-center gap-3">
							<div class="h-px flex-1 bg-neutral-200 dark:bg-white/10"></div>
							<span class="text-xs text-neutral-400 dark:text-white/30">or</span>
							<div class="h-px flex-1 bg-neutral-200 dark:bg-white/10"></div>
						</div>
					{/if}
				{/if}

				<!-- Local login / signup form -->
				{#if showLocal}
					<form
						onsubmit={auth.needsSetup ? handleSignup : handleLogin}
						class="space-y-4"
						autocomplete="on"
					>
						<div>
							<label class="mb-1.5 block text-sm font-medium" for="auth-username">Username</label>
							<input
								id="auth-username"
								type="text"
								bind:value={username}
								required
								autocomplete="username"
								class="w-full rounded-lg border border-neutral-200 bg-neutral-50 px-3 py-2 text-sm outline-none focus:border-neutral-400 focus:ring-1 focus:ring-neutral-400 dark:border-white/10 dark:bg-white/5 dark:focus:border-white/30 dark:focus:ring-white/30"
							/>
						</div>

						{#if auth.needsSetup}
							<div>
								<label class="mb-1.5 block text-sm font-medium" for="auth-email"
									>Email address</label
								>
								<input
									id="auth-email"
									type="email"
									bind:value={email}
									required
									autocomplete="email"
									class="w-full rounded-lg border border-neutral-200 bg-neutral-50 px-3 py-2 text-sm outline-none focus:border-neutral-400 focus:ring-1 focus:ring-neutral-400 dark:border-white/10 dark:bg-white/5 dark:focus:border-white/30 dark:focus:ring-white/30"
								/>
							</div>
						{/if}

						<div>
							<label class="mb-1.5 block text-sm font-medium" for="auth-password">Password</label>
							<input
								id="auth-password"
								type="password"
								bind:value={password}
								required
								minlength={auth.needsSetup ? MIN_PASSWORD_LENGTH : undefined}
								autocomplete={auth.needsSetup ? 'new-password' : 'current-password'}
								class="w-full rounded-lg border border-neutral-200 bg-neutral-50 px-3 py-2 text-sm outline-none focus:border-neutral-400 focus:ring-1 focus:ring-neutral-400 dark:border-white/10 dark:bg-white/5 dark:focus:border-white/30 dark:focus:ring-white/30"
							/>
							{#if auth.needsSetup}
								<p class="mt-1 text-xs text-neutral-400 dark:text-white/30">
									Minimum {MIN_PASSWORD_LENGTH} characters
								</p>
							{/if}
						</div>

						<button
							type="submit"
							disabled={submitting}
							class="w-full rounded-lg bg-neutral-900 px-4 py-2.5 text-sm font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-50 dark:bg-white dark:text-neutral-900"
						>
							{#if submitting}
								<span class="flex items-center justify-center gap-2">
									<svg class="h-4 w-4 animate-spin" viewBox="0 0 24 24" fill="none">
										<circle
											class="opacity-25"
											cx="12"
											cy="12"
											r="10"
											stroke="currentColor"
											stroke-width="4"
										/>
										<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8v8z" />
									</svg>
									{auth.needsSetup ? 'Creating account…' : 'Signing in…'}
								</span>
							{:else}
								{auth.needsSetup ? 'Create account' : 'Sign in'}
							{/if}
						</button>
					</form>
				{/if}
			</div>
		</div>
	</div>
{/if}
