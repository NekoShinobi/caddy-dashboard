<script lang="ts">
	import { timeRange, TIME_RANGES } from '$lib/time-range.svelte';
	import SegmentedControl from './SegmentedControl.svelte';

	let { onchange }: { onchange?: () => void } = $props();

	const options = TIME_RANGES.map((option) => ({
		label: option.label,
		value: String(option.seconds)
	}));

	function changeRange(value: string) {
		timeRange.set(Number(value));
		onchange?.();
	}
</script>

<SegmentedControl
	value={String(timeRange.seconds)}
	{options}
	onchange={changeRange}
	label="Time range"
/>
