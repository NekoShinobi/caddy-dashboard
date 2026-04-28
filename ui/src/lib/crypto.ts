export async function hashPassword(password: string): Promise<string> {
	const buf = await crypto.subtle.digest(
		'SHA-256',
		new TextEncoder().encode(password)
	);
	return Array.from(new Uint8Array(buf))
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('');
}
