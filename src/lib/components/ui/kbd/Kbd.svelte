<script lang="ts">
  import { cn } from "$lib/utils";
  import type { Snippet } from "svelte";

  interface Props {
    dim?: boolean;
    class?: string;
    children: Snippet;
  }

  let { dim = false, class: className = "", children }: Props = $props();
</script>

<span
  class={cn(
    "inline-flex items-center justify-center font-mono text-[10px] font-medium",
    "px-[5px] py-[1px] min-w-[16px] rounded border bg-background",
    "border-border leading-[1.5]",
    dim ? "text-muted-foreground" : "text-muted-fg-2",
    className,
  )}
  style="box-shadow: 0 1px 0 rgba(0,0,0,.03);"
>
  {@render children()}
</span>
