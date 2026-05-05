<script lang="ts">
  import type { Snippet } from "svelte";
  import { KbdSeq } from "$lib/components/ui/kbd";

  interface Props {
    icon?: Snippet;
    title: string;
    subtitle?: Snippet | string;
    kbd?: string[];
    /** When set (1-9), shows a dim ⌘N hint at the right so users
     *  discover the ⌘1–⌘9 jump shortcut. */
    numericHint?: number;
    selected?: boolean;
    onSelect: () => void;
    onHover?: () => void;
  }

  let {
    icon,
    title,
    subtitle,
    kbd,
    numericHint,
    selected = false,
    onSelect,
    onHover,
  }: Props = $props();
</script>

<button
  type="button"
  onclick={onSelect}
  onmouseenter={onHover}
  class="w-full flex items-center gap-3 px-[18px] py-2 mx-1.5 rounded-md text-left transition-colors
    {selected ? 'bg-secondary' : 'hover:bg-hover'}"
  style="width: calc(100% - 12px);"
>
  {#if icon}
    <div
      class="w-[26px] h-[26px] flex items-center justify-center text-muted-fg-2 flex-shrink-0"
    >
      {@render icon()}
    </div>
  {/if}
  <div class="flex-1 min-w-0">
    <div
      class="text-[13px] text-foreground font-medium overflow-hidden text-ellipsis whitespace-nowrap"
    >
      {title}
    </div>
    {#if subtitle}
      <div
        class="text-[11.5px] text-muted-foreground overflow-hidden text-ellipsis whitespace-nowrap"
      >
        {#if typeof subtitle === "string"}
          {subtitle}
        {:else}
          {@render subtitle()}
        {/if}
      </div>
    {/if}
  </div>
  {#if numericHint !== undefined}
    <KbdSeq keys={["⌘", String(numericHint)]} dim />
  {/if}
  {#if kbd}
    <KbdSeq keys={kbd} />
  {/if}
</button>
