<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    count?: number | string;
    subtitle?: string;
    right?: Snippet;
  }

  let { title, count, subtitle, right }: Props = $props();

  let countLabel = $derived(
    typeof count === "number" ? `${count.toLocaleString()} items` : count,
  );
</script>

<div
  class="h-14 px-5 flex items-center gap-3 border-b border-border bg-background flex-shrink-0"
>
  <div class="flex items-baseline gap-[10px]">
    <div class="text-base font-semibold text-foreground tracking-[-0.2px]">
      {title}
    </div>
    {#if count !== undefined}
      <div class="text-xs text-muted-foreground tabular-nums">
        {countLabel}
      </div>
    {/if}
    {#if subtitle}
      <div class="text-xs text-muted-foreground">{subtitle}</div>
    {/if}
  </div>
  <div class="flex-1"></div>
  {#if right}
    {@render right()}
  {/if}
</div>
