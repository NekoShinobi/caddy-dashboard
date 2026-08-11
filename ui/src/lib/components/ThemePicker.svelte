<script lang="ts">
	import { DropdownMenu } from 'bits-ui';
	import { colorTheme } from '$lib/color-theme.svelte';
	import { theme } from '$lib/theme.svelte';
	import { THEMES } from '$lib/themes';

	function quad(id: string): [string, string, string, string] {
		const selected = THEMES.find((item) => item.id === id) ?? THEMES[0];
		const colors = selected[theme.dark ? 'dark' : 'light'];
		return [colors.blue, colors.green, colors.red, colors.purple];
	}

	let current = $derived(quad(colorTheme.id));
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger
		class="icon-button"
		aria-label="Choose data color theme"
		title="Data colors"
	>
		<svg aria-hidden="true" width="17" height="17" viewBox="0 0 16 16">
			<rect x="0" y="0" width="7" height="7" fill={current[0]} rx="1.5" />
			<rect x="9" y="0" width="7" height="7" fill={current[1]} rx="1.5" />
			<rect x="0" y="9" width="7" height="7" fill={current[2]} rx="1.5" />
			<rect x="9" y="9" width="7" height="7" fill={current[3]} rx="1.5" />
		</svg>
	</DropdownMenu.Trigger>

	<DropdownMenu.Portal>
		<DropdownMenu.Content class="dropdown-content" sideOffset={8} align="end">
			<DropdownMenu.RadioGroup
				value={colorTheme.id}
				onValueChange={(value) => colorTheme.set(value)}
			>
				<DropdownMenu.GroupHeading class="dropdown-label">Data colors</DropdownMenu.GroupHeading>
				{#each THEMES as item (item.id)}
					{@const colors = quad(item.id)}
					<DropdownMenu.RadioItem value={item.id} class="dropdown-item">
						{#snippet children({ checked })}
							<svg aria-hidden="true" width="15" height="15" viewBox="0 0 16 16" class="shrink-0">
								<rect x="0" y="0" width="7" height="7" fill={colors[0]} rx="1.5" />
								<rect x="9" y="0" width="7" height="7" fill={colors[1]} rx="1.5" />
								<rect x="0" y="9" width="7" height="7" fill={colors[2]} rx="1.5" />
								<rect x="9" y="9" width="7" height="7" fill={colors[3]} rx="1.5" />
							</svg>
							<span>{item.name}</span>
							{#if checked}
								<svg
									aria-hidden="true"
									class="ml-auto h-3.5 w-3.5"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2.4"
								>
									<path d="m5 12 4 4L19 6" />
								</svg>
							{/if}
						{/snippet}
					</DropdownMenu.RadioItem>
				{/each}
			</DropdownMenu.RadioGroup>
		</DropdownMenu.Content>
	</DropdownMenu.Portal>
</DropdownMenu.Root>
