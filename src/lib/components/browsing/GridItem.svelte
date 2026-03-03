<script lang="ts">
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { getImage, type ImageRecord } from '$lib/commands/images';
  import { thumbnailQueue } from '$lib/utils/thumbnailQueue';
  import { onMount, onDestroy } from 'svelte';

  let {
    image,
    onclick
  }: {
    image: ImageRecord;
    onclick: (image: ImageRecord) => void;
  } = $props();

  // Cache-bust key — incremented when thumbnail regeneration completes
  let cacheBust = $state(Date.now());
  // Overrides image.thumbnail_path after on-demand generation when the path was previously null
  let fetchedThumbnailPath = $state<string | null>(null);

  let thumbnailSrc = $derived.by(() => {
    const path = fetchedThumbnailPath ?? image.thumbnail_path;
    return path ? `${convertFileSrc(path)}?t=${cacheBust}` : null;
  });

  let unsubscribe: (() => void) | null = null;

  onMount(() => {
    // Queue for full-quality generation if:
    // - no thumbnail at all (thumbnail_path is null), OR
    // - has an EXIF thumbnail but not yet full-quality (thumbnail_generated is false)
    if (!image.thumbnail_path || !image.thumbnail_generated) {
      thumbnailQueue.add(image.id);
    }

    // Listen for regeneration completion
    unsubscribe = thumbnailQueue.onRefresh(async (id) => {
      if (id !== image.id) return;
      if (!image.thumbnail_path && !fetchedThumbnailPath) {
        // thumbnail_path was null — re-fetch to get the newly generated path
        try {
          const updated = await getImage(image.id);
          fetchedThumbnailPath = updated.thumbnail_path;
        } catch {}
      }
      cacheBust = Date.now();
    });
  });

  onDestroy(() => {
    unsubscribe?.();
  });
</script>

<button
  onclick={() => onclick(image)}
  class="group flex flex-col items-center gap-1 rounded-lg p-1 hover:bg-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
>
  <div class="relative h-[200px] w-[200px] overflow-hidden rounded bg-gray-100">
    {#if thumbnailSrc}
      <img
        src={thumbnailSrc}
        alt={image.catalog_number}
        loading="lazy"
        class="h-full w-full object-cover transition-opacity duration-200 group-hover:opacity-90"
      />
    {:else}
      <!-- Placeholder for images without thumbnails -->
      <div class="flex h-full w-full items-center justify-center text-gray-300">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
        </svg>
      </div>
    {/if}
  </div>
  <span class="w-[200px] truncate text-center text-xs text-gray-600" title={image.catalog_number}>
    {image.catalog_number}
  </span>
</button>
