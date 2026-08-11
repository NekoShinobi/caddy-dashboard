<script lang="ts">
	import { onMount } from 'svelte';
	import { Tabs, ToggleGroup } from 'bits-ui';
	import SegmentedControl from '$lib/components/SegmentedControl.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import { auth } from '$lib/auth.svelte';
	import { theme } from '$lib/theme.svelte';
	import { colorTheme } from '$lib/color-theme.svelte';
	import { THEMES } from '$lib/themes';
	import { MIN_PASSWORD_LENGTH, hashPassword, validatePassword } from '$lib/crypto';

	type Tab = 'account' | 'users' | 'site';
	let tab = $state<Tab>('account');
	const modeOptions = [
		{ label: 'Light', value: 'light' },
		{ label: 'Dark', value: 'dark' }
	];

	function changeTab(value: string) {
		tab = value as Tab;
		if (tab === 'users' && auth.user?.is_admin) void fetchUsers();
		if (tab === 'site' && auth.user?.is_admin) void fetchPrompt();
	}

	function changeMode(value: string) {
		const shouldBeDark = value === 'dark';
		if (theme.dark !== shouldBeDark) theme.toggle();
	}

	// ── Account (change password) ────────────────────────────────────────────
	let currentPw = $state('');
	let newPw = $state('');
	let confirmPw = $state('');
	let pwError = $state('');
	let pwSuccess = $state('');
	let pwSubmitting = $state(false);

	async function handleChangePassword(e: SubmitEvent) {
		e.preventDefault();
		pwError = '';
		pwSuccess = '';
		if (newPw !== confirmPw) {
			pwError = 'Passwords do not match';
			return;
		}
		pwSubmitting = true;
		const err = await auth.changePassword(currentPw, newPw);
		pwSubmitting = false;
		if (err) {
			pwError = err;
		} else {
			pwSuccess = 'Password updated successfully';
			currentPw = newPw = confirmPw = '';
		}
	}

	function themeQuad(id: string): [string, string, string, string] {
		const t = THEMES.find((t) => t.id === id) ?? THEMES[0];
		const c = t[theme.dark ? 'dark' : 'light'];
		return [c.blue, c.green, c.red, c.purple];
	}

	// ── User management (admin) ───────────────────────────────────────────────
	interface UserRow {
		username: string;
		email: string;
		is_admin: boolean;
		created_at: number;
		is_oidc?: boolean;
	}
	let users = $state<UserRow[]>([]);
	let usersLoading = $state(false);
	let usersError = $state('');

	let newUsername = $state('');
	let newEmail = $state('');
	let newPassword = $state('');
	let newIsAdmin = $state(false);
	let createError = $state('');
	let createSuccess = $state('');
	let createSubmitting = $state(false);

	// Edit state
	let editTarget = $state<string | null>(null);
	let editUsername = $state('');
	let editEmail = $state('');
	let editIsAdmin = $state(false);
	let editError = $state('');
	let editSubmitting = $state(false);
	let deleteTarget = $state<string | null>(null);

	let adminCount = $derived(users.filter((u) => u.is_admin).length);

	async function fetchUsers() {
		usersLoading = true;
		usersError = '';
		try {
			const res = await fetch('/api/admin/users');
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			const data = await res.json();
			users = data.users ?? [];
		} catch (e) {
			usersError = e instanceof Error ? e.message : 'Failed to load users';
		} finally {
			usersLoading = false;
		}
	}

	async function handleCreateUser(e: SubmitEvent) {
		e.preventDefault();
		createError = '';
		createSuccess = '';
		const invalid = validatePassword(newPassword);
		if (invalid) {
			createError = invalid;
			return;
		}
		createSubmitting = true;
		try {
			const res = await fetch('/api/admin/users', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					username: newUsername,
					email: newEmail,
					password: await hashPassword(newPassword),
					is_admin: newIsAdmin
				})
			});
			const data = await res.json();
			if (!res.ok) {
				createError = data.error ?? 'Failed';
			} else {
				createSuccess = `User "${newUsername}" created`;
				newUsername = '';
				newEmail = '';
				newPassword = '';
				newIsAdmin = false;
				await fetchUsers();
			}
		} catch (e) {
			createError = e instanceof Error ? e.message : 'Failed to create user';
		} finally {
			createSubmitting = false;
		}
	}

	function startEdit(u: UserRow) {
		editTarget = u.username;
		editUsername = u.username;
		editEmail = u.email;
		editIsAdmin = u.is_admin;
		editError = '';
		resetTarget = null;
	}

	async function handleEditUser(originalUsername: string) {
		editError = '';
		editSubmitting = true;
		try {
			const res = await fetch(`/api/admin/users/${encodeURIComponent(originalUsername)}`, {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ username: editUsername, email: editEmail, is_admin: editIsAdmin })
			});
			const data = await res.json();
			if (!res.ok) {
				editError = data.error ?? 'Failed to update';
			} else {
				editTarget = null;
				await fetchUsers();
			}
		} catch (e) {
			editError = e instanceof Error ? e.message : 'Failed to update user';
		} finally {
			editSubmitting = false;
		}
	}

	async function handleDeleteUser(username: string) {
		usersError = '';
		try {
			const res = await fetch(`/api/admin/users/${encodeURIComponent(username)}`, {
				method: 'DELETE'
			});
			const data = (await res.json().catch(() => ({}))) as { error?: string };
			if (!res.ok) throw new Error(data.error ?? 'Failed to delete');
			await fetchUsers();
		} catch (e) {
			usersError = e instanceof Error ? e.message : 'Failed to delete user';
		}
	}

	// Reset password state per user
	let resetTarget = $state<string | null>(null);
	let resetPw = $state('');
	let resetError = $state('');
	let resetSubmitting = $state(false);

	async function handleResetPassword(username: string) {
		resetError = '';
		const invalid = validatePassword(resetPw);
		if (invalid) {
			resetError = invalid;
			return;
		}
		resetSubmitting = true;
		try {
			const res = await fetch(`/api/admin/users/${encodeURIComponent(username)}/password`, {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ new_password: await hashPassword(resetPw) })
			});
			const data = await res.json();
			if (!res.ok) {
				resetError = data.error ?? 'Failed';
			} else {
				resetTarget = null;
				resetPw = '';
			}
		} catch (e) {
			resetError = e instanceof Error ? e.message : 'Failed to reset password';
		} finally {
			resetSubmitting = false;
		}
	}

	onMount(() => {
		if (auth.user?.is_admin) fetchUsers();
	});

	// ── Site settings (admin) ────────────────────────────────────────────────
	let promptTemplate = $state('');
	let promptDefault = $state('');
	let promptLoading = $state(false);
	let promptError = $state('');
	let promptSuccess = $state('');
	let promptSubmitting = $state(false);

	async function fetchPrompt() {
		promptLoading = true;
		promptError = '';
		try {
			const res = await fetch('/api/admin/settings/ai-prompt');
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			const data = await res.json();
			promptTemplate = data.template;
			promptDefault = data.default;
		} catch (e) {
			promptError = e instanceof Error ? e.message : 'Failed to load prompt';
		} finally {
			promptLoading = false;
		}
	}

	async function handleSavePrompt(e: SubmitEvent) {
		e.preventDefault();
		promptError = '';
		promptSuccess = '';
		promptSubmitting = true;
		try {
			const res = await fetch('/api/admin/settings/ai-prompt', {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ template: promptTemplate })
			});
			const data = await res.json().catch(() => ({}));
			if (!res.ok) {
				promptError = (data as { error?: string }).error ?? 'Failed to save';
			} else {
				promptSuccess = 'Prompt saved.';
			}
		} catch (e) {
			promptError = e instanceof Error ? e.message : 'Failed to save';
		} finally {
			promptSubmitting = false;
		}
	}

	function resetPrompt() {
		promptTemplate = promptDefault;
		promptSuccess = '';
		promptError = '';
	}
</script>

<div class="page-shell max-w-4xl" data-od-id="settings-page">
	<header class="page-header" data-od-id="settings-header">
		<div>
			<p class="page-eyebrow">Workspace</p>
			<h1 class="page-title">Settings</h1>
			<p class="page-description">
				Manage your profile, appearance, access, and traffic-analysis configuration.
			</p>
		</div>
	</header>

	<!-- Tabs -->
	<Tabs.Root value={tab} onValueChange={changeTab}>
		<Tabs.List class="segmented-control settings-tabs" aria-label="Settings sections">
			<Tabs.Trigger value="account" class="segmented-item">My account</Tabs.Trigger>
			{#if auth.user?.is_admin}
				<Tabs.Trigger value="users" class="segmented-item">User management</Tabs.Trigger>
				<Tabs.Trigger value="site" class="segmented-item">Site</Tabs.Trigger>
			{/if}
		</Tabs.List>
	</Tabs.Root>

	<!-- Account Tab -->
	{#if tab === 'account'}
		<div class="space-y-4">
			<div
				class="rounded-lg border border-neutral-200 bg-neutral-50 p-4 dark:border-white/10 dark:bg-white/[0.03]"
			>
				<div class="text-xs text-neutral-500 dark:text-white/40">Signed in as</div>
				<div class="mt-0.5 font-medium">{auth.user?.username}</div>
				{#if auth.user?.email}
					<div class="text-sm text-neutral-500 dark:text-white/40">{auth.user.email}</div>
				{/if}
				{#if auth.user?.is_admin}
					<span
						class="mt-1 inline-block rounded-full bg-violet-100 px-2 py-0.5 text-xs font-medium text-violet-700 dark:bg-violet-500/15 dark:text-violet-400"
						>Admin</span
					>
				{/if}
			</div>

			<!-- Theme Preferences -->
			<div
				class="rounded-xl border border-neutral-200 bg-white p-6 dark:border-white/10 dark:bg-neutral-900"
			>
				<h2 class="mb-4 font-semibold">Appearance</h2>
				<div class="space-y-4">
					<!-- Light/Dark -->
					<div class="setting-row">
						<div>
							<div class="text-sm font-medium">Mode</div>
							<p class="text-xs text-[var(--app-muted-fg)]">
								Choose the contrast best suited to your environment.
							</p>
						</div>
						<SegmentedControl
							value={theme.dark ? 'dark' : 'light'}
							options={modeOptions}
							onchange={changeMode}
							label="Color mode"
						/>
					</div>
					<!-- Color Theme -->
					<div>
						<div class="mb-2 text-sm font-medium">Color theme</div>
						<ToggleGroup.Root
							type="single"
							value={colorTheme.id}
							onValueChange={(value) => {
								if (value) colorTheme.set(value);
							}}
							class="theme-grid"
							aria-label="Color theme"
						>
							{#each THEMES as option (option.id)}
								{@const [c1, c2, c3, c4] = themeQuad(option.id)}
								<ToggleGroup.Item value={option.id} class="theme-option">
									<svg
										width="16"
										height="16"
										viewBox="0 0 16 16"
										class="shrink-0"
										aria-hidden="true"
									>
										<rect x="0" y="0" width="7" height="7" fill={c1} rx="1.5" />
										<rect x="9" y="0" width="7" height="7" fill={c2} rx="1.5" />
										<rect x="0" y="9" width="7" height="7" fill={c3} rx="1.5" />
										<rect x="9" y="9" width="7" height="7" fill={c4} rx="1.5" />
									</svg>
									<span class="truncate">{option.name}</span>
									{#if colorTheme.id === option.id}<span class="ml-auto" aria-hidden="true">✓</span
										>{/if}
								</ToggleGroup.Item>
							{/each}
						</ToggleGroup.Root>
					</div>
				</div>
			</div>

			{#if !auth.user?.is_oidc}
				<div
					class="rounded-xl border border-neutral-200 bg-white p-6 dark:border-white/10 dark:bg-neutral-900"
				>
					<h2 class="mb-4 font-semibold">Change Password</h2>
					<form onsubmit={handleChangePassword} class="space-y-3">
						<div>
							<label class="mb-1 block text-sm font-medium" for="cur-pw">Current password</label>
							<input
								id="cur-pw"
								type="password"
								bind:value={currentPw}
								required
								autocomplete="current-password"
								class="w-full rounded-lg border border-neutral-200 bg-neutral-50 px-3 py-2 text-sm outline-none focus:border-neutral-400 focus:ring-1 focus:ring-neutral-400 dark:border-white/10 dark:bg-white/5 dark:focus:border-white/30 dark:focus:ring-white/30"
							/>
						</div>
						<div>
							<label class="mb-1 block text-sm font-medium" for="new-pw">New password</label>
							<input
								id="new-pw"
								type="password"
								bind:value={newPw}
								required
								minlength={MIN_PASSWORD_LENGTH}
								autocomplete="new-password"
								class="w-full rounded-lg border border-neutral-200 bg-neutral-50 px-3 py-2 text-sm outline-none focus:border-neutral-400 focus:ring-1 focus:ring-neutral-400 dark:border-white/10 dark:bg-white/5 dark:focus:border-white/30 dark:focus:ring-white/30"
							/>
						</div>
						<div>
							<label class="mb-1 block text-sm font-medium" for="confirm-pw"
								>Confirm new password</label
							>
							<input
								id="confirm-pw"
								type="password"
								bind:value={confirmPw}
								required
								minlength={MIN_PASSWORD_LENGTH}
								autocomplete="new-password"
								class="w-full rounded-lg border border-neutral-200 bg-neutral-50 px-3 py-2 text-sm outline-none focus:border-neutral-400 focus:ring-1 focus:ring-neutral-400 dark:border-white/10 dark:bg-white/5 dark:focus:border-white/30 dark:focus:ring-white/30"
							/>
						</div>
						{#if pwError}
							<p
								class="rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-600 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-400"
							>
								{pwError}
							</p>
						{/if}
						{#if pwSuccess}
							<p
								class="rounded-lg border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-700 dark:border-green-500/30 dark:bg-green-500/10 dark:text-green-400"
							>
								{pwSuccess}
							</p>
						{/if}
						<button
							type="submit"
							disabled={pwSubmitting}
							class="rounded-lg bg-neutral-900 px-4 py-2 text-sm font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-50 dark:bg-white dark:text-neutral-900"
						>
							{pwSubmitting ? 'Updating…' : 'Update password'}
						</button>
					</form>
				</div>
			{/if}
		</div>

		<!-- Users Tab (admin) -->
	{:else if tab === 'users'}
		<div class="space-y-6">
			<!-- Add user -->
			<div
				class="rounded-xl border border-neutral-200 bg-white p-6 dark:border-white/10 dark:bg-neutral-900"
			>
				<h2 class="mb-4 font-semibold">Add User</h2>
				<form onsubmit={handleCreateUser} class="space-y-3">
					<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
						<div>
							<label class="mb-1 block text-sm font-medium" for="new-user">Username</label>
							<input
								id="new-user"
								type="text"
								bind:value={newUsername}
								required
								class="w-full rounded-lg border border-neutral-200 bg-neutral-50 px-3 py-2 text-sm outline-none focus:border-neutral-400 focus:ring-1 focus:ring-neutral-400 dark:border-white/10 dark:bg-white/5 dark:focus:border-white/30"
							/>
						</div>
						<div>
							<label class="mb-1 block text-sm font-medium" for="new-user-email"
								>Email address</label
							>
							<input
								id="new-user-email"
								type="email"
								bind:value={newEmail}
								required
								class="w-full rounded-lg border border-neutral-200 bg-neutral-50 px-3 py-2 text-sm outline-none focus:border-neutral-400 focus:ring-1 focus:ring-neutral-400 dark:border-white/10 dark:bg-white/5 dark:focus:border-white/30"
							/>
						</div>
					</div>
					<div>
						<label class="mb-1 block text-sm font-medium" for="new-user-pw">Password</label>
						<input
							id="new-user-pw"
							type="password"
							bind:value={newPassword}
							required
							minlength={MIN_PASSWORD_LENGTH}
							autocomplete="new-password"
							class="w-full rounded-lg border border-neutral-200 bg-neutral-50 px-3 py-2 text-sm outline-none focus:border-neutral-400 focus:ring-1 focus:ring-neutral-400 dark:border-white/10 dark:bg-white/5 dark:focus:border-white/30"
						/>
					</div>
					<label class="flex items-center gap-2 text-sm">
						<input type="checkbox" bind:checked={newIsAdmin} class="rounded" />
						Grant admin privileges
					</label>
					{#if createError}
						<p
							class="rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-600 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-400"
						>
							{createError}
						</p>
					{/if}
					{#if createSuccess}
						<p
							class="rounded-lg border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-700 dark:border-green-500/30 dark:bg-green-500/10 dark:text-green-400"
						>
							{createSuccess}
						</p>
					{/if}
					<button
						type="submit"
						disabled={createSubmitting}
						class="rounded-lg bg-neutral-900 px-4 py-2 text-sm font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-50 dark:bg-white dark:text-neutral-900"
					>
						{createSubmitting ? 'Creating…' : 'Create user'}
					</button>
				</form>
			</div>

			<!-- User list -->
			<div
				class="rounded-xl border border-neutral-200 bg-white dark:border-white/10 dark:bg-neutral-900"
			>
				<div class="border-b border-neutral-200 px-5 py-4 dark:border-white/10">
					<h2 class="font-semibold">Users</h2>
				</div>
				{#if usersError}
					<p class="p-4 text-sm text-red-600 dark:text-red-400">{usersError}</p>
				{:else if usersLoading}
					<p class="p-4 text-sm text-neutral-400 dark:text-white/30">Loading…</p>
				{:else}
					<ul class="divide-y divide-neutral-100 dark:divide-white/5">
						{#each users as u (u.username)}
							<li class="px-5 py-3">
								{#if editTarget === u.username}
									<!-- Inline edit form -->
									<form
										onsubmit={(e) => {
											e.preventDefault();
											handleEditUser(u.username);
										}}
										class="space-y-3"
									>
										<div class="grid grid-cols-2 gap-3">
											<div>
												<label
													class="mb-1 block text-xs font-medium text-neutral-500 dark:text-white/40"
													for="edit-username">Username</label
												>
												<input
													id="edit-username"
													type="text"
													bind:value={editUsername}
													required
													class="w-full rounded-lg border border-neutral-200 bg-neutral-50 px-2 py-1.5 text-sm outline-none focus:border-neutral-400 dark:border-white/10 dark:bg-white/5 dark:focus:border-white/30"
												/>
											</div>
											<div>
												<label
													class="mb-1 block text-xs font-medium text-neutral-500 dark:text-white/40"
													for="edit-email">Email address</label
												>
												<input
													id="edit-email"
													type="email"
													bind:value={editEmail}
													required
													class="w-full rounded-lg border border-neutral-200 bg-neutral-50 px-2 py-1.5 text-sm outline-none focus:border-neutral-400 dark:border-white/10 dark:bg-white/5 dark:focus:border-white/30"
												/>
											</div>
										</div>
										<label
											class="flex items-center gap-2 text-sm {u.is_admin && adminCount <= 1
												? 'cursor-not-allowed opacity-50'
												: ''}"
										>
											<input
												type="checkbox"
												bind:checked={editIsAdmin}
												disabled={u.is_admin && adminCount <= 1}
												class="rounded"
											/>
											Admin
											{#if u.is_admin && adminCount <= 1}
												<span class="text-xs text-neutral-400 dark:text-white/30">(last admin)</span
												>
											{/if}
										</label>
										{#if editError}
											<p class="text-xs text-red-500 dark:text-red-400">{editError}</p>
										{/if}
										<div class="flex gap-2">
											<button
												type="submit"
												disabled={editSubmitting}
												class="rounded-lg bg-neutral-900 px-3 py-1.5 text-xs font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-50 dark:bg-white dark:text-neutral-900"
											>
												{editSubmitting ? 'Saving…' : 'Save'}
											</button>
											<button
												type="button"
												onclick={() => {
													editTarget = null;
													editError = '';
												}}
												class="rounded-lg border border-neutral-200 px-3 py-1.5 text-xs hover:bg-neutral-100 dark:border-white/10 dark:hover:bg-white/5"
											>
												Cancel
											</button>
										</div>
									</form>
								{:else}
									<div class="flex items-center gap-3">
										<div class="min-w-0 flex-1">
											<div class="flex items-center gap-2">
												<span class="text-sm font-medium">{u.username}</span>
												{#if u.is_admin}
													<span
														class="rounded-full bg-violet-100 px-2 py-0.5 text-xs font-medium text-violet-700 dark:bg-violet-500/15 dark:text-violet-400"
														>Admin</span
													>
												{/if}
												{#if u.username === auth.user?.username}
													<span
														class="rounded-full bg-neutral-100 px-2 py-0.5 text-xs text-neutral-500 dark:bg-white/10 dark:text-white/40"
														>You</span
													>
												{/if}
												{#if u.is_oidc}
													<span
														class="rounded-full bg-sky-100 px-2 py-0.5 text-xs font-medium text-sky-700 dark:bg-sky-500/15 dark:text-sky-400"
														>SSO</span
													>
												{/if}
											</div>
											{#if u.email}
												<div class="text-xs text-neutral-400 dark:text-white/30">{u.email}</div>
											{/if}
											<div class="text-xs text-neutral-400 dark:text-white/30">
												Created {new Date(u.created_at * 1000).toLocaleDateString()}
											</div>
										</div>
										<div class="flex items-center gap-2">
											{#if resetTarget === u.username}
												<form
													onsubmit={(e) => {
														e.preventDefault();
														handleResetPassword(u.username);
													}}
													class="flex items-center gap-2"
												>
													<input
														type="password"
														bind:value={resetPw}
														placeholder="New password"
														required
														minlength={MIN_PASSWORD_LENGTH}
														autocomplete="new-password"
														class="rounded-lg border border-neutral-200 bg-neutral-50 px-2 py-1 text-xs outline-none focus:border-neutral-400 dark:border-white/10 dark:bg-white/5"
													/>
													{#if resetError}<span class="text-xs text-red-500">{resetError}</span
														>{/if}
													<button
														type="submit"
														disabled={resetSubmitting}
														class="rounded-lg border border-neutral-200 px-2 py-1 text-xs hover:bg-neutral-100 disabled:opacity-50 dark:border-white/10 dark:hover:bg-white/5"
													>
														{resetSubmitting ? '…' : 'Save'}
													</button>
													<button
														type="button"
														onclick={() => {
															resetTarget = null;
															resetPw = '';
															resetError = '';
														}}
														class="rounded-lg border border-neutral-200 px-2 py-1 text-xs hover:bg-neutral-100 dark:border-white/10 dark:hover:bg-white/5"
													>
														Cancel
													</button>
												</form>
											{:else}
												<button
													onclick={() => startEdit(u)}
													class="rounded-lg border border-neutral-200 px-2 py-1.5 text-xs text-neutral-500 hover:bg-neutral-100 dark:border-white/10 dark:text-white/50 dark:hover:bg-white/5"
												>
													Edit
												</button>
												{#if !u.is_oidc}
													<button
														onclick={() => {
															resetTarget = u.username;
															resetPw = '';
															resetError = '';
															editTarget = null;
														}}
														class="rounded-lg border border-neutral-200 px-2 py-1.5 text-xs text-neutral-500 hover:bg-neutral-100 dark:border-white/10 dark:text-white/50 dark:hover:bg-white/5"
													>
														Reset password
													</button>
												{/if}
												{#if u.username !== auth.user?.username}
													<button
														onclick={() => (deleteTarget = u.username)}
														class="rounded-lg border border-red-200 px-2 py-1.5 text-xs text-red-600 hover:bg-red-50 dark:border-red-500/30 dark:text-red-400 dark:hover:bg-red-500/10"
													>
														Delete
													</button>
												{/if}
											{/if}
										</div>
									</div>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		</div>

		<!-- Site Tab (admin) -->
	{:else if tab === 'site'}
		<div class="space-y-6">
			<div
				class="rounded-xl border border-neutral-200 bg-white p-6 dark:border-white/10 dark:bg-neutral-900"
			>
				<div class="mb-1 flex items-center justify-between">
					<h2 class="font-semibold">AI Analysis Prompt</h2>
					<button
						type="button"
						onclick={resetPrompt}
						class="rounded-lg border border-neutral-200 px-2.5 py-1 text-xs text-neutral-500 hover:bg-neutral-100 dark:border-white/10 dark:text-white/50 dark:hover:bg-white/5"
						>Reset to default</button
					>
				</div>
				<p class="mb-4 text-sm text-neutral-500 dark:text-white/40">
					Instructions sent to Ollama before the traffic data. Use <code
						class="rounded bg-neutral-100 px-1 dark:bg-white/10">{'{summary}'}</code
					> where the generated traffic stats should appear — it is required.
				</p>

				{#if promptLoading}
					<p class="text-sm text-neutral-400 dark:text-white/30">Loading…</p>
				{:else}
					<form onsubmit={handleSavePrompt} class="space-y-3">
						<textarea
							bind:value={promptTemplate}
							rows={14}
							spellcheck={false}
							class="w-full rounded-lg border border-neutral-200 bg-neutral-50 px-3 py-2 font-mono text-xs leading-relaxed outline-none focus:border-neutral-400 focus:ring-1 focus:ring-neutral-400 dark:border-white/10 dark:bg-white/5 dark:focus:border-white/30 dark:focus:ring-white/30"
						></textarea>
						{#if promptError}
							<p
								class="rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-600 dark:border-red-500/30 dark:bg-red-500/10 dark:text-red-400"
							>
								{promptError}
							</p>
						{/if}
						{#if promptSuccess}
							<p
								class="rounded-lg border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-700 dark:border-green-500/30 dark:bg-green-500/10 dark:text-green-400"
							>
								{promptSuccess}
							</p>
						{/if}
						<button
							type="submit"
							disabled={promptSubmitting}
							class="rounded-lg bg-neutral-900 px-4 py-2 text-sm font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-50 dark:bg-white dark:text-neutral-900"
						>
							{promptSubmitting ? 'Saving…' : 'Save prompt'}
						</button>
					</form>
				{/if}
			</div>
		</div>
	{/if}
</div>

{#if deleteTarget}
	{@const username = deleteTarget}
	<ConfirmDialog
		open={true}
		onOpenChange={(open) => {
			if (!open) deleteTarget = null;
		}}
		title="Delete user?"
		description={`Delete “${username}”? This action cannot be undone.`}
		confirmLabel="Delete user"
		onconfirm={() => handleDeleteUser(username)}
	/>
{/if}

<style>
	:global(.settings-tabs) {
		width: fit-content;
		max-width: 100%;
		overflow-x: auto;
	}

	:global(.settings-tabs .segmented-item) {
		color: var(--app-muted);
		font-weight: 550;
		letter-spacing: 0.01em;
	}

	:global(.settings-tabs .segmented-item:hover) {
		background: var(--app-surface-muted);
		color: var(--app-fg);
	}

	:global(.settings-tabs .segmented-item[data-state='active']),
	:global(.settings-tabs .segmented-item[aria-selected='true']) {
		background: var(--app-fg);
		color: var(--app-bg);
		font-weight: 620;
		box-shadow: none;
	}

	:global(.settings-tabs .segmented-item[data-state='active']:hover),
	:global(.settings-tabs .segmented-item[aria-selected='true']:hover) {
		background: color-mix(in oklch, var(--app-fg) 88%, var(--app-bg));
		color: var(--app-bg);
	}

	.setting-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
	}

	:global(.theme-grid) {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: 0.5rem;
	}

	:global(.theme-option) {
		display: flex;
		align-items: center;
		gap: 0.55rem;
		min-height: 44px;
		padding: 0.65rem 0.75rem;
		border: 1px solid var(--app-border);
		border-radius: var(--radius-sm);
		background: var(--app-surface);
		font-size: 0.82rem;
		text-align: left;
		transition:
			border-color 140ms ease,
			background 140ms ease;
	}

	:global(.theme-option:hover) {
		border-color: var(--app-border-strong);
		background: var(--app-surface-muted);
	}

	:global(.theme-option[data-state='on']) {
		border-color: var(--app-fg);
		background: var(--app-surface-muted);
	}

	@media (max-width: 720px) {
		.setting-row {
			align-items: flex-start;
			flex-direction: column;
		}

		:global(.theme-grid) {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
</style>
