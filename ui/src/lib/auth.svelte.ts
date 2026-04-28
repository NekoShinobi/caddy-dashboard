import { hashPassword } from './crypto';

export interface AuthUser {
	username: string;
	email: string;
	is_admin: boolean;
	created_at: number;
	is_oidc: boolean;
}

export interface OidcConfig {
	enabled: boolean;
	provider_name: string;
	logo_url: string | null;
	disable_login: boolean;
}

function createAuth() {
	let user = $state<AuthUser | null>(null);
	let loading = $state(true);
	let needsSetup = $state(false);
	let oidcConfig = $state<OidcConfig | null>(null);

	async function check() {
		loading = true;
		try {
			const [meRes, oidcRes] = await Promise.all([
				fetch('/api/auth/me'),
				fetch('/api/auth/oidc/config'),
			]);
			oidcConfig = await oidcRes.json().catch(() => null);
			if (meRes.ok) {
				user = await meRes.json();
				needsSetup = false;
			} else {
				const data = await meRes.json().catch(() => ({}));
				needsSetup = data.needs_setup ?? false;
				user = null;
			}
		} catch {
			user = null;
		} finally {
			loading = false;
		}
	}

	async function login(username: string, password: string): Promise<string | null> {
		const res = await fetch('/api/auth/login', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ username, password: await hashPassword(password) })
		});
		if (res.ok) {
			user = await res.json();
			needsSetup = false;
			return null;
		}
		const data = await res.json().catch(() => ({}));
		return data.error ?? 'Login failed';
	}

	async function logout() {
		const res = await fetch('/api/auth/logout', { method: 'POST' });
		user = null;
		const data = await res.json().catch(() => ({}));
		if (data.logout_url) {
			window.location.href = data.logout_url;
		}
	}

	async function signup(username: string, email: string, password: string): Promise<string | null> {
		const res = await fetch('/api/auth/signup', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ username, email, password: await hashPassword(password) })
		});
		if (res.ok) {
			user = await res.json();
			needsSetup = false;
			return null;
		}
		const data = await res.json().catch(() => ({}));
		return data.error ?? 'Signup failed';
	}

	async function changePassword(
		currentPassword: string,
		newPassword: string
	): Promise<string | null> {
		const res = await fetch('/api/auth/password', {
			method: 'PUT',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				current_password: await hashPassword(currentPassword),
				new_password: await hashPassword(newPassword)
			})
		});
		if (res.ok) return null;
		const data = await res.json().catch(() => ({}));
		return data.error ?? 'Failed to change password';
	}

	return {
		get user() { return user; },
		get loading() { return loading; },
		get needsSetup() { return needsSetup; },
		get oidcConfig() { return oidcConfig; },
		check,
		login,
		logout,
		signup: (username: string, email: string, password: string) => signup(username, email, password),
		changePassword
	};
}

export const auth = createAuth();
