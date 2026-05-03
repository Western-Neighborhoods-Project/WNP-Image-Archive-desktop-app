<script module lang="ts">
	export type InputSize = "default" | "sm" | "xs";
</script>

<script lang="ts">
	import type { HTMLInputAttributes, HTMLInputTypeAttribute } from "svelte/elements";
	import { cn, type WithElementRef } from "$lib/utils.js";

	type InputType = Exclude<HTMLInputTypeAttribute, "file">;

	type Props = WithElementRef<
		Omit<HTMLInputAttributes, "type" | "size"> &
			({ type: "file"; files?: FileList } | { type?: InputType; files?: undefined })
	> & { size?: InputSize };

	let {
		ref = $bindable(null),
		value = $bindable(),
		type,
		files = $bindable(),
		class: className,
		size = "default",
		"data-slot": dataSlot = "input",
		...restProps
	}: Props = $props();

	// Size variants — match the Select trigger sizes so form rows can
	// mix Inputs and Selects at the same height/font without per-call overrides.
	const sizeClasses: Record<InputSize, string> = {
		default: "h-9 px-3 py-1 text-base md:text-sm",
		sm: "h-8 px-3 py-1 text-sm",
		xs: "h-7 px-2 py-0 text-xs",
	};
</script>

{#if type === "file"}
	<input
		bind:this={ref}
		data-slot={dataSlot}
		data-size={size}
		class={cn(
			"selection:bg-primary dark:bg-input/30 selection:text-primary-foreground border-input ring-offset-background placeholder:text-muted-foreground flex w-full min-w-0 rounded-md border bg-transparent font-medium shadow-xs transition-[color,box-shadow] outline-none disabled:cursor-not-allowed disabled:opacity-50",
			"focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]",
			"aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
			sizeClasses[size],
			className
		)}
		type="file"
		bind:files
		bind:value
		{...restProps}
	/>
{:else}
	<input
		bind:this={ref}
		data-slot={dataSlot}
		data-size={size}
		class={cn(
			"border-input bg-background selection:bg-primary dark:bg-input/30 selection:text-primary-foreground ring-offset-background placeholder:text-muted-foreground flex w-full min-w-0 rounded-md border shadow-xs transition-[color,box-shadow] outline-none disabled:cursor-not-allowed disabled:opacity-50",
			"focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]",
			"aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
			sizeClasses[size],
			className
		)}
		{type}
		bind:value
		{...restProps}
	/>
{/if}
