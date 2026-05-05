<script lang="ts">
	import { ContextMenu as ContextMenuPrimitive } from "bits-ui";

	let { ...restProps }: ContextMenuPrimitive.RootProps = $props();
</script>

<ContextMenuPrimitive.Root {...restProps} />
