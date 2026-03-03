<script lang="ts">
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { currentView, currentCollectionId, currentImageId } from '$lib/stores/navigation';
  import { filters } from '$lib/stores/filters';
  import { formatCount } from '$lib/utils/format';
  import { getScanStats, getCollections, getRecentlyViewed, type Collection, type ImageRecord } from '$lib/commands/images';
  import { userCollections, refreshUserCollections } from '$lib/stores/collections';
  import { ordersResponse, refreshOrders } from '$lib/stores/requests';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { DropdownMenuPrimitive } from '$lib/components/ui/dropdown-menu';
  import CollectionDialogs from '$lib/components/collections/CollectionDialogs.svelte';

  let totalImages = $state(0);
  let archiveCollections = $state<Collection[]>([]);
  let recentlyViewed = $state<ImageRecord[]>([]);

  // Dialog state for CollectionDialogs
  let showCreate = $state(false);
  let showRename = $state(false);
  let showDelete = $state(false);
  let targetCollection = $state<{ id: number; name: string } | null>(null);

  onMount(async () => {
    try {
      const [stats, allCollections, recent] = await Promise.all([
        getScanStats(),
        getCollections(),
        getRecentlyViewed(),
        refreshUserCollections(),
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

  function goToRequests() {
    currentView.set('requests');
    refreshOrders();
  }

  function goToSettings() {
    currentView.set('settings');
  }

  function thumbnailSrc(img: ImageRecord): string | null {
    return img.thumbnail_path ? convertFileSrc(img.thumbnail_path) : null;
  }

  function openRename(col: { id: number; name: string }) {
    targetCollection = col;
    showRename = true;
  }

  function openDelete(col: { id: number; name: string }) {
    targetCollection = col;
    showDelete = true;
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

    <!-- Requests -->
    <button
      onclick={goToRequests}
      class="flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm text-left hover:bg-gray-200 {$currentView === 'requests' ? 'bg-gray-200 font-medium' : 'text-gray-700'}"
    >
      <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 shrink-0 text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
      </svg>
      <span class="flex-1">Requests</span>
      {#if $ordersResponse && $ordersResponse.meta.fulfillable > 0}
        <span class="rounded-full bg-blue-600 px-1.5 py-0.5 text-[10px] font-semibold leading-none text-white">
          {$ordersResponse.meta.fulfillable}
        </span>
      {/if}
    </button>

    <!-- User Collections -->
    <div class="mt-3 mb-1 px-3 flex items-center justify-between">
      <span class="text-xs font-medium uppercase tracking-wider text-gray-400">Collections</span>
      <button
        onclick={() => (showCreate = true)}
        class="rounded p-0.5 text-gray-400 hover:bg-gray-200 hover:text-gray-600"
        title="New collection"
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 4v16m8-8H4" />
        </svg>
      </button>
    </div>

    {#if $userCollections.length === 0}
      <p class="px-3 text-xs text-gray-400 italic">No collections yet</p>
    {:else}
      {#each $userCollections as col (col.id)}
        <div class="group flex items-center rounded-md hover:bg-gray-200 {$currentCollectionId === col.id ? 'bg-gray-200' : ''}">
          <button
            onclick={() => goToCollection(col.id)}
            class="flex flex-1 items-center justify-between gap-1 rounded-md px-3 py-1.5 text-sm text-left {$currentCollectionId === col.id ? 'font-medium text-gray-800' : 'text-gray-600'}"
          >
            <span class="truncate">{col.name}</span>
            <span class="ml-1 shrink-0 text-xs text-gray-400">{formatCount(col.image_count)}</span>
          </button>
          <!-- "..." dropdown: visible on row hover -->
          <DropdownMenu.Root>
            <DropdownMenuPrimitive.Trigger>
              {#snippet child({ props })}
                <button
                  {...props}
                  class="mr-1 shrink-0 rounded p-0.5 text-gray-400 opacity-0 group-hover:opacity-100 hover:bg-gray-300 hover:text-gray-700"
                  title="Collection options"
                  onclick={(e) => e.stopPropagation()}
                >
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="currentColor" viewBox="0 0 24 24">
                    <circle cx="5" cy="12" r="1.5" />
                    <circle cx="12" cy="12" r="1.5" />
                    <circle cx="19" cy="12" r="1.5" />
                  </svg>
                </button>
              {/snippet}
            </DropdownMenuPrimitive.Trigger>
            <DropdownMenu.Content align="end">
              <DropdownMenu.Item onclick={() => openRename(col)}>Rename</DropdownMenu.Item>
              <DropdownMenu.Separator />
              <DropdownMenu.Item
                class="text-destructive focus:text-destructive"
                onclick={() => openDelete(col)}
              >Delete</DropdownMenu.Item>
            </DropdownMenu.Content>
          </DropdownMenu.Root>
        </div>
      {/each}
    {/if}

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

<!-- Collection CRUD dialogs (mounted once, shared state) -->
<CollectionDialogs
  bind:showCreate
  bind:showRename
  bind:showDelete
  bind:targetCollection
/>
