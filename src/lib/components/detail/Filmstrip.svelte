<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { ImageRecord } from "$lib/commands/images";

  interface Props {
    images: ImageRecord[];
    currentId: number | null;
    onSelect: (img: ImageRecord) => void;
  }

  let { images, currentId, onSelect }: Props = $props();

  function thumbSrc(img: ImageRecord): string | null {
    return img.thumbnail_path ? convertFileSrc(img.thumbnail_path) : null;
  }

  function isMissing(img: ImageRecord): boolean {
    return !img.title && !img.city && !img.date_display;
  }
</script>

<div
  class="h-[108px] flex-shrink-0 px-4 py-3 flex items-center gap-2 min-w-0 border-t relative z-10"
  style="background: #0a0a0b; border-top-color: rgba(255,255,255,0.06);"
>
  <div
    class="flex flex-col gap-[3px] mr-1.5 pr-3 flex-shrink-0"
    style="border-right: 1px solid rgba(255,255,255,0.08);"
  >
    <div
      class="text-[10.5px] font-medium tracking-wide uppercase"
      style="color: rgba(255,255,255,0.5);"
    >
      Recent
    </div>
    <div
      class="text-xs tabular-nums font-mono"
      style="color: rgba(255,255,255,0.85);"
    >
      {images.length} images
    </div>
  </div>
  <div class="flex-1 flex gap-1 overflow-x-auto">
    {#each images as img (img.id)}
      {@const isCurrent = img.id === currentId}
      <button
        type="button"
        onclick={() => onSelect(img)}
        class="relative flex-shrink-0 overflow-hidden rounded-[3px] transition-all"
        style="width: {isCurrent ? 98 : 74}px; height: 82px; box-shadow: {isCurrent
          ? '0 0 0 2px #fff, 0 0 0 4px #0a0a0b'
          : 'none'};"
        aria-label={img.catalog_number}
      >
        {#if thumbSrc(img)}
          <img
            src={thumbSrc(img)}
            alt={img.catalog_number}
            loading="lazy"
            class="h-full w-full object-cover"
          />
        {:else}
          <div
            class="h-full w-full"
            style="background: linear-gradient(135deg, rgba(255,255,255,.05), rgba(0,0,0,.2));"
          ></div>
        {/if}
        {#if isMissing(img)}
          <div
            class="pointer-events-none absolute right-1 top-1 h-1.5 w-1.5 rounded-full bg-warning"
          ></div>
        {/if}
        <div
          class="pointer-events-none absolute bottom-0.5 left-1 font-mono text-[9px] font-medium"
          style="color: rgba(255,255,255,0.75); text-shadow: 0 1px 2px rgba(0,0,0,0.5);"
        >
          {img.catalog_number.split(".")[1] ?? img.catalog_number}
        </div>
      </button>
    {/each}
  </div>
</div>
