<script lang="ts">
  import { onMount } from "svelte";
  import {
    getRecentlyViewed,
    type ImageRecord,
  } from "$lib/commands/images";
  import { activityVersion } from "$lib/stores/activity";
  import { savedScrollOffset, currentImageId, currentView } from "$lib/stores/navigation";
  import { PageHeader } from "$lib/components/ui/page-header";
  import { StatusBar } from "$lib/components/ui/status-bar";
  import DriveIndicator from "$lib/components/drive/DriveIndicator.svelte";
  import { Kbd } from "$lib/components/ui/kbd";
  import { openShortcutsHelp } from "$lib/stores/shortcutsHelp";
  import GridItem from "./GridItem.svelte";
  import AddToCollectionDialog from "$lib/components/collections/AddToCollectionDialog.svelte";

  let images = $state<ImageRecord[]>([]);
  let loading = $state(true);

  let scrollEl = $state<HTMLElement | undefined>();

  // Add-to-collection dialog
  let showAddToCollection = $state(false);
  let addToCollectionImageId = $state<number | null>(null);

  function openAddToCollection(image: ImageRecord) {
    addToCollectionImageId = image.id;
    showAddToCollection = true;
  }

  function handleImageClick(image: ImageRecord) {
    savedScrollOffset.set(scrollEl?.scrollTop ?? 0);
    currentImageId.set(image.id);
    currentView.set("detail");
  }

  // Re-fetch when the activity version bumps (a metadata edit logs a view).
  // Same store the sidebar ActivityCard subscribes to, so list stays fresh.
  $effect(() => {
    $activityVersion;
    loading = true;
    getRecentlyViewed()
      .then((r) => {
        images = r;
      })
      .catch((e) => console.error("Failed to load recently viewed", e))
      .finally(() => {
        loading = false;
      });
  });

  onMount(() => {
    // Restore scroll position when navigating back from detail view
    const offset = $savedScrollOffset;
    if (offset > 0 && scrollEl) {
      scrollEl.scrollTop = offset;
      savedScrollOffset.set(0);
    }
  });
</script>

<div class="flex flex-1 flex-col min-w-0 min-h-0">
  <PageHeader title="Recently viewed" count={images.length} />

  <main class="flex-1 min-h-0 overflow-hidden">
    <div bind:this={scrollEl} class="h-full w-full overflow-y-auto bg-background">
      {#if loading && images.length === 0}
        <div class="flex h-full items-center justify-center text-muted-foreground">
          <p class="text-sm">Loading…</p>
        </div>
      {:else if images.length === 0}
        <div class="flex h-full items-center justify-center text-muted-foreground">
          <p class="text-sm">
            No recently viewed images yet. Open an image and it'll appear here.
          </p>
        </div>
      {:else}
        <div class="flex flex-wrap gap-1.5 px-6 pt-1.5">
          {#each images as image (image.id)}
            <GridItem
              {image}
              onclick={handleImageClick}
              onaddtocollection={openAddToCollection}
            />
          {/each}
        </div>
      {/if}
    </div>
  </main>

  <StatusBar>
    <span>{images.length} recently viewed</span>
    <span class="text-border">|</span>
    <button
      type="button"
      onclick={openShortcutsHelp}
      class="hover:text-foreground transition-colors"
    >
      Press <Kbd dim>?</Kbd> for shortcuts
    </button>
    <div class="flex-1"></div>
    <DriveIndicator />
  </StatusBar>
</div>

<AddToCollectionDialog
  bind:open={showAddToCollection}
  bind:imageId={addToCollectionImageId}
/>
