<script lang="ts">
	import { ContextMenu as ContextMenuPrimitive } from "bits-ui";
	import { cn, type WithoutChildrenOrChild } from "$lib/utils.js";
	import type { Snippet } from "svelte";

	let {
		ref = $bindable(null),
		class: className,
		inset = false,
		children,
		...restProps
	}: WithoutChildrenOrChild<ContextMenuPrimitive.ItemProps> & {
		inset?: boolean;
		children: Snippet;
	} = $props();
</script>

<ContextMenuPrimitive.Item
	bind:ref
	data-slot="context-menu-item"
	data-inset={inset || undefined}
	class={cn(
		"focus:bg-accent focus:text-accent-foreground relative flex cursor-default select-none items-center gap-2 rounded-sm px-2 py-1.5 text-xs outline-none",
		"data-[disabled]:pointer-events-none data-[disabled]:opacity-50",
		"data-[inset]:pl-8",
		className
	)}
	{...restProps}
>
	{@render children()}
</ContextMenuPrimitive.Item>
