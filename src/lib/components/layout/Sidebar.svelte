<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    currentView,
    currentCollectionId,
    currentSmartCollectionId,
  } from "$lib/stores/navigation";
  import { filters, resetFilters } from "$lib/stores/filters";
  import { scanDirectory } from "$lib/commands/images";
  import {
    userCollections,
    refreshUserCollections,
  } from "$lib/stores/collections";
  import {
    getSourceDirectoryTree,
    listSourceDirectories,
    type SourceTreeRoot,
  } from "$lib/commands/sources";
  import { ordersResponse, refreshOrders } from "$lib/stores/requests";
  import SourceTree from "./sidebar/SourceTree.svelte";
  import { Kbd, KbdSeq } from "$lib/components/ui/kbd";
  import { openCommandBar } from "$lib/stores/commandBar";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
  import { DropdownMenuPrimitive } from "$lib/components/ui/dropdown-menu";
  import * as ContextMenu from "$lib/components/ui/context-menu";
  import { ContextMenuPrimitive } from "$lib/components/ui/context-menu";
  import {
    smartCollections,
    refreshSmartCollections,
  } from "$lib/stores/smartCollections";
  import {
    deleteSmartCollection,
    type SmartCollection,
  } from "$lib/commands/smartCollections";
  import CollectionDialogs from "$lib/components/collections/CollectionDialogs.svelte";
  import SideGroup from "./sidebar/SideGroup.svelte";
  import SideItem from "./sidebar/SideItem.svelte";
  import ActivityCard from "./sidebar/ActivityCard.svelte";
  import UserMenu from "$lib/components/auth/UserMenu.svelte";
  import { isAdmin } from "$lib/stores/currentUser";
  import { addToCollection } from "$lib/commands/collections";

  // Lucide icons
  import Search from "@lucide/svelte/icons/search";
  import Inbox from "@lucide/svelte/icons/inbox";
  import AlignJustify from "@lucide/svelte/icons/align-justify";
  import Clock from "@lucide/svelte/icons/clock";
  import Folder from "@lucide/svelte/icons/folder";
  import Star from "@lucide/svelte/icons/star";
  import Filter from "@lucide/svelte/icons/filter";
  import History from "@lucide/svelte/icons/history";
  import Plus from "@lucide/svelte/icons/plus";
  import Settings from "@lucide/svelte/icons/settings";

  let sourceTree = $state<SourceTreeRoot[]>([]);
  // Track expanded state for the source-directory tree. Keys are
  // "<sourceId>:<relativeDir>". Source roots default to expanded so the
  // user sees the structure on first sight.
  let expandedKeys = $state<Set<string>>(new Set());

  // Dialog state for CollectionDialogs
  let showCreate = $state(false);
  let showRename = $state(false);
  let showDelete = $state(false);
  let targetCollection = $state<{ id: number; name: string } | null>(null);

  async function refreshSourceTree() {
    try {
      sourceTree = await getSourceDirectoryTree();
      // Auto-expand each source root the first time we see it.
      const next = new Set(expandedKeys);
      for (const root of sourceTree) {
        next.add(`${root.source.id}:`);
      }
      expandedKeys = next;
    } catch (e) {
      console.error("Failed to load source tree", e);
    }
  }

  onMount(async () => {
    try {
      await Promise.all([
        refreshSourceTree(),
        refreshUserCollections(),
        refreshOrders(),
        refreshSmartCollections(),
      ]);
    } catch (e) {
      console.error("Sidebar load error:", e);
    }
  });

  // Refresh the tree whenever the user comes back to the library —
  // catches re-scans from settings without needing a Tauri event bus.
  $effect(() => {
    if ($currentView === "library") {
      refreshSourceTree();
    }
  });

  // ── Plan 12 watcher subscription ─────────────────────────────────────────
  // The backend emits library:filesystem-changed with a list of source ids
  // when files appear/disappear. We just call scanDirectory for each
  // affected source (inserts new image rows in 'pending' state) and
  // refresh the tree. The Plan 13 background worker picks up thumbnail +
  // metadata generation on its own poll cycle.
  let isHandlingFsChange = false;
  let unlistenFsChange: UnlistenFn | null = null;
  onMount(async () => {
    try {
      unlistenFsChange = await listen<number[]>(
        "library:filesystem-changed",
        async (event) => {
          if (isHandlingFsChange) return;
          isHandlingFsChange = true;
          try {
            const affected = event.payload;
            // Fetch a fresh list rather than relying on the cached
            // sourceTree — covers the race where a source is added
            // (e.g. via Settings) right before this event fires and
            // the tree hasn't refreshed yet.
            const sources = await listSourceDirectories();
            const sourceById = new Map(sources.map((s) => [s.id, s.path]));
            for (const id of affected) {
              const path = sourceById.get(id);
              if (path) {
                try {
                  await scanDirectory(path);
                } catch (e) {
                  console.warn("watcher rescan failed", id, e);
                }
              }
            }
            await refreshSourceTree();
          } finally {
            isHandlingFsChange = false;
          }
        },
      );
    } catch (e) {
      console.error("Failed to subscribe to library:filesystem-changed", e);
    }
  });
  onDestroy(() => {
    unlistenFsChange?.();
  });

  // Apply a saved smart collection. The SC's saved filter values are
  // pulled in via the lockedFilters derived store (in filters.ts) —
  // we just clear user filters and set the id so that derivation
  // takes over. Navigation lands on the library view, which now
  // renders the SC name as its title.
  function applySmartCollection(sc: SmartCollection) {
    resetFilters();
    currentCollectionId.set(null);
    currentSmartCollectionId.set(sc.id);
    currentView.set("library");
  }

  async function handleDeleteSmartCollection(sc: SmartCollection) {
    try {
      await deleteSmartCollection(sc.id);
      await refreshSmartCollections();
      // If we just deleted the active SC, also clear the navigation
      // state so we don't dangle pointing at a missing id.
      if ($currentSmartCollectionId === sc.id) {
        currentSmartCollectionId.set(null);
      }
    } catch (e) {
      console.error("Failed to delete smart collection", e);
    }
  }

  // Refresh order count when navigating to/from requests
  $effect(() => {
    if ($currentView === "requests") {
      refreshOrders();
    }
  });

  function goToLibrary() {
    currentView.set("library");
    currentCollectionId.set(null);
    currentSmartCollectionId.set(null);
    resetFilters();
  }

  function goToRecentlyViewed() {
    currentView.set("recently-viewed");
  }

  function goToCollection(id: number) {
    currentView.set("library");
    currentCollectionId.set(id);
    currentSmartCollectionId.set(null);
    filters.update((f) => ({ ...f, collectionId: id }));
  }

  // ── Source tree click + active state ─────────────────────────────────────
  // Selecting a tree node sets the source/relativeDir filters and clears
  // any user/smart collection selection. The tree key for the active node
  // is derived from $filters so navigation outside the tree (e.g. via
  // command bar) keeps highlighting in sync.
  function selectSourceTreeNode(sourceId: number, relativeDir: string) {
    currentView.set("library");
    currentCollectionId.set(null);
    currentSmartCollectionId.set(null);
    filters.update((f) => ({
      ...f,
      collectionId: null,
      sourceDirectoryId: sourceId,
      relativeDir: relativeDir === "" ? null : relativeDir,
    }));
  }

  function toggleTreeKey(key: string) {
    const next = new Set(expandedKeys);
    if (next.has(key)) {
      next.delete(key);
    } else {
      next.add(key);
    }
    expandedKeys = next;
  }

  let activeTreeKey = $derived(
    $currentView === "library" &&
      $currentCollectionId === null &&
      $currentSmartCollectionId === null &&
      $filters.sourceDirectoryId !== null
      ? `${$filters.sourceDirectoryId}:${$filters.relativeDir ?? ""}`
      : null,
  );

  function goTo(view: typeof $currentView) {
    currentView.set(view);
    if (view === "requests") refreshOrders();
  }

  function openRename(col: { id: number; name: string }) {
    targetCollection = col;
    showRename = true;
  }

  function openDelete(col: { id: number; name: string }) {
    targetCollection = col;
    showDelete = true;
  }

  // Sidebar badge counts only "processing" orders (i.e. requests still
  // awaiting action). meta.fulfillable from the API isn't strictly the
  // same — derive directly from the data so the number always matches
  // what RequestsView's "Processing" tab shows.
  let pendingCount = $derived(
    $ordersResponse?.data.filter((o) => o.status === "processing").length ?? 0,
  );

  // ── Drag-and-drop into collections (Plan 11) ───────────────────────────
  // GridItem stashes the dragged image-id list on the dataTransfer under
  // "application/x-wnp-images". We accept it here on user collection
  // rows; archive collections (auto-generated from folder structure) are
  // not valid drop targets.
  let dragOverCollectionId = $state<number | null>(null);

  function handleCollectionDragOver(e: DragEvent, collectionId: number) {
    if (!e.dataTransfer) return;
    if (!e.dataTransfer.types.includes("application/x-wnp-images")) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    dragOverCollectionId = collectionId;
  }

  function handleCollectionDragLeave(collectionId: number) {
    if (dragOverCollectionId === collectionId) dragOverCollectionId = null;
  }

  async function handleCollectionDrop(e: DragEvent, collectionId: number) {
    if (!e.dataTransfer) return;
    e.preventDefault();
    dragOverCollectionId = null;
    const raw = e.dataTransfer.getData("application/x-wnp-images");
    if (!raw) return;
    let ids: number[] = [];
    try {
      ids = JSON.parse(raw);
    } catch {
      return;
    }
    if (!Array.isArray(ids) || ids.length === 0) return;
    try {
      await addToCollection(collectionId, ids);
      await refreshUserCollections();
    } catch (err) {
      console.error("Drop add-to-collection failed", err);
    }
  }
</script>

<aside
  class="w-[248px] flex-shrink-0 bg-sidebar-bg border-r border-border flex flex-col overflow-hidden"
>
  <!-- Brand header -->
  <div
    class="h-[56px] px-4 flex items-center gap-[10px] border-b border-border"
  >
    <div
      class="w-[22px] h-[22px] rounded-[5px] bg-primary flex items-center justify-center"
    >
      <div class="w-2 h-2 bg-primary-foreground rounded-[1px]"></div>
    </div>
    <div class="text-[13px] font-semibold text-foreground tracking-[-0.1px]">
      Image Archive Manager
    </div>
  </div>

  <!-- ⌘K launcher: opens the global command palette -->
  <div class="p-[10px] pb-[6px]">
    <button
      type="button"
      onclick={openCommandBar}
      class="w-full flex items-center gap-2 h-[30px] px-[10px] rounded-md bg-background border border-border text-muted-foreground text-[12.5px] hover:bg-hover hover:text-foreground transition-colors"
      title="Open command bar"
    >
      <Search size={13} />
      <span class="flex-1 text-left">Search or jump to…</span>
      <KbdSeq keys={["⌘", "K"]} />
    </button>
  </div>

  <!-- Scrollable nav -->
  <div class="flex-1 overflow-auto py-[6px]">
    <SideGroup title="Actions">
      <SideItem
        label="Image requests"
        badge={pendingCount > 0 ? pendingCount : undefined}
        selected={$currentView === "requests"}
        kbd="G Q"
        onclick={() => goTo("requests")}
      >
        {#snippet icon()}
          <Inbox size={14} />
        {/snippet}
      </SideItem>
    </SideGroup>

    <SideGroup title="Library">
      <SideItem
        label="All images"
        selected={$currentView === "library" &&
          $currentCollectionId === null &&
          $currentSmartCollectionId === null &&
          $filters.sourceDirectoryId === null}
        kbd="G A"
        onclick={goToLibrary}
      >
        {#snippet icon()}
          <AlignJustify size={14} />
        {/snippet}
      </SideItem>
      <SideItem
        label="Recently viewed"
        selected={$currentView === "recently-viewed"}
        kbd="G R"
        onclick={goToRecentlyViewed}
      >
        {#snippet icon()}
          <Clock size={14} />
        {/snippet}
      </SideItem>
    </SideGroup>

    {#if sourceTree.length > 0}
      <SideGroup title="Archival Collections">
        {#each sourceTree as root (root.source.id)}
          {@const rootNode = {
            sourceDirectoryId: root.source.id,
            label: root.source.label,
            relativeDir: "",
            imageCount: root.source.imageCount,
            children: root.children,
          }}
          <SourceTree
            node={rootNode}
            depth={0}
            activeKey={activeTreeKey}
            expanded={expandedKeys}
            onToggle={toggleTreeKey}
            onSelect={selectSourceTreeNode}
            labelOverride={root.source.label}
            isSourceRoot={true}
          />
        {/each}
      </SideGroup>
    {/if}

    <SideGroup title="Collections">
      {#snippet action()}
        <button
          type="button"
          onclick={() => (showCreate = true)}
          class="rounded p-0.5 text-muted-foreground hover:bg-hover hover:text-foreground"
          title="New collection"
        >
          <Plus size={13} />
        </button>
      {/snippet}
      {#if $userCollections.length === 0}
        <p class="px-[14px] text-xs text-muted-foreground italic">
          No collections yet
        </p>
      {:else}
        {#each $userCollections as col (col.id)}
          <div
            role="presentation"
            class="group relative w-[calc(100%-16px)] mx-2 flex items-center rounded-md transition-colors
              {dragOverCollectionId === col.id
              ? 'bg-primary/15 ring-1 ring-primary'
              : $currentCollectionId === col.id
                ? 'bg-secondary'
                : 'hover:bg-hover'}"
            ondragover={(e) => handleCollectionDragOver(e, col.id)}
            ondragleave={() => handleCollectionDragLeave(col.id)}
            ondrop={(e) => handleCollectionDrop(e, col.id)}
          >
            <button
              type="button"
              onclick={() => goToCollection(col.id)}
              class="flex-1 flex items-center gap-[10px] h-[30px] pl-3 pr-[10px] text-[13px] text-left
                {$currentCollectionId === col.id
                ? 'text-foreground font-medium'
                : 'text-muted-fg-2'}"
            >
              <span
                class="flex {$currentCollectionId === col.id
                  ? 'text-foreground'
                  : 'text-muted-foreground'}"
              >
                <Star size={14} />
              </span>
              <span
                class="flex-1 overflow-hidden text-ellipsis whitespace-nowrap"
              >
                {col.name}
              </span>
              <span class="text-[11px] text-muted-foreground tabular-nums">
                {col.image_count.toLocaleString()}
              </span>
            </button>
            <DropdownMenu.Root>
              <DropdownMenuPrimitive.Trigger>
                {#snippet child({ props })}
                  <button
                    {...props}
                    class="mr-1 shrink-0 rounded p-0.5 text-muted-foreground opacity-0 group-hover:opacity-100 hover:bg-hover hover:text-foreground"
                    title="Collection options"
                    onclick={(e: MouseEvent) => e.stopPropagation()}
                  >
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      class="h-3.5 w-3.5"
                      fill="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <circle cx="5" cy="12" r="1.5" />
                      <circle cx="12" cy="12" r="1.5" />
                      <circle cx="19" cy="12" r="1.5" />
                    </svg>
                  </button>
                {/snippet}
              </DropdownMenuPrimitive.Trigger>
              <DropdownMenu.Content align="end">
                <DropdownMenu.Item onclick={() => openRename(col)}
                  >Rename</DropdownMenu.Item
                >
                <DropdownMenu.Separator />
                <DropdownMenu.Item
                  class="text-destructive focus:text-destructive"
                  onclick={() => openDelete(col)}>Delete</DropdownMenu.Item
                >
              </DropdownMenu.Content>
            </DropdownMenu.Root>
          </div>
        {/each}
      {/if}
    </SideGroup>

    <SideGroup title="Smart Collections">
      {#if $smartCollections.length === 0}
        <p class="px-[14px] text-xs text-muted-foreground italic">
          No smart collections yet
        </p>
      {:else}
        {#each $smartCollections as sc (sc.id)}
          {@const active = $currentSmartCollectionId === sc.id}
          <ContextMenu.Root>
            <ContextMenuPrimitive.Trigger class="block">
              <button
                type="button"
                onclick={() => applySmartCollection(sc)}
                class="w-[calc(100%-16px)] mx-2 flex items-center gap-[10px] h-[30px] pl-3 pr-[10px] rounded-md text-[13px] text-left transition-colors {active
                  ? 'bg-secondary text-foreground font-medium'
                  : 'text-muted-fg-2 hover:bg-hover'}"
              >
                <span class="flex {active ? 'text-foreground' : 'text-muted-foreground'}">
                  <Filter size={14} />
                </span>
                <span class="flex-1 overflow-hidden text-ellipsis whitespace-nowrap">
                  {sc.name}
                </span>
              </button>
            </ContextMenuPrimitive.Trigger>
            <ContextMenu.Content>
              <ContextMenu.Item
                class="text-destructive focus:text-destructive"
                onclick={() => handleDeleteSmartCollection(sc)}
              >
                Delete
              </ContextMenu.Item>
            </ContextMenu.Content>
          </ContextMenu.Root>
        {/each}
      {/if}
    </SideGroup>

    <SideGroup title="System">
      <SideItem
        label="Audit log"
        selected={$currentView === "audit"}
        kbd="G L"
        onclick={() => goTo("audit")}
      >
        {#snippet icon()}
          <History size={14} />
        {/snippet}
      </SideItem>
      {#if $isAdmin}
        <SideItem
          label="Settings"
          selected={$currentView === "settings"}
          kbd="G S"
          onclick={() => goTo("settings")}
        >
          {#snippet icon()}
            <Settings size={14} />
          {/snippet}
        </SideItem>
      {/if}
    </SideGroup>
  </div>

  <ActivityCard />
  <UserMenu />
</aside>

<!-- Collection CRUD dialogs (mounted once, shared state) -->
<CollectionDialogs
  bind:showCreate
  bind:showRename
  bind:showDelete
  bind:targetCollection
/>
