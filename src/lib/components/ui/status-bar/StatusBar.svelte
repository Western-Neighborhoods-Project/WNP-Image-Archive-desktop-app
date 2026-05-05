<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();
</script>

<div
  class="h-7 border-t border-border flex items-center px-4 text-[11px] text-muted-foreground gap-4 bg-sidebar-bg tabular-nums flex-shrink-0"
>
  {@render children()}
</div>
