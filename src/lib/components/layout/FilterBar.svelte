<script lang="ts">
  import { onMount } from 'svelte';
  import { filters } from '$lib/stores/filters';
  import { getFilterOptions, getCollections, type FilterOptions } from '$lib/commands/images';
  import type { Collection } from '$lib/commands/images';

  let filterOptions = $state<FilterOptions | null>(null);
  let archiveCollections = $state<Collection[]>([]);

  // Local copies of filter values, bound to inputs
  let city = $state<string>($filters.city ?? '');
  let photographer = $state<string>($filters.photographer ?? '');
  let yearStart = $state<string>($filters.yearStart?.toString() ?? '');
  let yearEnd = $state<string>($filters.yearEnd?.toString() ?? '');
  let missingMetadata = $state<boolean>($filters.missingMetadata);
  let collectionId = $state<string>($filters.collectionId?.toString() ?? '');

  let hasActiveFilters = $derived(
    !!city || !!photographer || !!yearStart || !!yearEnd || missingMetadata || !!collectionId
  );

  onMount(async () => {
    // Load filter options in parallel (non-fatal)
    try {
      [filterOptions, archiveCollections] = await Promise.all([
        getFilterOptions(),
        getCollections(),
      ]);
    } catch (e) {
      console.warn('Could not load filter options:', e);
    }
  });

  function applyFilters() {
    filters.update((f) => ({
      ...f,
      city: city || null,
      photographer: photographer || null,
      yearStart: yearStart ? parseInt(yearStart, 10) : null,
      yearEnd: yearEnd ? parseInt(yearEnd, 10) : null,
      missingMetadata,
      collectionId: collectionId ? parseInt(collectionId, 10) : null,
    }));
  }

  function clearFilters() {
    city = '';
    photographer = '';
    yearStart = '';
    yearEnd = '';
    missingMetadata = false;
    collectionId = '';
    filters.update((f) => ({
      ...f,
      city: null,
      photographer: null,
      yearStart: null,
      yearEnd: null,
      missingMetadata: false,
      collectionId: null,
    }));
  }

  // Apply filters whenever any value changes
  $effect(() => {
    city; photographer; yearStart; yearEnd; missingMetadata; collectionId;
    applyFilters();
  });

  let archiveOnly = $derived(archiveCollections.filter((c) => c.source === 'archive'));
</script>

<div class="flex shrink-0 flex-wrap items-center gap-2 border-b border-gray-100 bg-gray-50/70 px-4 py-2">

  <!-- City -->
  <div class="flex items-center gap-1.5">
    <label for="filter-city" class="text-xs text-gray-500">City</label>
    {#if filterOptions && filterOptions.cities.length > 0}
      <select
        id="filter-city"
        bind:value={city}
        class="h-7 rounded border border-gray-200 bg-white px-2 text-xs text-gray-700 focus:outline-none focus:ring-1 focus:ring-blue-500"
      >
        <option value="">All</option>
        {#each filterOptions.cities as c}
          <option value={c}>{c}</option>
        {/each}
      </select>
    {:else}
      <input
        id="filter-city"
        type="text"
        bind:value={city}
        placeholder="Any city"
        class="h-7 w-28 rounded border border-gray-200 bg-white px-2 text-xs focus:outline-none focus:ring-1 focus:ring-blue-500"
      />
    {/if}
  </div>

  <div class="h-4 w-px bg-gray-200"></div>

  <!-- Photographer -->
  <div class="flex items-center gap-1.5">
    <label for="filter-photographer" class="text-xs text-gray-500">Photographer</label>
    {#if filterOptions && filterOptions.photographers.length > 0}
      <select
        id="filter-photographer"
        bind:value={photographer}
        class="h-7 rounded border border-gray-200 bg-white px-2 text-xs text-gray-700 focus:outline-none focus:ring-1 focus:ring-blue-500"
      >
        <option value="">All</option>
        {#each filterOptions.photographers as p}
          <option value={p}>{p}</option>
        {/each}
      </select>
    {:else}
      <input
        id="filter-photographer"
        type="text"
        bind:value={photographer}
        placeholder="Any"
        class="h-7 w-28 rounded border border-gray-200 bg-white px-2 text-xs focus:outline-none focus:ring-1 focus:ring-blue-500"
      />
    {/if}
  </div>

  <div class="h-4 w-px bg-gray-200"></div>

  <!-- Year range -->
  <div class="flex items-center gap-1.5">
    <span class="text-xs text-gray-500">Year</span>
    <input
      type="number"
      bind:value={yearStart}
      placeholder={filterOptions?.year_min?.toString() ?? 'From'}
      min="1800"
      max="2100"
      class="h-7 w-20 rounded border border-gray-200 bg-white px-2 text-xs focus:outline-none focus:ring-1 focus:ring-blue-500"
    />
    <span class="text-xs text-gray-400">–</span>
    <input
      type="number"
      bind:value={yearEnd}
      placeholder={filterOptions?.year_max?.toString() ?? 'To'}
      min="1800"
      max="2100"
      class="h-7 w-20 rounded border border-gray-200 bg-white px-2 text-xs focus:outline-none focus:ring-1 focus:ring-blue-500"
    />
  </div>

  <div class="h-4 w-px bg-gray-200"></div>

  <!-- Archive collection -->
  {#if archiveOnly.length > 0}
    <div class="flex items-center gap-1.5">
      <label for="filter-collection" class="text-xs text-gray-500">Collection</label>
      <select
        id="filter-collection"
        bind:value={collectionId}
        class="h-7 rounded border border-gray-200 bg-white px-2 text-xs text-gray-700 focus:outline-none focus:ring-1 focus:ring-blue-500"
      >
        <option value="">All</option>
        {#each archiveOnly as col}
          <option value={col.id.toString()}>{col.name} ({col.image_count})</option>
        {/each}
      </select>
    </div>
    <div class="h-4 w-px bg-gray-200"></div>
  {/if}

  <!-- Missing metadata -->
  <label class="flex cursor-pointer items-center gap-1.5">
    <input
      type="checkbox"
      bind:checked={missingMetadata}
      class="h-3.5 w-3.5 rounded border-gray-300 accent-blue-600"
    />
    <span class="text-xs text-gray-600">Missing metadata</span>
  </label>

  <!-- Clear filters -->
  {#if hasActiveFilters}
    <div class="ml-auto">
      <button
        onclick={clearFilters}
        class="rounded px-2 py-1 text-xs text-blue-600 hover:bg-blue-50 hover:text-blue-700"
      >
        Clear filters
      </button>
    </div>
  {/if}
</div>
