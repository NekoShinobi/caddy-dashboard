<script lang="ts">
	import { AlertDialog } from 'bits-ui';

	interface Props {
		open: boolean;
		title: string;
		description: string;
		confirmLabel?: string;
		onOpenChange?: (open: boolean) => void;
		onconfirm: () => void | Promise<void>;
	}

	let {
		open,
		title,
		description,
		confirmLabel = 'Confirm',
		onOpenChange,
		onconfirm
	}: Props = $props();
</script>

<AlertDialog.Root {open} {onOpenChange}>
	<AlertDialog.Portal>
		<AlertDialog.Overlay class="dialog-overlay" />
		<AlertDialog.Content class="dialog-content dialog-sm">
			<header class="dialog-header">
				<div>
					<AlertDialog.Title class="dialog-title">{title}</AlertDialog.Title>
					<AlertDialog.Description class="dialog-description">{description}</AlertDialog.Description
					>
				</div>
			</header>
			<footer class="dialog-actions">
				<AlertDialog.Cancel class="button-secondary">Cancel</AlertDialog.Cancel>
				<AlertDialog.Action class="button-danger" onclick={onconfirm}>
					{confirmLabel}
				</AlertDialog.Action>
			</footer>
		</AlertDialog.Content>
	</AlertDialog.Portal>
</AlertDialog.Root>
