<script lang="ts">
	import { ToggleGroup } from 'bits-ui';

	interface Option {
		label: string;
		value: string;
	}

	interface Props {
		value: string;
		options: Option[];
		onchange?: (value: string) => void;
		label: string;
		allowEmpty?: boolean;
	}

	let { value, options, onchange, label, allowEmpty = false }: Props = $props();

	function handleValue(next: string) {
		if (!next && !allowEmpty) return;
		onchange?.(next);
	}
</script>

<ToggleGroup.Root
	type="single"
	{value}
	onValueChange={handleValue}
	aria-label={label}
	class="segmented-control"
>
	{#each options as option (option.value)}
		<ToggleGroup.Item value={option.value} class="segmented-item">
			{option.label}
		</ToggleGroup.Item>
	{/each}
</ToggleGroup.Root>
