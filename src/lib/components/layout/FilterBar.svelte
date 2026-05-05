<script lang="ts">
  import { onMount } from "svelte";
  import {
    filters,
    fieldLocks,
    lockedFilters,
    effectiveFilters,
    type FilterState,
  } from "$lib/stores/filters";
  import {
    getFilterOptions,
    getCollections,
    type FilterOptions,
    type Collection,
  } from "$lib/commands/images";
  import { Input } from "$lib/components/ui/input";
  import * as Select from "$lib/components/ui/select";
  import SaveSmartCollectionDialog from "$lib/components/collections/SaveSmartCollectionDialog.svelte";
  import { Bookmark, Lock } from "@lucide/svelte";

  let filterOptions = $state<FilterOptions | null>(null);
  let archiveCollections = $state<Collection[]>([]);

  let city = $state<string>($filters.city ?? "");
  let photographer = $state<string>($filters.photographer ?? "");
  let yearStart = $state<string>($filters.yearStart?.toString() ?? "");
  let yearEnd = $state<string>($filters.yearEnd?.toString() ?? "");
  let missingMetadata = $state<boolean>($filters.missingMetadata);
  let collectionId = $state<string>($filters.collectionId?.toString() ?? "");

  let hasActiveFilters = $derived(
    !!city ||
      !!photographer ||
      !!yearStart ||
      !!yearEnd ||
      missingMetadata ||
      !!collectionId,
  );

  onMount(async () => {
    try {
      [filterOptions, archiveCollections] = await Promise.all([
        getFilterOptions(),
        getCollections(),
      ]);
    } catch (e) {
      console.warn("Could not load filter options:", e);
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
    city = "";
    photographer = "";
    yearStart = "";
    yearEnd = "";
    missingMetadata = false;
    collectionId = "";
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

  $effect(() => {
    city;
    photographer;
    yearStart;
    yearEnd;
    missingMetadata;
    collectionId;
    applyFilters();
  });

  let archiveOnly = $derived(
    archiveCollections.filter((c) => c.source === "archive"),
  );

  let collectionLabel = $derived.by(() => {
    if (!collectionId) return null;
    return (
      archiveOnly.find((c) => c.id.toString() === collectionId)?.name ?? null
    );
  });

  // Save-as-smart-collection state. Snapshot is captured from the
  // EFFECTIVE filters (locked + user) at the moment the dialog opens
  // so saving from inside an existing SC captures both halves.
  let showSaveSmart = $state(false);
  let saveSnapshot = $state<FilterState | null>(null);

  function openSaveSmart() {
    saveSnapshot = { ...$effectiveFilters };
    showSaveSmart = true;
  }
</script>

<div
  class="flex shrink-0 flex-wrap items-center gap-1.5 border-b border-border-muted bg-background px-5 py-2"
>
  <span class="text-xs text-muted-foreground mr-1">Filters</span>

  <!-- City -->
  {#if $fieldLocks.city}
    <span
      class="inline-flex items-center gap-1 h-[26px] rounded-md bg-secondary px-2 text-xs text-foreground"
      title="Locked by smart collection"
    >
      <Lock class="size-2.5 text-muted-foreground" />
      City: {$lockedFilters?.city}
    </span>
  {:else if filterOptions && filterOptions.cities.length > 0}
    <Select.Root
      type="single"
      size="xs"
      value={city}
      onValueChange={(v) => (city = v ?? "")}
    >
      <Select.Trigger>
        {#if city}
          {city}
        {:else}
          <span class="text-muted-foreground">City</span>
        {/if}
      </Select.Trigger>
      <Select.Content>
        <Select.Item value="">All cities</Select.Item>
        {#each filterOptions.cities as c (c)}
          <Select.Item value={c}>{c}</Select.Item>
        {/each}
      </Select.Content>
    </Select.Root>
  {:else}
    <Input size="xs" bind:value={city} placeholder="City" class="w-28" />
  {/if}

  <!-- Photographer -->
  {#if $fieldLocks.photographer}
    <span
      class="inline-flex items-center gap-1 h-[26px] rounded-md bg-secondary px-2 text-xs text-foreground"
      title="Locked by smart collection"
    >
      <Lock class="size-2.5 text-muted-foreground" />
      Photographer: {$lockedFilters?.photographer}
    </span>
  {:else if filterOptions && filterOptions.photographers.length > 0}
    <Select.Root
      type="single"
      size="xs"
      value={photographer}
      onValueChange={(v) => (photographer = v ?? "")}
    >
      <Select.Trigger>
        {#if photographer}
          {photographer}
        {:else}
          <span class="text-muted-foreground">Photographer</span>
        {/if}
      </Select.Trigger>
      <Select.Content>
        <Select.Item value="">All photographers</Select.Item>
        {#each filterOptions.photographers as p (p)}
          <Select.Item value={p}>{p}</Select.Item>
        {/each}
      </Select.Content>
    </Select.Root>
  {:else}
    <Input size="xs" bind:value={photographer} placeholder="Photographer" class="w-32" />
  {/if}

  <!-- Year range -->
  {#if $fieldLocks.yearStart || $fieldLocks.yearEnd}
    <span
      class="inline-flex items-center gap-1 h-[26px] rounded-md bg-secondary px-2 text-xs text-foreground"
      title="Locked by smart collection"
    >
      <Lock class="size-2.5 text-muted-foreground" />
      Year: {$lockedFilters?.yearStart ?? "*"}–{$lockedFilters?.yearEnd ?? "*"}
    </span>
  {:else}
    <div class="flex items-center gap-1">
      <Input
        type="number"
        size="xs"
        bind:value={yearStart}
        placeholder="Year from"
        min="1800"
        max="2100"
        class="w-[88px]"
      />
      <span class="text-xs text-muted-foreground">–</span>
      <Input
        type="number"
        size="xs"
        bind:value={yearEnd}
        placeholder="to"
        min="1800"
        max="2100"
        class="w-[64px]"
      />
    </div>
  {/if}

  <!-- Archive collection -->
  {#if $fieldLocks.collectionId}
    {@const lockedColName = archiveOnly.find(
      (c) => c.id === $lockedFilters?.collectionId,
    )?.name}
    <span
      class="inline-flex items-center gap-1 h-[26px] rounded-md bg-secondary px-2 text-xs text-foreground"
      title="Locked by smart collection"
    >
      <Lock class="size-2.5 text-muted-foreground" />
      Collection: {lockedColName ?? `#${$lockedFilters?.collectionId}`}
    </span>
  {:else if archiveOnly.length > 0}
    <Select.Root
      type="single"
      size="xs"
      value={collectionId}
      onValueChange={(v) => (collectionId = v ?? "")}
    >
      <Select.Trigger>
        {#if collectionLabel}
          {collectionLabel}
        {:else}
          <span class="text-muted-foreground">Collection</span>
        {/if}
      </Select.Trigger>
      <Select.Content>
        <Select.Item value="">All collections</Select.Item>
        {#each archiveOnly as col (col.id)}
          <Select.Item value={col.id.toString()}>
            {col.name} ({col.image_count.toLocaleString()})
          </Select.Item>
        {/each}
      </Select.Content>
    </Select.Root>
  {/if}

  <!-- Missing metadata -->
  {#if $fieldLocks.missingMetadata}
    <span
      class="inline-flex items-center gap-1 h-[26px] rounded-md bg-secondary px-2 text-xs text-foreground"
      title="Locked by smart collection"
    >
      <Lock class="size-2.5 text-muted-foreground" />
      Missing metadata
    </span>
  {:else}
    <label class="flex cursor-pointer items-center gap-1.5 ml-1">
      <input
        type="checkbox"
        bind:checked={missingMetadata}
        class="h-3.5 w-3.5 rounded border-border accent-primary"
      />
      <span class="text-xs text-muted-fg-2">Missing metadata</span>
    </label>
  {/if}

  {#if hasActiveFilters}
    <div class="ml-auto flex items-center gap-1">
      <button
        type="button"
        onclick={openSaveSmart}
        class="inline-flex items-center gap-1 rounded px-2 py-1 text-xs text-muted-foreground hover:bg-hover hover:text-foreground"
        title="Save these filters as a smart collection"
      >
        <Bookmark class="size-3" />
        Save filter
      </button>
      <button
        type="button"
        onclick={clearFilters}
        class="rounded px-2 py-1 text-xs text-muted-foreground hover:bg-hover hover:text-foreground"
      >
        Clear all
      </button>
    </div>
  {/if}
</div>

{#if saveSnapshot}
  <SaveSmartCollectionDialog
    bind:open={showSaveSmart}
    snapshot={saveSnapshot}
  />
{/if}
