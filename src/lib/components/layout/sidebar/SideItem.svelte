<script lang="ts">
  import type { Snippet } from "svelte";
  import { Kbd } from "$lib/components/ui/kbd";

  interface Props {
    icon?: Snippet;
    label: string;
    count?: number;
    badge?: number | string;
    kbd?: string;
    selected?: boolean;
    onclick?: () => void;
  }

  let {
    icon,
    label,
    count,
    badge,
    kbd,
    selected = false,
    onclick,
  }: Props = $props();
</script>

<button
  type="button"
  {onclick}
  class="w-[calc(100%-16px)] flex items-center gap-[10px] h-[30px] pl-3 pr-[10px] mx-2 rounded-md text-[13px] text-left transition-colors
    {selected
    ? 'bg-secondary text-foreground font-medium'
    : 'text-muted-fg-2 hover:bg-hover'}"
>
  {#if icon}
    <span
      class="flex {selected ? 'text-foreground' : 'text-muted-foreground'}"
    >
      {@render icon()}
    </span>
  {/if}
  <span class="flex-1 overflow-hidden text-ellipsis whitespace-nowrap">
    {label}
  </span>
  {#if badge !== undefined}
    <span
      class="text-[10px] font-semibold px-[6px] py-[1px] rounded-[10px] bg-primary text-primary-foreground leading-[1.4]"
    >
      {badge}
    </span>
  {:else if count !== undefined}
    <span class="text-[11px] text-muted-foreground tabular-nums">
      {count.toLocaleString()}
    </span>
  {/if}
  {#if kbd}
    <Kbd dim>{kbd}</Kbd>
  {/if}
</button>
