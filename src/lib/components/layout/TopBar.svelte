<script lang="ts">
  import { filters } from '$lib/stores/filters';

  let searchQuery = $state($filters.searchQuery ?? '');
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  // Debounced search — waits 200ms after the user stops typing
  function onSearchInput(e: Event) {
    searchQuery = (e.target as HTMLInputElement).value;
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      filters.update((f) => ({ ...f, searchQuery: searchQuery.trim() || null }));
    }, 200);
  }

  function clearSearch() {
    searchQuery = '';
    filters.update((f) => ({ ...f, searchQuery: null }));
  }

  function updateSort(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    const [col, order] = value.split(':');
    filters.update((f) => ({ ...f, sortBy: col, sortOrder: order as 'asc' | 'desc' }));
  }

  let sortBy = $derived($filters.sortBy);
  let sortOrder = $derived($filters.sortOrder);
</script>

<header class="flex h-12 shrink-0 items-center gap-3 border-b border-gray-200 bg-white px-4">
  <!-- Search -->
  <div class="relative flex-1">
    <svg
      xmlns="http://www.w3.org/2000/svg"
      class="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-gray-400"
      fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"
    >
      <path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-4.35-4.35M17 11A6 6 0 111 11a6 6 0 0116 0z" />
    </svg>
    <input
      type="search"
      value={searchQuery}
      oninput={onSearchInput}
      placeholder="Search catalog numbers, descriptions, cities, keywords…"
      class="h-8 w-full rounded-md border border-gray-200 bg-gray-50 pl-8 pr-8 text-sm placeholder-gray-400 focus:border-blue-400 focus:bg-white focus:outline-none focus:ring-1 focus:ring-blue-400"
    />
    {#if searchQuery}
      <button
        onclick={clearSearch}
        class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600"
        aria-label="Clear search"
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    {/if}
  </div>

  <!-- Sort -->
  <div class="flex shrink-0 items-center gap-2">
    <label for="sort-select" class="text-xs text-gray-500">Sort:</label>
    <select
      id="sort-select"
      value="{sortBy}:{sortOrder}"
      onchange={updateSort}
      class="rounded border border-gray-200 bg-white px-2 py-1 text-xs text-gray-700 focus:outline-none focus:ring-1 focus:ring-blue-500"
    >
      <option value="catalog_number:asc">Catalog # (A–Z)</option>
      <option value="catalog_number:desc">Catalog # (Z–A)</option>
      <option value="date_start:asc">Date (oldest first)</option>
      <option value="date_start:desc">Date (newest first)</option>
      <option value="updated_at:desc">Recently updated</option>
      <option value="created_at:desc">Recently added</option>
    </select>
  </div>
</header>
