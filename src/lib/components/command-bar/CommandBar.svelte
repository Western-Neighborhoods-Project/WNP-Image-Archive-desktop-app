<script lang="ts">
  import { tick } from "svelte";
  import {
    commandBarOpen,
    commandBarQuery,
    closeCommandBar,
  } from "$lib/stores/commandBar";
  import { openShortcutsHelp } from "$lib/stores/shortcutsHelp";
  import {
    currentView,
    currentCollectionId,
    currentImageId,
    currentSettingsPage,
    type SettingsPageKey,
  } from "$lib/stores/navigation";
  import { filters } from "$lib/stores/filters";
  import {
    queryImages,
    type ImageRecord,
  } from "$lib/commands/images";
  import { Input } from "$lib/components/ui/input";
  import { Kbd } from "$lib/components/ui/kbd";
  import CommandGroup from "./CommandGroup.svelte";
  import CommandRow from "./CommandRow.svelte";

  // Lucide icons
  import Search from "@lucide/svelte/icons/search";
  import Inbox from "@lucide/svelte/icons/inbox";
  import AlignJustify from "@lucide/svelte/icons/align-justify";
  import Clock from "@lucide/svelte/icons/clock";
  import History from "@lucide/svelte/icons/history";
  import Settings from "@lucide/svelte/icons/settings";
  import EraserIcon from "@lucide/svelte/icons/eraser";
  import EyeOff from "@lucide/svelte/icons/eye-off";
  import BookOpen from "@lucide/svelte/icons/book-open";

  // ── Types ──────────────────────────────────────────────────────
  interface CommandItem {
    id: string;
    title: string;
    subtitle?: string;
    iconKey?: string; // map to component below
    image?: ImageRecord; // present for image rows
    kbd?: string[];
    action: () => void;
  }

  // Map iconKey strings → component refs (avoids Svelte's no-functions-in-state rule)
  const iconMap = {
    inbox: Inbox,
    library: AlignJustify,
    clock: Clock,
    history: History,
    settings: Settings,
    eraser: EraserIcon,
    eyeOff: EyeOff,
    book: BookOpen,
  } as const;

  let inputEl = $state<HTMLInputElement | undefined>();
  let searchResults = $state<ImageRecord[]>([]);
  let searchInflight = false;
  let selectedIndex = $state(0);

  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  // ── Recently used persistence ──────────────────────────────────
  // Tracks the last 5 things the user invoked from the bar — both static
  // commands (Go to / Actions / Settings sub-pages) and image opens.
  // Image entries cache the metadata needed to render the row, so we
  // don't have to re-fetch on bar open. Persisted to localStorage.
  // localStorage key bumped to v2 so the previous string-array format
  // (which excluded images) is treated as empty rather than crashing.
  const RECENT_KEY = "wnp.commandBar.recentEntries.v2";
  const RECENT_MAX = 5;

  type RecentEntry =
    | { kind: "action"; id: string }
    | {
        kind: "image";
        imageId: number;
        title: string;
        catalog_number: string;
        date_display: string | null;
      };

  let recentEntries = $state<RecentEntry[]>([]);

  function loadRecentEntries(): RecentEntry[] {
    try {
      const raw = localStorage.getItem(RECENT_KEY);
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return [];
      return parsed.filter((e): e is RecentEntry => {
        if (!e || typeof e !== "object") return false;
        if (e.kind === "action") return typeof e.id === "string";
        if (e.kind === "image")
          return (
            typeof e.imageId === "number" &&
            typeof e.title === "string" &&
            typeof e.catalog_number === "string"
          );
        return false;
      });
    } catch {
      return [];
    }
  }

  function saveRecentEntries(entries: RecentEntry[]) {
    try {
      localStorage.setItem(RECENT_KEY, JSON.stringify(entries));
    } catch {
      // localStorage can be unavailable (private mode etc); not fatal.
    }
  }

  /** True if two entries refer to the same item (used for dedupe). */
  function sameEntry(a: RecentEntry, b: RecentEntry): boolean {
    if (a.kind !== b.kind) return false;
    if (a.kind === "action" && b.kind === "action") return a.id === b.id;
    if (a.kind === "image" && b.kind === "image") return a.imageId === b.imageId;
    return false;
  }

  function recordRecent(item: CommandItem) {
    let entry: RecentEntry;
    if (item.image) {
      entry = {
        kind: "image",
        imageId: item.image.id,
        title: item.image.title || "(untitled)",
        catalog_number: item.image.catalog_number,
        date_display: item.image.date_display,
      };
    } else {
      entry = { kind: "action", id: item.id };
    }
    const next = [
      entry,
      ...recentEntries.filter((e) => !sameEntry(e, entry)),
    ].slice(0, RECENT_MAX);
    recentEntries = next;
    saveRecentEntries(next);
  }

  // ── Open lifecycle ─────────────────────────────────────────────
  $effect(() => {
    if ($commandBarOpen) {
      selectedIndex = 0;
      searchResults = [];
      recentEntries = loadRecentEntries();
      tick().then(() => inputEl?.focus());
    }
  });

  // ── Catalog-number routing ─────────────────────────────────────
  // Type "wnp27.4283" → that exact image gets pinned to the top of the
  // image results regardless of FTS5 ranking. Pattern: any "wnp" prefix
  // (case-insensitive) followed by digits, a dot, and more digits.
  const CATALOG_REGEX = /^wnp\d+\.\d+$/i;
  function isCatalogQuery(q: string): boolean {
    return CATALOG_REGEX.test(q.trim());
  }

  // ── Debounced search ───────────────────────────────────────────
  $effect(() => {
    const q = $commandBarQuery.trim();
    if (searchTimer !== null) clearTimeout(searchTimer);

    if (!q) {
      searchResults = [];
      return;
    }

    searchTimer = setTimeout(async () => {
      if (searchInflight) return;
      searchInflight = true;
      try {
        const result = await queryImages({
          offset: 0,
          limit: 6,
          search_query: q,
          sort_by: "catalog_number",
          sort_order: "asc",
        });
        searchResults = result.images;
      } catch (e) {
        console.error("Command bar search failed", e);
      } finally {
        searchInflight = false;
      }
    }, 150);
  });

  // ── Navigation actions ─────────────────────────────────────────
  function go(view: typeof $currentView, collectionId: number | null = null) {
    currentView.set(view);
    if (collectionId !== null) {
      currentCollectionId.set(collectionId);
      filters.update((f) => ({ ...f, collectionId }));
    }
    closeCommandBar();
  }

  function openImage(img: ImageRecord) {
    currentImageId.set(img.id);
    currentView.set("detail");
    closeCommandBar();
  }

  function clearAllFilters() {
    filters.update((f) => ({
      ...f,
      city: null,
      photographer: null,
      yearStart: null,
      yearEnd: null,
      missingMetadata: false,
      collectionId: null,
      searchQuery: null,
    }));
    closeCommandBar();
  }

  function toggleMissingFilter() {
    filters.update((f) => ({ ...f, missingMetadata: !f.missingMetadata }));
    closeCommandBar();
  }

  // ── Item lists ─────────────────────────────────────────────────
  const goToItems: CommandItem[] = [
    {
      id: "go-all",
      title: "All images",
      iconKey: "library",
      kbd: ["G", "A"],
      action: () => {
        currentCollectionId.set(null);
        filters.update((f) => ({ ...f, collectionId: null }));
        go("library");
      },
    },
    {
      id: "go-recent",
      title: "Recently viewed",
      iconKey: "clock",
      kbd: ["G", "R"],
      action: () => go("recently-viewed"),
    },
    {
      id: "go-requests",
      title: "Image requests",
      iconKey: "inbox",
      kbd: ["G", "Q"],
      action: () => go("requests"),
    },
    {
      id: "go-audit",
      title: "Audit log",
      iconKey: "history",
      kbd: ["G", "L"],
      action: () => go("audit"),
    },
    {
      id: "go-settings",
      title: "Settings",
      iconKey: "settings",
      kbd: ["G", "S"],
      action: () => go("settings"),
    },
  ];

  // Settings sub-pages — searchable as their own command bar entries.
  // Picking one navigates to settings AND deep-links to that sub-page.
  interface SettingsPageEntry {
    id: string;
    page: SettingsPageKey;
    title: string;
    subtitle: string;
  }

  const settingsPageEntries: SettingsPageEntry[] = [
    {
      id: "set-general",
      page: "general",
      title: "Settings: General",
      subtitle: "Catalog source, reset",
    },
    {
      id: "set-sharing",
      page: "sharing",
      title: "Settings: Sharing",
      subtitle: "Resolution presets — high, medium, low",
    },
    {
      id: "set-external",
      page: "external",
      title: "Settings: External services",
      subtitle: "OpenSFHistory API URL + token, Backblaze B2 credentials",
    },
    {
      id: "set-users",
      page: "users",
      title: "Settings: Users",
      subtitle: "Accounts, roles, inactivity timeout",
    },
    {
      id: "set-keyboard",
      page: "keyboard",
      title: "Settings: Keyboard shortcuts",
      subtitle: "⌘K, ⌘;, G-chords",
    },
  ];

  const settingsItems: CommandItem[] = settingsPageEntries.map((entry) => ({
    id: entry.id,
    title: entry.title,
    subtitle: entry.subtitle,
    iconKey: "settings",
    action: () => {
      currentSettingsPage.set(entry.page);
      go("settings");
    },
  }));

  const actionItems: CommandItem[] = [
    {
      id: "act-clear",
      title: "Clear all filters",
      subtitle: "Reset city, photographer, year, collection, missing-metadata",
      iconKey: "eraser",
      action: clearAllFilters,
    },
    {
      id: "act-toggle-missing",
      title: "Toggle 'Missing metadata' filter",
      iconKey: "eyeOff",
      action: toggleMissingFilter,
    },
    {
      id: "act-shortcuts",
      title: "Show keyboard shortcuts",
      subtitle: "Cheat sheet of every shortcut",
      iconKey: "book",
      kbd: ["?"],
      action: () => {
        closeCommandBar();
        openShortcutsHelp();
      },
    },
  ];

  // Filter Go-to and Actions by query (simple substring on title)
  const filteredGoTo = $derived.by(() => {
    const q = $commandBarQuery.trim().toLowerCase();
    if (!q) return goToItems;
    return goToItems.filter((i) => i.title.toLowerCase().includes(q));
  });

  const filteredActions = $derived.by(() => {
    const q = $commandBarQuery.trim().toLowerCase();
    if (!q) return actionItems;
    return actionItems.filter(
      (i) =>
        i.title.toLowerCase().includes(q) ||
        i.subtitle?.toLowerCase().includes(q),
    );
  });

  // Settings sub-pages only surface when there's a query — otherwise
  // they'd clutter the default view. Search hits "fields", "api",
  // "keyboard", etc. show the matching page entry.
  const filteredSettings = $derived.by(() => {
    const q = $commandBarQuery.trim().toLowerCase();
    if (!q) return [];
    return settingsItems.filter(
      (i) =>
        i.title.toLowerCase().includes(q) ||
        i.subtitle?.toLowerCase().includes(q),
    );
  });

  const imageItems = $derived.by<CommandItem[]>(() => {
    const q = $commandBarQuery.trim().toLowerCase();
    let results = searchResults;

    // Catalog routing: when the query is a catalog number, pin any exact
    // match to the top so the user gets there in one keystroke.
    if (isCatalogQuery(q)) {
      const exactIdx = results.findIndex(
        (img) => img.catalog_number.toLowerCase() === q,
      );
      if (exactIdx > 0) {
        const exact = results[exactIdx];
        results = [exact, ...results.slice(0, exactIdx), ...results.slice(exactIdx + 1)];
      }
    }

    return results.map((img, i) => {
      const isExactCatalog = isCatalogQuery(q) && i === 0
        && img.catalog_number.toLowerCase() === q;
      return {
        id: `img-${img.id}`,
        title: img.title || "(untitled)",
        subtitle: isExactCatalog
          ? `${img.catalog_number} · exact match${img.date_display ? ` · ${img.date_display}` : ""}`
          : `${img.catalog_number}${img.date_display ? ` · ${img.date_display}` : ""}`,
        image: img,
        action: () => openImage(img),
      };
    });
  });

  // Recently used commands — shown only at the empty state. Action
  // entries are resolved against the live command lists; image entries
  // are reconstructed from cached metadata so we don't need an async
  // round-trip on bar open. (If the image was deleted, picking the row
  // routes to a 'not found' detail view — same as any stale link.)
  const recentItems = $derived.by<CommandItem[]>(() => {
    if ($commandBarQuery.trim()) return [];
    const lookup = new Map<string, CommandItem>();
    for (const item of [...goToItems, ...actionItems, ...settingsItems]) {
      lookup.set(item.id, item);
    }
    const out: CommandItem[] = [];
    for (const entry of recentEntries) {
      if (entry.kind === "action") {
        const item = lookup.get(entry.id);
        if (item) out.push(item);
      } else {
        out.push({
          id: `img-${entry.imageId}`,
          title: entry.title,
          subtitle: `${entry.catalog_number}${entry.date_display ? ` · ${entry.date_display}` : ""}`,
          action: () => {
            currentImageId.set(entry.imageId);
            currentView.set("detail");
            closeCommandBar();
          },
        });
      }
    }
    return out;
  });

  // Flat selectable list — used for arrow-key navigation. Order matches
  // the visual order: when there's a query → Images, Actions, Go to,
  // Settings; when empty → Recently used (if any), then Actions, then Go to.
  const flatItems = $derived.by<CommandItem[]>(() => {
    const q = $commandBarQuery.trim();
    if (q)
      return [
        ...imageItems,
        ...filteredActions,
        ...filteredGoTo,
        ...filteredSettings,
      ];
    return [...recentItems, ...filteredActions, ...filteredGoTo];
  });

  // Wrapper that records the item to recents before firing the underlying
  // action. Always called from the keyboard / row click — never call
  // item.action() directly elsewhere.
  function runItem(item: CommandItem) {
    recordRecent(item);
    item.action();
  }

  // Reset selection when the result set changes
  $effect(() => {
    flatItems;
    selectedIndex = 0;
  });

  // ── Key handling on the search input ───────────────────────────
  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      closeCommandBar();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (flatItems.length === 0) return;
      selectedIndex = (selectedIndex + 1) % flatItems.length;
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (flatItems.length === 0) return;
      selectedIndex =
        (selectedIndex - 1 + flatItems.length) % flatItems.length;
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      const item = flatItems[selectedIndex];
      if (item) runItem(item);
      return;
    }
    // ⌘1 – ⌘9: jump straight to the N-th visible result. Bare 1-9 keep
    // typing into the search input as expected; the modifier disambiguates.
    if ((e.metaKey || e.ctrlKey) && /^[1-9]$/.test(e.key)) {
      e.preventDefault();
      const idx = parseInt(e.key, 10) - 1;
      const item = flatItems[idx];
      if (item) runItem(item);
      return;
    }
  }

  function onBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) closeCommandBar();
  }

  // Helper: index of an item in the flat list (so each row knows its
  // own position for selected/hover state)
  function indexOf(item: CommandItem): number {
    return flatItems.findIndex((i) => i.id === item.id);
  }

  // Cleanup any pending debounce when the bar closes
  $effect(() => {
    if (!$commandBarOpen && searchTimer !== null) {
      clearTimeout(searchTimer);
      searchTimer = null;
    }
  });
</script>

{#if $commandBarOpen}
  <div
    class="fixed inset-0 z-50 flex items-start justify-center pt-20"
    style="background: rgba(9, 9, 11, 0.35); backdrop-filter: blur(2px); -webkit-backdrop-filter: blur(2px);"
    onclick={onBackdropClick}
    role="presentation"
  >
    <div
      class="w-[600px] max-w-[calc(100vw-32px)] bg-popover text-popover-foreground rounded-[10px] overflow-hidden flex flex-col max-h-[70vh]"
      style="box-shadow: 0 0 0 1px rgba(0,0,0,0.08), 0 24px 48px rgba(0,0,0,0.2), 0 4px 12px rgba(0,0,0,0.1);"
    >
      <!-- Search input -->
      <div
        class="flex items-center gap-[10px] px-[18px] py-[14px] border-b border-border-muted"
      >
        <Search size={18} class="text-muted-foreground flex-shrink-0" />
        <input
          bind:this={inputEl}
          bind:value={$commandBarQuery}
          onkeydown={onKeyDown}
          type="text"
          placeholder="Search images, jump to a view, run an action…"
          class="flex-1 bg-transparent outline-none border-none text-base text-foreground placeholder:text-muted-foreground"
        />
        <Kbd>Esc</Kbd>
      </div>

      <!-- Result groups -->
      <div class="flex-1 overflow-y-auto py-[6px]">
        {#if flatItems.length === 0}
          <div
            class="px-[18px] py-8 text-center text-sm text-muted-foreground"
          >
            No results. Try a different search.
          </div>
        {:else}
          {#if recentItems.length > 0}
            <CommandGroup title="Recently used">
              {#each recentItems as item (item.id)}
                {@const idx = indexOf(item)}
                {@const Icon =
                  item.iconKey ? iconMap[item.iconKey as keyof typeof iconMap] : undefined}
                {@const isImage = item.id.startsWith("img-")}
                <CommandRow
                  title={item.title}
                  subtitle={item.subtitle}
                  kbd={item.kbd}
                  numericHint={idx < 9 ? idx + 1 : undefined}
                  selected={idx === selectedIndex}
                  onSelect={() => runItem(item)}
                  onHover={() => (selectedIndex = idx)}
                >
                  {#snippet icon()}
                    {#if isImage}
                      <div
                        class="w-[26px] h-[26px] rounded-[3px] bg-secondary"
                      ></div>
                    {:else if Icon}
                      <Icon size={15} />
                    {/if}
                  {/snippet}
                </CommandRow>
              {/each}
            </CommandGroup>
          {/if}

          {#if imageItems.length > 0}
            <CommandGroup
              title="Images · {imageItems.length} match{imageItems.length === 1
                ? ''
                : 'es'}"
            >
              {#each imageItems as item (item.id)}
                {@const idx = indexOf(item)}
                <CommandRow
                  title={item.title}
                  subtitle={item.subtitle}
                  numericHint={idx < 9 ? idx + 1 : undefined}
                  selected={idx === selectedIndex}
                  onSelect={() => runItem(item)}
                  onHover={() => (selectedIndex = idx)}
                >
                  {#snippet icon()}
                    <div
                      class="w-[26px] h-[26px] rounded-[3px] bg-secondary"
                    ></div>
                  {/snippet}
                </CommandRow>
              {/each}
            </CommandGroup>
          {/if}

          {#if filteredActions.length > 0}
            <CommandGroup title="Actions">
              {#each filteredActions as item (item.id)}
                {@const idx = indexOf(item)}
                {@const Icon =
                  item.iconKey ? iconMap[item.iconKey as keyof typeof iconMap] : undefined}
                <CommandRow
                  title={item.title}
                  subtitle={item.subtitle}
                  kbd={item.kbd}
                  numericHint={idx < 9 ? idx + 1 : undefined}
                  selected={idx === selectedIndex}
                  onSelect={() => runItem(item)}
                  onHover={() => (selectedIndex = idx)}
                >
                  {#snippet icon()}
                    {#if Icon}
                      <Icon size={15} />
                    {/if}
                  {/snippet}
                </CommandRow>
              {/each}
            </CommandGroup>
          {/if}

          {#if filteredGoTo.length > 0}
            <CommandGroup title="Go to">
              {#each filteredGoTo as item (item.id)}
                {@const idx = indexOf(item)}
                {@const Icon =
                  item.iconKey ? iconMap[item.iconKey as keyof typeof iconMap] : undefined}
                <CommandRow
                  title={item.title}
                  subtitle={item.subtitle}
                  kbd={item.kbd}
                  numericHint={idx < 9 ? idx + 1 : undefined}
                  selected={idx === selectedIndex}
                  onSelect={() => runItem(item)}
                  onHover={() => (selectedIndex = idx)}
                >
                  {#snippet icon()}
                    {#if Icon}
                      <Icon size={15} />
                    {/if}
                  {/snippet}
                </CommandRow>
              {/each}
            </CommandGroup>
          {/if}

          {#if filteredSettings.length > 0}
            <CommandGroup title="Settings">
              {#each filteredSettings as item (item.id)}
                {@const idx = indexOf(item)}
                {@const Icon =
                  item.iconKey ? iconMap[item.iconKey as keyof typeof iconMap] : undefined}
                <CommandRow
                  title={item.title}
                  subtitle={item.subtitle}
                  numericHint={idx < 9 ? idx + 1 : undefined}
                  selected={idx === selectedIndex}
                  onSelect={() => runItem(item)}
                  onHover={() => (selectedIndex = idx)}
                >
                  {#snippet icon()}
                    {#if Icon}
                      <Icon size={15} />
                    {/if}
                  {/snippet}
                </CommandRow>
              {/each}
            </CommandGroup>
          {/if}
        {/if}
      </div>

      <!-- Footer hints -->
      <div
        class="flex items-center gap-4 px-[14px] py-2 border-t border-border-muted bg-sidebar-bg text-[11px] text-muted-foreground"
      >
        <span class="flex items-center gap-1.5">
          <Kbd>↵</Kbd>
          Open
        </span>
        <span class="flex items-center gap-1.5">
          <Kbd>↑</Kbd>
          <Kbd>↓</Kbd>
          Navigate
        </span>
        <div class="flex-1"></div>
        <span>Search everything</span>
      </div>
    </div>
  </div>
{/if}
