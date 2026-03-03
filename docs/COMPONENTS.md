# Component Reference

All Svelte components in `src/lib/components/` and `src/routes/`.

---

## Routes

### `src/routes/+layout.svelte`
Root layout. Imports `app.css` (Tailwind). Renders the `{@render children()}` slot.

### `src/routes/+page.svelte`
Main entry point. Reads `source_directory` from app_settings on mount to determine initial view. Renders the appropriate view based on `currentView` store:
- `setup` → `SetupScreen`
- `import` → `ImportProgress`
- `library`/`settings` → sidebar + content layout

---

## Setup Components (`src/lib/components/setup/`)

### `SetupScreen.svelte`
**Purpose:** First-run setup. Lets user pick the archive directory.

**Props:**
- `onDirectorySelected: (path: string) => void` — called when user clicks "Start Import"

**Behavior:**
- Opens native directory picker via `@tauri-apps/plugin-dialog`
- Saves selected path to `app_settings.source_directory` on confirm
- Calls `onDirectorySelected` to trigger import

---

### `ImportProgress.svelte`
**Purpose:** Three-stage import progress display.

**Props:**
- `sourceDirectory: string` — directory being imported
- `onComplete: () => void` — called when all stages finish

**Behavior:**
1. Runs `scan_directory` → shows file count
2. Runs `extract_metadata_batch` → polls `get_scan_stats` every 2s for live updates
3. Runs `extract_exif_thumbnails_batch` → polls stats during extraction
4. Shows summary + "Browse Library" button on completion
5. Shows error message on failure

---

## Layout Components (`src/lib/components/layout/`)

### `Sidebar.svelte`
**Purpose:** App navigation sidebar. Fixed 220px width.

**Behavior:**
- Shows "Library" link (full grid)
- Shows "Archive Folders" section: all `source='archive'` collections with image counts
- Shows image count in footer
- Shows settings gear icon → navigates to settings view
- Clicking a collection sets `filters.collectionId` and updates the grid

---

### `TopBar.svelte`
**Purpose:** Top bar with sort dropdown.

**Behavior:**
- Sort selector updates `filters.sortBy` and `filters.sortOrder`
- Sort options: Catalog #, Date, Recently updated, Recently added

---

### `SettingsView.svelte`
**Purpose:** Settings panel.

**Props:**
- `onResetComplete: () => void` — called after successful catalog reset

**Behavior:**
- Shows current source directory
- "Change Source Directory" → confirmation dialog → `reset_catalog` → calls `onResetComplete`

---

## Browsing Components (`src/lib/components/browsing/`)

### `Grid.svelte`
**Purpose:** Virtual-scrolling image grid. Core browsing component.

**Props:**
- `onImageClick: (image: ImageRecord) => void`

**Architecture:**
- Uses `createVirtualizer` from `@tanstack/svelte-virtual` (Svelte store-based)
- Column count computed from container width: `Math.floor((width + gap) / (itemSize + gap))`
- Row height: 236px (200px thumbnail + 28px label + 8px gap)
- Pages fetched on demand as rows scroll into view (100 images/page)
- Page cache: `Map<pageIndex, ImageRecord[]>` (in-memory, cleared on filter change)
- Subscribes to `filters` store — reloads when filters change

**Loading strategy:**
- `fetchPage(0)` on mount → totalCount → totalRows → virtualizer count
- For each visible virtual row, compute needed pages → fetch if not cached
- Images render immediately from cache; unfetched slots show a loading placeholder

---

### `GridItem.svelte`
**Purpose:** Single image thumbnail cell in the grid.

**Props:**
- `image: ImageRecord`
- `onclick: (image: ImageRecord) => void`

**Behavior:**
- Loads thumbnail via `convertFileSrc(thumbnail_path)` with cache-buster
- If `thumbnail_generated === false`, queues image in `thumbnailQueue` on mount
- Subscribes to `thumbnailQueue.onRefresh()` to detect when its thumbnail is upgraded
- Shows a placeholder SVG when `thumbnail_path` is null

---

## Stores (`src/lib/stores/`)

### `navigation.ts`
```typescript
currentView: Writable<'setup' | 'import' | 'library' | 'detail' | 'collection' | 'requests' | 'settings'>
currentImageId: Writable<number | null>
currentCollectionId: Writable<number | null>
```

### `filters.ts`
```typescript
filters: Writable<{
  city: string | null;
  photographer: string | null;
  collectionId: number | null;
  yearStart: number | null;
  yearEnd: number | null;
  missingMetadata: boolean;
  searchQuery: string | null;
  sortBy: string;
  sortOrder: 'asc' | 'desc';
}>
```
Grid subscribes to this store and reloads when it changes.

---

## Utilities (`src/lib/utils/`)

### `format.ts`
- `formatFileSize(bytes)` → `"14.2 MB"`
- `formatCount(n)` → `"52,341"`
- `parseKeywords(json)` → `string[]`

### `thumbnailQueue.ts`
Singleton queue for on-demand full-quality thumbnail generation.
- `thumbnailQueue.add(imageId)` — queue an image for generation
- `thumbnailQueue.onRefresh(callback)` — subscribe to completion events; returns unsubscribe function
- Debounces 300ms, batches up to 20 IDs, calls `generate_full_thumbnails`
