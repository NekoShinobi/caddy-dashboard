export const MIN_PASSWORD_LENGTH = 8;

// The server cannot enforce this. `hashPassword` sends a 64-character SHA-256
// digest, so the `len() < 8` checks in auth.rs and admin.rs always see 64 and
// always pass. The real length is only observable here, which means every call
// site has to validate before hashing — including the ones that POST directly
// instead of going through `auth`.
export function validatePassword(password: string): string | null {
	return password.length < MIN_PASSWORD_LENGTH
		? `Password must be at least ${MIN_PASSWORD_LENGTH} characters`
		: null;
}

export async function hashPassword(password: string): Promise<string> {
	const buf = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(password));
	return Array.from(new Uint8Array(buf))
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('');
}
