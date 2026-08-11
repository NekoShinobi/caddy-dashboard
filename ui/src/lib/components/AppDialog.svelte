<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Dialog } from 'bits-ui';

	type DialogSize = 'sm' | 'md' | 'lg';

	interface Props {
		open: boolean;
		onOpenChange?: (open: boolean) => void;
		title: string;
		description?: string;
		size?: DialogSize;
		children: Snippet;
		actions?: Snippet;
	}

	let { open, onOpenChange, title, description, size = 'md', children, actions }: Props = $props();
</script>

<Dialog.Root {open} {onOpenChange}>
	<Dialog.Portal>
		<Dialog.Overlay class="dialog-overlay" />
		<Dialog.Content class="dialog-content dialog-{size}">
			<header class="dialog-header">
				<div>
					<Dialog.Title class="dialog-title">{title}</Dialog.Title>
					{#if description}
						<Dialog.Description class="dialog-description">{description}</Dialog.Description>
					{/if}
				</div>
				<Dialog.Close class="icon-button" aria-label="Close dialog">
					<svg
						aria-hidden="true"
						class="h-4 w-4"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2.2"
					>
						<path d="M18 6 6 18M6 6l12 12" />
					</svg>
				</Dialog.Close>
			</header>
			<div class="dialog-body">
				{@render children()}
			</div>
			{#if actions}
				<footer class="dialog-actions">
					{@render actions()}
				</footer>
			{/if}
		</Dialog.Content>
	</Dialog.Portal>
</Dialog.Root>
