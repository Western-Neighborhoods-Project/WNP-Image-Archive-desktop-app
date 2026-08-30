<script lang="ts">
  import { onMount, onDestroy, untrack } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import { queryImages, type ImageRecord, type ImageQuery } from '$lib/commands/images';
  import { removeFromCollection } from '$lib/commands/collections';
  import { effectiveFilters } from '$lib/stores/filters';
  import {
    currentCollectionId,
    savedScrollOffset,
  } from '$lib/stores/navigation';
  import {
    userCollections,
    refreshUserCollections,
  } from '$lib/stores/collections';
  import {
    selectedImageIds,
    setSelection,
    clearSelection,
  } from '$lib/stores/selection';
  import GridItem from './GridItem.svelte';
  import AddToCollectionDialog from '$lib/components/collections/AddToCollectionDialog.svelte';
  import AddMultipleToCollectionDialog from '$lib/components/collections/AddMultipleToCollectionDialog.svelte';
  import ShareDialog from '$lib/components/sharing/ShareDialog.svelte';

  // ── Props ──────────────────────────────────────────────────────────────────
  let {
    onImageClick,
    onCountChange,
  }: {
    onImageClick: (image: ImageRecord, scrollOffset: number) => void;
    onCountChange?: (n: number) => void;
  } = $props();

  // ── Layout constants ───────────────────────────────────────────────────────
  // Design uses 176px thumbnails with 6px gap and no separate label row.
  const ITEM_SIZE = 176;
  const GAP = 6;
  const ROW_HEIGHT = ITEM_SIZE + GAP;
  const PAD_LEFT = 24;   // matches px-6 on the row container
  const PAD_TOP = 6;     // matches pt-1.5

  const PAGE_SIZE = 100;

  // ── State ──────────────────────────────────────────────────────────────────
  let showAddToCollection = $state(false);
  let addToCollectionImageId = $state<number | null>(null);

  let showAddMultipleToCollection = $state(false);
  let addMultipleIds = $state<number[]>([]);

  // Share dialog (Plan 5) — single instance shared across the grid;
  // GridItem's context menu calls openShareDialog with the right-clicked
  // image regardless of the current selection.
  let showShareDialog = $state(false);
  let shareImage = $state<ImageRecord | null>(null);

  function openShareDialog(image: ImageRecord) {
    shareImage = image;
    showShareDialog = true;
  }

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

  // Ordered ids for Shift-click range selection — recomputed when
  // loadedImages changes.
  let orderedIds = $derived(
    loadedImages
      .filter((img): img is ImageRecord => img !== null)
      .map((img) => img.id),
  );

  // ── Virtualizer ────────────────────────────────────────────────────────────
  const rowVirtualizer = createVirtualizer({
    count: 0,
    getScrollElement: () => scrollEl ?? null,
    estimateSize: () => ROW_HEIGHT,
    overscan: 4,
  });

  // Pushes option changes into the virtualizer. The store must be read via
  // untrack(): setOptions notifies the store on newer svelte/svelte-virtual
  // versions, so a tracked read makes this effect its own dependency
  // (effect_update_depth_exceeded, which kills the whole reactive tree).
  // Only the real inputs (totalRows, scrollEl) are tracked, and unchanged
  // options are skipped so no notification path can re-enter setOptions.
  let appliedCount = -1;
  let appliedScrollEl: HTMLElement | null = null;

  $effect(() => {
    const count = totalRows;
    const el = scrollEl ?? null;
    if (count === appliedCount && el === appliedScrollEl) return;
    appliedCount = count;
    appliedScrollEl = el;
    untrack(() => $rowVirtualizer).setOptions({
      count,
      getScrollElement: () => el,
      estimateSize: () => ROW_HEIGHT,
      // Re-stated from createVirtualizer: the pinned adapter merges existing
      // options into setOptions calls, but that merge isn't contractual.
      overscan: 4,
    });
  });

  // ── Data loading ───────────────────────────────────────────────────────────
  // Bumped on every reload. A page fetch captures the epoch before awaiting and
  // discards its result if a reload happened meanwhile, so a slow response for
  // the previous filter set can't splice stale images (and a wrong total) into
  // the freshly-reloaded grid.
  let reloadEpoch = 0;

  async function fetchPage(pageIndex: number): Promise<void> {
    if (pageCache.has(pageIndex)) return;

    const epoch = reloadEpoch;
    const f = $effectiveFilters;
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
      source_directory_id: f.sourceDirectoryId,
      relative_dir: f.relativeDir,
    };

    const result = await queryImages(q);

    // A reload started while this was in flight — its results belong to the
    // old filter/sort set; drop them.
    if (epoch !== reloadEpoch) return;

    if (totalCount !== result.total_count) {
      totalCount = result.total_count;
    }

    pageCache.set(pageIndex, result.images);

    const newLoaded = [...loadedImages];
    while (newLoaded.length < totalCount) newLoaded.push(null);
    result.images.forEach((img, i) => {
      newLoaded[pageIndex * PAGE_SIZE + i] = img;
    });
    loadedImages = newLoaded;
  }

  async function reload() {
    const epoch = ++reloadEpoch;
    loading = true;
    pageCache.clear();
    loadedImages = [];
    totalCount = 0;
    // Filter / sort changes invalidate the current selection — old ids may
    // not be visible under the new filter.
    clearSelection();
    await fetchPage(0);
    // A newer reload superseded this one while page 0 was loading.
    if (epoch !== reloadEpoch) return;
    onCountChange?.(totalCount);
    loading = false;
  }

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

  $effect(() => {
    const items = $rowVirtualizer.getVirtualItems();
    if (items.length > 0) ensureVisiblePagesLoaded(items);
  });

  $effect(() => {
    const _f = $effectiveFilters;
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
          // Pages are keyed by DB offset, independent of column count, and
          // getRowImages maps global index → row from the already-loaded
          // array. So a width change only needs to update containerWidth; the
          // derived `columns`/`totalRows` and the virtualizer re-map existing
          // images into new rows. Clearing the cache and reloading here re-ran
          // a COUNT + SELECT over the whole filtered set on every resize tick
          // (e.g. dragging the window edge or toggling the sidebar).
          containerWidth = newWidth;
        }
      });
      resizeObserver.observe(scrollEl);
    }
    await reload();
    const offset = $savedScrollOffset;
    if (offset > 0 && scrollEl) {
      scrollEl.scrollTop = offset;
      savedScrollOffset.set(0);
    }
  });

  onDestroy(() => resizeObserver?.disconnect());

  // ── Plan 12 watcher event ──────────────────────────────────────────────────
  // The Sidebar handles re-scanning when library:filesystem-changed fires;
  // we just need to bust our page cache + reload so newly-added images
  // appear without a manual refresh. Sidebar's debounced handler runs
  // first; an extra reload here once the rescan is in flight is fine.
  let unlistenFsChange: UnlistenFn | null = null;
  onMount(async () => {
    try {
      unlistenFsChange = await listen('library:filesystem-changed', async () => {
        // Tiny delay so the sidebar's scan + thumbnail batch completes
        // before we re-query. The user sees thumbnails on the first
        // refreshed render rather than a flash of empty placeholders.
        setTimeout(() => {
          pageCache.clear();
          loadedImages = [];
          reload();
        }, 500);
      });
    } catch (e) {
      console.error('Grid failed to subscribe to filesystem events', e);
    }
  });
  onDestroy(() => unlistenFsChange?.());

  // ── Click-out to clear selection on this view ──────────────────────────────
  // Also: clear on view unmount.
  onDestroy(() => clearSelection());

  // ── Add-to-collection handlers ─────────────────────────────────────────────
  function openAddToCollection(image: ImageRecord) {
    addToCollectionImageId = image.id;
    showAddToCollection = true;
  }

  function openAddMultipleToCollection() {
    addMultipleIds = [...$selectedImageIds];
    showAddMultipleToCollection = true;
  }

  // Remove-from-collection — only enabled when the current view is a
  // user-created collection (archive collections are folder-derived and
  // would just re-import next scan).
  let activeUserCollection = $derived.by(() => {
    if ($currentCollectionId === null) return null;
    return $userCollections.find((c) => c.id === $currentCollectionId) ?? null;
  });

  async function removeOneFromCurrentCollection(image: ImageRecord) {
    if (!activeUserCollection) return;
    try {
      await removeFromCollection(activeUserCollection.id, [image.id]);
      await refreshUserCollections();
      pageCache.clear();
      loadedImages = [];
      reload();
    } catch (e) {
      console.error('Remove from collection failed', e);
    }
  }

  async function removeSelectionFromCurrentCollection() {
    if (!activeUserCollection) return;
    const ids = [...$selectedImageIds];
    if (ids.length === 0) return;
    try {
      await removeFromCollection(activeUserCollection.id, ids);
      await refreshUserCollections();
      clearSelection();
      pageCache.clear();
      loadedImages = [];
      reload();
    } catch (e) {
      console.error('Remove from collection failed', e);
    }
  }

  // ── Helpers ────────────────────────────────────────────────────────────────
  function getRowImages(rowIndex: number): (ImageRecord | null)[] {
    const result: (ImageRecord | null)[] = [];
    for (let c = 0; c < columns; c++) {
      const globalIndex = rowIndex * columns + c;
      result.push(globalIndex < totalCount ? (loadedImages[globalIndex] ?? null) : null);
    }
    return result;
  }

  // ── Marquee selection ──────────────────────────────────────────────────────
  // Drag from empty space (between/around items) draws a selection
  // rectangle in content coordinates. Items whose square overlaps the
  // rect get added to the selection. Shift-drag extends the existing
  // selection instead of replacing it.
  let marqueeActive = $state(false);
  let startCX = $state(0);
  let startCY = $state(0);
  let curCX = $state(0);
  let curCY = $state(0);
  let savedSelection: Set<number> = new Set();
  let pointerIdCaptured: number | null = null;

  function pointerToContent(e: PointerEvent): { x: number; y: number } {
    const rect = scrollEl!.getBoundingClientRect();
    return {
      x: e.clientX - rect.left + scrollEl!.scrollLeft,
      y: e.clientY - rect.top + scrollEl!.scrollTop,
    };
  }

  function handlePointerDown(e: PointerEvent) {
    if (e.button !== 0 || !scrollEl) return;
    // If the pointerdown is on a grid item (or its descendants), let
    // click / drag-start handle it. Marquee only kicks in for empty
    // areas of the grid.
    const target = e.target as HTMLElement;
    if (target.closest('[data-grid-item]')) return;

    const { x, y } = pointerToContent(e);
    startCX = x;
    startCY = y;
    curCX = x;
    curCY = y;
    marqueeActive = true;

    if (e.shiftKey) {
      savedSelection = new Set($selectedImageIds);
    } else {
      savedSelection = new Set();
      clearSelection();
    }

    pointerIdCaptured = e.pointerId;
    scrollEl.setPointerCapture(e.pointerId);
  }

  function handlePointerMove(e: PointerEvent) {
    if (!marqueeActive || !scrollEl) return;
    const { x, y } = pointerToContent(e);
    curCX = x;
    curCY = y;
    recomputeMarqueeSelection();
  }

  function handlePointerUp(e: PointerEvent) {
    if (!marqueeActive) return;
    marqueeActive = false;
    if (scrollEl && pointerIdCaptured !== null) {
      scrollEl.releasePointerCapture(pointerIdCaptured);
    }
    pointerIdCaptured = null;
  }

  function handlePointerCancel() {
    marqueeActive = false;
    pointerIdCaptured = null;
  }

  function recomputeMarqueeSelection() {
    const minX = Math.min(startCX, curCX);
    const maxX = Math.max(startCX, curCX);
    const minY = Math.min(startCY, curCY);
    const maxY = Math.max(startCY, curCY);

    const minRow = Math.max(0, Math.floor((minY - PAD_TOP) / ROW_HEIGHT));
    const maxRow = Math.min(
      totalRows - 1,
      Math.floor((maxY - PAD_TOP) / ROW_HEIGHT),
    );

    const ids = new Set(savedSelection);
    for (let row = minRow; row <= maxRow; row++) {
      for (let col = 0; col < columns; col++) {
        const itemX = PAD_LEFT + col * (ITEM_SIZE + GAP);
        const itemY = PAD_TOP + row * ROW_HEIGHT;
        if (
          itemX < maxX &&
          itemX + ITEM_SIZE > minX &&
          itemY < maxY &&
          itemY + ITEM_SIZE > minY
        ) {
          const idx = row * columns + col;
          if (idx < totalCount) {
            const img = loadedImages[idx];
            if (img) ids.add(img.id);
          }
        }
      }
    }
    setSelection(ids);
  }

  // Marquee rect in content coords (for rendering inside the virtualizer's
  // positioned container).
  let marqueeRect = $derived({
    left: Math.min(startCX, curCX),
    top: Math.min(startCY, curCY),
    width: Math.abs(curCX - startCX),
    height: Math.abs(curCY - startCY),
  });
</script>

<div
  bind:this={scrollEl}
  role="presentation"
  class="h-full w-full overflow-y-auto bg-background select-none"
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerCancel}
>
  {#if loading}
    <div class="flex h-full items-center justify-center text-muted-foreground">
      <p class="text-sm">Loading…</p>
    </div>
  {:else if totalCount === 0}
    <div class="flex h-full items-center justify-center text-muted-foreground">
      <p class="text-sm">No images found.</p>
    </div>
  {:else}
    <div
      style="height: {$rowVirtualizer.getTotalSize()}px; position: relative;"
    >
      {#each $rowVirtualizer.getVirtualItems() as virtualRow (virtualRow.key)}
        {@const rowImages = getRowImages(virtualRow.index)}
        <div
          style="position: absolute; top: 0; left: 0; width: 100%; transform: translateY({virtualRow.start}px); height: {virtualRow.size}px;"
          class="flex gap-1.5 px-6 pt-1.5"
        >
          {#each rowImages as image, colIdx (colIdx)}
            {#if image}
              <GridItem
                {image}
                {orderedIds}
                onclick={(img) =>
                  onImageClick(img, scrollEl?.scrollTop ?? 0)}
                onaddtocollection={openAddToCollection}
                onaddmultipletocollection={openAddMultipleToCollection}
                onshare={openShareDialog}
                onremovefromcollection={activeUserCollection
                  ? removeOneFromCurrentCollection
                  : undefined}
                onremovemultiplefromcollection={activeUserCollection
                  ? removeSelectionFromCurrentCollection
                  : undefined}
                currentCollectionName={activeUserCollection?.name ?? null}
              />
            {:else if virtualRow.index * columns + colIdx < totalCount}
              <div
                class="h-[176px] w-[176px] animate-pulse rounded bg-secondary"
              ></div>
            {/if}
          {/each}
        </div>
      {/each}

      <!-- Marquee rectangle. Sits inside the virtualizer's positioned
           container so its coordinates are in the same content space. -->
      {#if marqueeActive}
        <div
          class="pointer-events-none absolute rounded-sm border border-primary/60 bg-primary/10"
          style="left: {marqueeRect.left}px; top: {marqueeRect.top}px; width: {marqueeRect.width}px; height: {marqueeRect.height}px;"
        ></div>
      {/if}
    </div>
  {/if}
</div>

<AddToCollectionDialog
  bind:open={showAddToCollection}
  bind:imageId={addToCollectionImageId}
/>
<AddMultipleToCollectionDialog
  bind:open={showAddMultipleToCollection}
  bind:imageIds={addMultipleIds}
/>
{#if shareImage}
  <ShareDialog bind:open={showShareDialog} image={shareImage} />
{/if}
