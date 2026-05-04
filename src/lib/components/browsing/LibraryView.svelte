<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { filters, fieldLocks, lockedFilters } from "$lib/stores/filters";
  import {
    currentCollectionId,
    currentSmartCollectionId,
    libraryWindowTitle,
  } from "$lib/stores/navigation";
  import { smartCollections } from "$lib/stores/smartCollections";
  import { getCollections, getScanStats, type Collection, type ImageRecord } from "$lib/commands/images";
  import { listSourceDirectories, type SourceDirectory } from "$lib/commands/sources";
  import { PageHeader } from "$lib/components/ui/page-header";
  import { StatusBar } from "$lib/components/ui/status-bar";
  import DriveIndicator from "$lib/components/drive/DriveIndicator.svelte";
  import BackgroundActivityIndicator from "$lib/components/footer/BackgroundActivityIndicator.svelte";
  import { Kbd } from "$lib/components/ui/kbd";
  import { Input } from "$lib/components/ui/input";
  import * as Select from "$lib/components/ui/select";
  import { openShortcutsHelp } from "$lib/stores/shortcutsHelp";
  import FilterBar from "$lib/components/layout/FilterBar.svelte";
  import Grid from "./Grid.svelte";
  import Search from "@lucide/svelte/icons/search";
  import X from "@lucide/svelte/icons/x";

  interface Props {
    onImageClick: (image: ImageRecord, scrollOffset: number) => void;
  }

  let { onImageClick }: Props = $props();

  let totalCount = $state<number>(0);
  let collectionName = $state<string | null>(null);

  let searchQuery = $state<string>($filters.searchQuery ?? "");
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  function onSearchInput(e: Event) {
    searchQuery = (e.target as HTMLInputElement).value;
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      filters.update((f) => ({
        ...f,
        searchQuery: searchQuery.trim() || null,
      }));
    }, 200);
  }

  function clearSearch() {
    searchQuery = "";
    filters.update((f) => ({ ...f, searchQuery: null }));
  }

  const SORT_OPTIONS: { value: string; label: string }[] = [
    { value: "catalog_number:asc", label: "Catalog # (A→Z)" },
    { value: "catalog_number:desc", label: "Catalog # (Z→A)" },
    { value: "date_start:asc", label: "Date (oldest)" },
    { value: "date_start:desc", label: "Date (newest)" },
    { value: "updated_at:desc", label: "Recently updated" },
    { value: "created_at:desc", label: "Recently added" },
  ];

  let sortValue = $derived(`${$filters.sortBy}:${$filters.sortOrder}`);

  function applySort(value: string) {
    const [col, order] = value.split(":");
    filters.update((f) => ({
      ...f,
      sortBy: col,
      sortOrder: order as "asc" | "desc",
    }));
  }

  let sortLabel = $derived(
    SORT_OPTIONS.find((o) => o.value === sortValue)?.label ?? "Sort",
  );

  // `totalImages` is the catalog-wide total (from get_scan_stats — used in
  // the StatusBar). `filteredCount` reflects the live result count of the
  // currently active filters/collection — fed by Grid via onCountChange.
  let totalImages = $state(0);
  let filteredCount = $state<number | null>(null);

  onMount(async () => {
    try {
      const stats = await getScanStats();
      totalImages = stats.total_images;
    } catch {
      // non-fatal
    }
  });

  // Look up collection name when currentCollectionId changes
  let allCollections = $state<Collection[]>([]);
  onMount(async () => {
    try {
      allCollections = await getCollections();
    } catch {}
  });

  $effect(() => {
    const id = $currentCollectionId;
    if (id === null) {
      collectionName = null;
    } else {
      collectionName = allCollections.find((c) => c.id === id)?.name ?? null;
    }
  });

  // SC name takes precedence over the regular collection name when both
  // happen to be set (shouldn't normally — Sidebar clears one when
  // setting the other — but be defensive).
  let smartName = $derived.by(() => {
    if ($currentSmartCollectionId === null) return null;
    return $smartCollections.find((s) => s.id === $currentSmartCollectionId)
      ?.name ?? null;
  });

  // Plan 12: when the source-directory tree is the active filter, render
  // the source label (and any selected subfolder) as the page title.
  let sources = $state<SourceDirectory[]>([]);
  onMount(async () => {
    try {
      sources = await listSourceDirectories();
    } catch {
      // non-fatal
    }
  });

  let sourceTitle = $derived.by(() => {
    const sourceId = $filters.sourceDirectoryId;
    if (sourceId === null || sourceId === undefined) return null;
    const source = sources.find((s) => s.id === sourceId);
    if (!source) return null;
    const sub = $filters.relativeDir;
    if (sub && sub.length > 0) {
      // Show only the leaf folder name in the title — keeps the chrome
      // tidy. The full path lives in the breadcrumb if/when we add one.
      const parts = sub.split("/").filter(Boolean);
      const leaf = parts[parts.length - 1] ?? sub;
      return `${source.label} / ${leaf}`;
    }
    return source.label;
  });

  let pageTitle = $derived(
    smartName ?? collectionName ?? sourceTitle ?? "All images",
  );

  // Mirror pageTitle into the window-chrome store so the title bar
  // reflects which slice of the catalog the user is viewing. Reset on
  // unmount so other views fall back to their own viewSuffix.
  $effect(() => {
    libraryWindowTitle.set(pageTitle);
  });
  onDestroy(() => libraryWindowTitle.set(null));

  // Show the live filtered count when known; fall back to catalog total
  // before the first query lands.
  let pageCount = $derived(filteredCount ?? totalImages);
</script>

<div class="flex flex-1 flex-col min-w-0 min-h-0">
  <PageHeader title={pageTitle} count={pageCount}>
    {#snippet right()}
      <div class="flex items-center gap-2">
        <!-- Temporary search input until ⌘K command bar lands (Plan 3).
             Width + icon padding are layout (per-instance); height, border,
             font, and focus ring all come from the Input size="xs" defaults. -->
        <div class="relative">
          <Search
            size={12}
            class="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground"
          />
          <Input
            type="search"
            size="xs"
            value={$fieldLocks.searchQuery
              ? ($lockedFilters?.searchQuery ?? "")
              : searchQuery}
            oninput={onSearchInput}
            placeholder="Search…"
            class="w-56 pl-7 pr-6"
            disabled={$fieldLocks.searchQuery}
            title={$fieldLocks.searchQuery
              ? "Search is locked by smart collection"
              : undefined}
          />
          {#if searchQuery && !$fieldLocks.searchQuery}
            <button
              onclick={clearSearch}
              class="absolute right-1.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
              aria-label="Clear search"
            >
              <X size={12} />
            </button>
          {/if}
        </div>

        <Select.Root
          type="single"
          size="xs"
          value={sortValue}
          onValueChange={(v) => v && applySort(v)}
          disabled={$fieldLocks.sort}
        >
          <Select.Trigger>{sortLabel}</Select.Trigger>
          <Select.Content>
            {#each SORT_OPTIONS as opt (opt.value)}
              <Select.Item value={opt.value}>{opt.label}</Select.Item>
            {/each}
          </Select.Content>
        </Select.Root>
      </div>
    {/snippet}
  </PageHeader>

  <FilterBar />

  <main class="flex-1 min-h-0 overflow-hidden">
    <Grid {onImageClick} onCountChange={(n) => (filteredCount = n)} />
  </main>

  <StatusBar>
    <span>{totalImages.toLocaleString()} images</span>
    <span class="text-border">|</span>
    <button
      type="button"
      onclick={openShortcutsHelp}
      class="hover:text-foreground transition-colors"
    >
      Press <Kbd dim>?</Kbd> for shortcuts
    </button>
    <div class="flex-1"></div>
    <BackgroundActivityIndicator />
    <DriveIndicator />
  </StatusBar>
</div>
