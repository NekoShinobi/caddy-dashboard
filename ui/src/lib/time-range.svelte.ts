export interface TimeRangeOption {
	label: string;
	seconds: number; // 0 = all time
}

export const TIME_RANGES: TimeRangeOption[] = [
	{ label: '1h',  seconds: 3_600 },
	{ label: '6h',  seconds: 21_600 },
	{ label: '24h', seconds: 86_400 },
	{ label: '7d',  seconds: 604_800 },
	{ label: '30d', seconds: 2_592_000 },
	{ label: 'All', seconds: 0 }
];

function createTimeRange() {
	let seconds = $state(86_400); // default: last 24h

	return {
		get seconds() { return seconds; },
		get label() { return TIME_RANGES.find(r => r.seconds === seconds)?.label ?? 'All'; },
		set(s: number) { seconds = s; },
		// returns the `since` query param value, or null for all time
		sinceParam(): string | null {
			if (seconds === 0) return null;
			return String(Math.floor(Date.now() / 1000) - seconds);
		}
	};
}

export const timeRange = createTimeRange();
