<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { getImage, type ImageRecord } from "$lib/commands/images";
  import { thumbnailQueue } from "$lib/utils/thumbnailQueue";
  import { onMount, onDestroy } from "svelte";
  import * as ContextMenu from "$lib/components/ui/context-menu";
  import { ContextMenuPrimitive } from "$lib/components/ui/context-menu";
  import {
    selectedImageIds,
    selectOnly,
    toggleSelected,
    selectRange,
  } from "$lib/stores/selection";

  let {
    image,
    orderedIds = [],
    onclick,
    onaddtocollection,
    onaddmultipletocollection,
    onshare,
    onremovefromcollection,
    onremovemultiplefromcollection,
    currentCollectionName,
  }: {
    image: ImageRecord;
    /** All currently loaded image ids in display order — used for
     *  Shift-click range selection. Optional; views without a multi-
     *  selection notion (e.g. RecentlyViewed) can omit it. */
    orderedIds?: number[];
    onclick: (image: ImageRecord) => void;
    onaddtocollection?: (image: ImageRecord) => void;
    /** Fired by the context menu when this item is part of a multi-
     *  image selection. The handler should open the multi-add dialog
     *  using the current selectedImageIds store. */
    onaddmultipletocollection?: () => void;
    /** Fired when the user picks "Share this image" from the context
     *  menu. Always operates on the right-clicked image (not the
     *  whole selection), so it works the same in single + multi
     *  modes. */
    onshare?: (image: ImageRecord) => void;
    /** Provided by Grid only when the current view is a user-created
     *  collection. When present, the context menu offers a "Remove
     *  from <name>" entry. Archive collections don't qualify because
     *  they're folder-derived and would just re-import. */
    onremovefromcollection?: (image: ImageRecord) => void;
    onremovemultiplefromcollection?: () => void;
    /** Name of the user collection currently being viewed. Used in
     *  the menu label so the user knows what they're removing from. */
    currentCollectionName?: string | null;
  } = $props();

  // Cache-bust key — incremented when thumbnail regeneration completes
  let cacheBust = $state(Date.now());
  let fetchedThumbnailPath = $state<string | null>(null);

  let thumbnailSrc = $derived.by(() => {
    const path = fetchedThumbnailPath ?? image.thumbnail_path;
    return path ? `${convertFileSrc(path)}?t=${cacheBust}` : null;
  });

  // "Missing metadata" mirrors backend criterion in queries.rs:
  // image is missing if title, city, AND date_display are all empty.
  let missing = $derived(
    !image.title && !image.city && !image.date_display,
  );

  let selected = $derived($selectedImageIds.has(image.id));
  let multiSelected = $derived(selected && $selectedImageIds.size > 1);

  let unsubscribe: (() => void) | null = null;

  onMount(() => {
    if (!image.thumbnail_path || !image.thumbnail_generated) {
      thumbnailQueue.add(image.id);
    }
    // Refresh when the backend signals this image's thumbnail is ready
    // (per-image, so it pops in as soon as it's committed — from either the
    // on-demand path or the background worker).
    unsubscribe = thumbnailQueue.onReady(image.id, async () => {
      if (!image.thumbnail_path && !fetchedThumbnailPath) {
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

  // ── Click / selection handling ─────────────────────────────────────────
  // Plain click  → open detail
  // Cmd-click    → toggle this image in/out of the selection (macOS
  //                multi-select modifier)
  // Shift-click  → range select from last clicked to here
  // Ctrl-click   → no-op for click; bits-ui's oncontextmenu handles it
  //                (macOS treats Ctrl+click as a right-click; the click
  //                event still fires alongside, so we explicitly skip
  //                any click action to avoid toggling selection or
  //                opening the detail view alongside the menu)
  function handleClick(e: MouseEvent) {
    if (e.button !== 0) return;
    if (e.ctrlKey) return;
    if (e.metaKey) {
      e.preventDefault();
      toggleSelected(image.id);
      return;
    }
    if (e.shiftKey) {
      e.preventDefault();
      const sel = $selectedImageIds;
      const anchor = sel.size > 0 ? [...sel][sel.size - 1] : image.id;
      selectRange(orderedIds, anchor, image.id);
      return;
    }
    onclick(image);
  }

  // ── Drag start ─────────────────────────────────────────────────────────
  // If the user starts dragging an image that isn't selected yet, replace
  // the selection with just this image first — same pattern as Finder /
  // Lightroom. Then the dragged payload is whatever is currently selected.
  function handleDragStart(e: DragEvent) {
    if (!$selectedImageIds.has(image.id)) {
      selectOnly(image.id);
    }
    const ids = [...$selectedImageIds];
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "copy";
      // Stash the ids on the dataTransfer so drop targets can read
      // them without coupling to the selection store.
      e.dataTransfer.setData("application/x-wnp-images", JSON.stringify(ids));
    }
  }
</script>

<ContextMenu.Root>
  <!-- bits-ui spreads onpointerdown / oncontextmenu / etc. onto whatever
       element receives the trigger props. Putting those on the same
       <button> that we want to be a drag source caused two regressions:
       (1) my oncontextmenu attribute overrode bits-ui's (since attr
       order wins over spread) so the browser's default menu appeared,
       (2) WebKit's drag-start detection didn't fire reliably on a
       button laden with bits-ui handlers. Letting the Trigger render
       its own wrapper div keeps the menu logic and the drag source
       cleanly separated. -->
  <ContextMenuPrimitive.Trigger class="h-[176px] w-[176px]">
    <button
      data-grid-item
      data-image-id={image.id}
      type="button"
      draggable="true"
      ondragstart={handleDragStart}
      onclick={handleClick}
      class="group relative block h-full w-full overflow-hidden rounded bg-secondary cursor-pointer transition-shadow ring-2 focus:outline-none {selected
        ? 'ring-primary'
        : 'ring-transparent hover:ring-border'}"
    >
      {#if thumbnailSrc}
        <img
          src={thumbnailSrc}
          alt={image.catalog_number}
          loading="lazy"
          draggable="false"
          class="h-full w-full object-contain transition-opacity duration-200 group-hover:opacity-95"
        />
      {:else}
        <div
          class="flex h-full w-full items-center justify-center text-border"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            class="h-12 w-12"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="1"
              d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
            />
          </svg>
        </div>
      {/if}

      <!-- Subtle inner gradient for catalog readability -->
      <div
        class="pointer-events-none absolute inset-0"
        style="background: linear-gradient(135deg, rgba(255,255,255,.06), rgba(0,0,0,.18));"
      ></div>

      <!-- Catalog number overlay (bottom-left) -->
      <div
        class="pointer-events-none absolute bottom-1.5 left-1.5 font-mono text-[10px] font-medium text-white/85"
        style="text-shadow: 0 1px 2px rgba(0,0,0,.5);"
      >
        {image.catalog_number}
      </div>

      <!-- Missing-metadata amber dot (top-right) -->
      {#if missing}
        <div
          class="pointer-events-none absolute right-1.5 top-1.5 h-1.5 w-1.5 rounded-full bg-warning"
        ></div>
      {/if}
    </button>
  </ContextMenuPrimitive.Trigger>
  <ContextMenu.Content>
    {#if multiSelected}
      <ContextMenu.Item onclick={() => onaddmultipletocollection?.()}>
        Add {$selectedImageIds.size} images to Collection…
      </ContextMenu.Item>
    {:else}
      <ContextMenu.Item
        onclick={() => {
          // Right-click on a non-selected item should act on just that
          // item, not the existing selection — fold the selection in
          // before triggering the action.
          if (!$selectedImageIds.has(image.id)) selectOnly(image.id);
          onaddtocollection?.(image);
        }}
      >
        Add to Collection…
      </ContextMenu.Item>
    {/if}
    {#if onshare}
      <ContextMenu.Separator />
      <ContextMenu.Item onclick={() => onshare?.(image)}>
        Share this image…
      </ContextMenu.Item>
    {/if}
    {#if onremovefromcollection || onremovemultiplefromcollection}
      <ContextMenu.Separator />
      {#if multiSelected && onremovemultiplefromcollection}
        <ContextMenu.Item
          class="text-destructive focus:text-destructive"
          onclick={() => onremovemultiplefromcollection?.()}
        >
          Remove {$selectedImageIds.size} from{currentCollectionName
            ? ` "${currentCollectionName}"`
            : " this collection"}
        </ContextMenu.Item>
      {:else if onremovefromcollection}
        <ContextMenu.Item
          class="text-destructive focus:text-destructive"
          onclick={() => {
            if (!$selectedImageIds.has(image.id)) selectOnly(image.id);
            onremovefromcollection?.(image);
          }}
        >
          Remove from{currentCollectionName
            ? ` "${currentCollectionName}"`
            : " this collection"}
        </ContextMenu.Item>
      {/if}
    {/if}
  </ContextMenu.Content>
</ContextMenu.Root>
