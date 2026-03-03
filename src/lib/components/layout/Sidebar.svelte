<script lang="ts">
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { currentView, currentCollectionId, currentImageId } from '$lib/stores/navigation';
  import { filters } from '$lib/stores/filters';
  import { formatCount } from '$lib/utils/format';
  import { getScanStats, getCollections, getRecentlyViewed, type Collection, type ImageRecord } from '$lib/commands/images';

  let totalImages = $state(0);
  let archiveCollections = $state<Collection[]>([]);
  let recentlyViewed = $state<ImageRecord[]>([]);

  onMount(async () => {
    try {
      const [stats, allCollections, recent] = await Promise.all([
        getScanStats(),
        getCollections(),
        getRecentlyViewed(),
      ]);
      totalImages = stats.total_images;
      archiveCollections = allCollections.filter((c) => c.source === 'archive');
      recentlyViewed = recent;
    } catch (e) {
      console.error('Sidebar load error:', e);
    }
  });

  // Refresh recently viewed whenever we return to library from detail
  $effect(() => {
    if ($currentView === 'library') {
      getRecentlyViewed()
        .then((r) => { recentlyViewed = r; })
        .catch(() => {});
    }
  });

  function goToLibrary() {
    currentView.set('library');
    currentCollectionId.set(null);
    filters.update((f) => ({ ...f, collectionId: null }));
  }

  function goToCollection(id: number) {
    currentView.set('library');
    currentCollectionId.set(id);
    filters.update((f) => ({ ...f, collectionId: id }));
  }

  function goToImage(img: ImageRecord) {
    currentImageId.set(img.id);
    currentView.set('detail');
  }

  function goToSettings() {
    currentView.set('settings');
  }

  function thumbnailSrc(img: ImageRecord): string | null {
    return img.thumbnail_path ? convertFileSrc(img.thumbnail_path) : null;
  }
</script>

<aside class="flex w-[220px] shrink-0 flex-col border-r border-gray-200 bg-gray-50/80 backdrop-blur-md">
  <!-- App name -->
  <div class="border-b border-gray-200 px-4 py-3">
    <h1 class="text-sm font-semibold text-gray-800">Image Archive Manager</h1>
  </div>

  <nav class="flex flex-1 flex-col overflow-y-auto p-2 gap-0.5">
    <!-- Library -->
    <button
      onclick={goToLibrary}
      class="flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm text-left hover:bg-gray-200 {$currentView === 'library' && $currentCollectionId === null ? 'bg-gray-200 font-medium' : 'text-gray-700'}"
    >
      <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 shrink-0 text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
      </svg>
      Library
    </button>

    <!-- Recently Viewed -->
    {#if recentlyViewed.length > 0}
      <div class="mt-3 mb-1 px-3">
        <span class="text-xs font-medium uppercase tracking-wider text-gray-400">Recently Viewed</span>
      </div>
      {#each recentlyViewed.slice(0, 8) as img}
        <button
          onclick={() => goToImage(img)}
          class="flex w-full items-center gap-2 rounded-md px-2 py-1 text-left hover:bg-gray-200 {$currentImageId === img.id && $currentView === 'detail' ? 'bg-gray-200' : ''}"
          title={img.catalog_number}
        >
          <div class="h-7 w-7 shrink-0 overflow-hidden rounded bg-gray-200">
            {#if thumbnailSrc(img)}
              <img src={thumbnailSrc(img)} alt={img.catalog_number} class="h-full w-full object-cover" />
            {:else}
              <div class="flex h-full w-full items-center justify-center text-gray-300">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
                </svg>
              </div>
            {/if}
          </div>
          <span class="truncate text-xs text-gray-600">{img.catalog_number}</span>
        </button>
      {/each}
    {/if}

    <!-- Archive Collections -->
    {#if archiveCollections.length > 0}
      <div class="mt-3 mb-1 px-3">
        <span class="text-xs font-medium uppercase tracking-wider text-gray-400">Archive Folders</span>
      </div>
      {#each archiveCollections as col}
        <button
          onclick={() => goToCollection(col.id)}
          class="flex w-full items-center justify-between rounded-md px-3 py-1.5 text-sm text-left hover:bg-gray-200 {$currentCollectionId === col.id ? 'bg-gray-200 font-medium' : 'text-gray-600'}"
        >
          <span class="truncate">{col.name}</span>
          <span class="ml-1 shrink-0 text-xs text-gray-400">{formatCount(col.image_count)}</span>
        </button>
      {/each}
    {/if}
  </nav>

  <!-- Footer: image count + settings -->
  <div class="border-t border-gray-200 px-4 py-3 flex items-center justify-between">
    <span class="text-xs text-gray-400">{formatCount(totalImages)} images</span>
    <button
      onclick={goToSettings}
      class="rounded p-1 hover:bg-gray-200 text-gray-400 hover:text-gray-600"
      title="Settings"
    >
      <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    </button>
  </div>
</aside>
