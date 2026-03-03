<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import { queryImages, type ImageRecord, type ImageQuery } from '$lib/commands/images';
  import { filters } from '$lib/stores/filters';
  import GridItem from './GridItem.svelte';

  // ── Props ──────────────────────────────────────────────────────────────────
  let { onImageClick }: { onImageClick: (image: ImageRecord) => void } = $props();

  // ── Layout constants ───────────────────────────────────────────────────────
  const ITEM_SIZE = 208;  // thumbnail + padding (200px thumb + 8px padding)
  const GAP = 8;
  const LABEL_HEIGHT = 28;
  const ROW_HEIGHT = ITEM_SIZE + LABEL_HEIGHT; // ~236px

  const PAGE_SIZE = 100;

  // ── State ──────────────────────────────────────────────────────────────────
  let scrollEl = $state<HTMLElement | undefined>();
  let containerWidth = $state(800);
  let totalCount = $state(0);
  let loading = $state(true);

  let columns = $derived(Math.max(1, Math.floor((containerWidth + GAP) / (ITEM_SIZE + GAP))));
  let totalRows = $derived(Math.ceil(totalCount / columns));

  // Page cache: pageIndex → sorted array of ImageRecord
  const pageCache = new Map<number, ImageRecord[]>();
  // All loaded images, indexed by global index (image position in full dataset)
  let loadedImages = $state<(ImageRecord | null)[]>([]);

  // ── Virtualizer ────────────────────────────────────────────────────────────
  const rowVirtualizer = createVirtualizer({
    count: 0,
    getScrollElement: () => scrollEl ?? null,
    estimateSize: () => ROW_HEIGHT,
    overscan: 4,
  });

  // Update virtualizer when totalRows or scrollEl changes
  $effect(() => {
    $rowVirtualizer.setOptions({
      count: totalRows,
      getScrollElement: () => scrollEl ?? null,
      estimateSize: () => ROW_HEIGHT,
    });
  });

  // ── Data loading ───────────────────────────────────────────────────────────
  async function fetchPage(pageIndex: number): Promise<void> {
    if (pageCache.has(pageIndex)) return;

    const f = $filters;
    const q: ImageQuery = {
      offset: pageIndex * PAGE_SIZE,
      limit: PAGE_SIZE,
      sort_by: f.sortBy,
      sort_order: f.sortOrder,
      city: f.city || null,
      photographer: f.photographer || null,
      collection_id: f.collectionId,
      year_start: f.yearStart,
      year_end: f.yearEnd,
      missing_metadata: f.missingMetadata || null,
      search_query: f.searchQuery || null,
    };

    const result = await queryImages(q);

    // Update totalCount from first fetch
    if (totalCount !== result.total_count) {
      totalCount = result.total_count;
    }

    pageCache.set(pageIndex, result.images);

    // Copy into loadedImages
    const newLoaded = [...loadedImages];
    // Expand array if needed
    while (newLoaded.length < totalCount) newLoaded.push(null);
    result.images.forEach((img, i) => {
      newLoaded[pageIndex * PAGE_SIZE + i] = img;
    });
    loadedImages = newLoaded;
  }

  async function reload() {
    loading = true;
    pageCache.clear();
    loadedImages = [];
    totalCount = 0;
    await fetchPage(0);
    loading = false;
  }

  // Fetch pages for all currently visible rows
  function ensureVisiblePagesLoaded(virtualItems: { index: number }[]) {
    const pagesToFetch = new Set<number>();
    for (const item of virtualItems) {
      const firstGlobal = item.index * columns;
      const lastGlobal = Math.min(firstGlobal + columns - 1, totalCount - 1);
      const firstPage = Math.floor(firstGlobal / PAGE_SIZE);
      const lastPage = Math.floor(lastGlobal / PAGE_SIZE);
      for (let p = firstPage; p <= lastPage; p++) {
        if (!pageCache.has(p)) pagesToFetch.add(p);
      }
    }
    pagesToFetch.forEach((p) => fetchPage(p));
  }

  // Watch visible items and load data as needed
  $effect(() => {
    const items = $rowVirtualizer.getVirtualItems();
    if (items.length > 0) ensureVisiblePagesLoaded(items);
  });

  // Reload when filters change
  $effect(() => {
    const _f = $filters; // subscribe
    if (scrollEl) reload();
  });

  // ── Resize observer ────────────────────────────────────────────────────────
  let resizeObserver: ResizeObserver | undefined;

  onMount(async () => {
    if (scrollEl) {
      containerWidth = scrollEl.clientWidth;
      resizeObserver = new ResizeObserver((entries) => {
        const newWidth = entries[0].contentRect.width;
        if (newWidth !== containerWidth) {
          containerWidth = newWidth;
          pageCache.clear();
          loadedImages = [];
          reload();
        }
      });
      resizeObserver.observe(scrollEl);
    }
    await reload();
  });

  onDestroy(() => resizeObserver?.disconnect());

  // ── Helpers ────────────────────────────────────────────────────────────────
  function getRowImages(rowIndex: number): (ImageRecord | null)[] {
    const result: (ImageRecord | null)[] = [];
    for (let c = 0; c < columns; c++) {
      const globalIndex = rowIndex * columns + c;
      result.push(globalIndex < totalCount ? (loadedImages[globalIndex] ?? null) : null);
    }
    return result;
  }
</script>

<div
  bind:this={scrollEl}
  class="h-full w-full overflow-y-auto"
>
  {#if loading}
    <div class="flex h-full items-center justify-center text-gray-400">
      <p>Loading...</p>
    </div>
  {:else if totalCount === 0}
    <div class="flex h-full items-center justify-center text-gray-400">
      <p>No images found.</p>
    </div>
  {:else}
    <!-- Total height container for virtual scrolling -->
    <div style="height: {$rowVirtualizer.getTotalSize()}px; position: relative;">
      {#each $rowVirtualizer.getVirtualItems() as virtualRow (virtualRow.key)}
        {@const rowImages = getRowImages(virtualRow.index)}
        <div
          style="position: absolute; top: 0; left: 0; width: 100%; transform: translateY({virtualRow.start}px); height: {virtualRow.size}px;"
          class="flex gap-2 px-2 py-1"
        >
          {#each rowImages as image, colIdx (colIdx)}
            {#if image}
              <GridItem {image} onclick={onImageClick} />
            {:else if (virtualRow.index * columns + colIdx) < totalCount}
              <!-- Loading placeholder -->
              <div class="h-[{ITEM_SIZE}px] w-[{ITEM_SIZE}px] animate-pulse rounded bg-gray-200"></div>
            {/if}
          {/each}
        </div>
      {/each}
    </div>
  {/if}
</div>
