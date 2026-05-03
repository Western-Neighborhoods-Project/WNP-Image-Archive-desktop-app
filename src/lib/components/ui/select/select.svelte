<script module lang="ts">
	export type SelectSize = "default" | "sm" | "xs";
	export const SELECT_SIZE_CONTEXT = "shadcn-select-size";
</script>

<script lang="ts">
	import { Select as SelectPrimitive } from "bits-ui";
	import { setContext } from "svelte";

	let {
		open = $bindable(false),
		value = $bindable(),
		size = "default",
		...restProps
	}: SelectPrimitive.RootProps & { size?: SelectSize } = $props();

	// Stash the size in context so Trigger and Item can pick it up without
	// requiring callers to repeat it. Single source of truth: Select.Root.
	// Pass a getter so consumers always read the current value (Svelte 5
	// avoids capturing reactive props by reference at component instantiation).
	setContext<() => SelectSize>(SELECT_SIZE_CONTEXT, () => size);
</script>

<SelectPrimitive.Root bind:open bind:value={value as never} {...restProps} />
