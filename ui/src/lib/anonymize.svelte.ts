// FNV-32a — deterministic hash so the same real value always maps to the same fake value
function fnv32(s: string): number {
	let h = 0x811c9dc5;
	for (let i = 0; i < s.length; i++) {
		h ^= s.charCodeAt(i);
		h = Math.imul(h, 0x01000193) >>> 0;
	}
	return h;
}

export function anonIP(ip: string): string {
	const h = fnv32(ip);
	return `${h & 0xff}.${(h >> 8) & 0xff}.${(h >> 16) & 0xff}.${(h >> 24) & 0xff}`;
}

export function anonHost(host: string): string {
	return `host-${(fnv32(host) % 9999) + 1}.example.com`;
}

// path keys are "host/path" — anonymize the host prefix, keep the path
export function anonPathKey(key: string): string {
	const i = key.indexOf('/');
	if (i === -1) return anonHost(key);
	return anonHost(key.slice(0, i)) + key.slice(i);
}

function createAnonymize() {
	let on = $state(false);
	return {
		get on() {
			return on;
		},
		toggle() {
			on = !on;
		}
	};
}

export const anonymize = createAnonymize();
