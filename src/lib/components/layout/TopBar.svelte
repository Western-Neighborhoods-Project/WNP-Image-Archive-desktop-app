<script lang="ts">
  import { filters } from '$lib/stores/filters';

  let sortBy = $derived($filters.sortBy);
  let sortOrder = $derived($filters.sortOrder);

  function updateSort(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    const [col, order] = value.split(':');
    filters.update((f) => ({ ...f, sortBy: col, sortOrder: order as 'asc' | 'desc' }));
  }
</script>

<header class="flex h-12 shrink-0 items-center gap-4 border-b border-gray-200 bg-white px-4">
  <!-- Sort -->
  <div class="flex items-center gap-2 ml-auto">
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
