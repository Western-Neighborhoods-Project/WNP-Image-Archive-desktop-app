# Design System Refactor & Sidebar Rebuild — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the app's current slate-blue color tokens, font stack, and sidebar layout with the design-system equivalents from the Claude Design handoff bundle (`_design/wnp-app/project/`). Add the shared primitives (`Kbd`, `KbdSeq`, `FilterChip`, `PageHeader`, `StatusBar`, `ActivityCard`) that all subsequent view refactors will consume. Stub the not-yet-built sidebar destinations so navigation works end-to-end.

**Architecture:** Tokens live in `src/app.css` as HSL CSS variables in `:root` mapped to Tailwind v4 `@theme` color tokens, matching the existing shadcn-svelte v1.1.1 pattern. New primitives live in `src/lib/components/ui/` following the existing folder-per-component shadcn-svelte convention. Sidebar internals (`SideGroup`, `SideItem`, `ActivityCard`) are co-located in `src/lib/components/layout/sidebar/` since `Sidebar.svelte` is the only consumer. Stub views are bare Svelte components in `src/lib/components/stubs/` and route via the existing `$currentView` writable.

**Tech Stack:** Svelte 5 runes, SvelteKit 2, Tailwind CSS v4 + `@tailwindcss/vite`, shadcn-svelte v1.1.1, bits-ui v1.x, `@lucide/svelte`, `tailwind-merge`, `clsx`, `tailwind-variants`, Tauri 2 (Rust backend, SQLite via rusqlite).

**Out of scope (future plans):** main-content view refactors (Library/Grid, Detail, Requests, Settings); ⌘K command bar; full Audit log view; full Backups view; full Import inbox refactor; Drive monitoring; Share dialog; OpenSFHistory metadata sync.

---

## File Inventory

### Files created
- `src/lib/components/ui/kbd/Kbd.svelte`
- `src/lib/components/ui/kbd/KbdSeq.svelte`
- `src/lib/components/ui/kbd/index.ts`
- `src/lib/components/ui/filter-chip/FilterChip.svelte`
- `src/lib/components/ui/filter-chip/index.ts`
- `src/lib/components/ui/page-header/PageHeader.svelte`
- `src/lib/components/ui/page-header/index.ts`
- `src/lib/components/ui/status-bar/StatusBar.svelte`
- `src/lib/components/ui/status-bar/index.ts`
- `src/lib/components/layout/sidebar/SideGroup.svelte`
- `src/lib/components/layout/sidebar/SideItem.svelte`
- `src/lib/components/layout/sidebar/ActivityCard.svelte`
- `src/lib/components/stubs/StubView.svelte` (shared "coming soon" wrapper)
- `src/lib/components/stubs/AuditLogStub.svelte`
- `src/lib/components/stubs/BackupsStub.svelte`
- `src/lib/components/stubs/ImportInboxStub.svelte`
- `src/lib/components/stubs/MissingMetadataStub.svelte`
- `src/lib/components/stubs/SmartCollectionsStub.svelte`
- `src/lib/commands/activity.ts`

### Files modified
- `src/app.css` (full rewrite of `:root`/`@theme`/base)
- `src-tauri/tauri.conf.json` (window width)
- `src-tauri/src/models.rs` (add `RecentActivityEntry` struct)
- `src-tauri/src/editor.rs` (add `get_recent_activity` command)
- `src-tauri/src/lib.rs` (register new command)
- `src/lib/components/layout/Sidebar.svelte` (full rewrite)
- `src/lib/stores/navigation.ts` (extend `ViewType`)
- `src/routes/+page.svelte` (route the new view types)

---

## Design palette reference (zinc neutrals from `_design/wnp-app/project/lib-shadcn.jsx`)

| Token | Hex | HSL |
|---|---|---|
| bg | `#ffffff` | `0 0% 100%` |
| sidebarBg | `#fafafa` | `0 0% 98%` |
| panelBg | `#f7f7f8` | `240 5% 97%` |
| border | `#e4e4e7` | `240 6% 90%` |
| borderMuted | `#f4f4f5` | `240 5% 96%` |
| fg | `#09090b` | `240 10% 4%` |
| muted | `#71717a` | `240 4% 46%` |
| mutedFg | `#52525b` | `240 5% 34%` |
| mutedFg2 | `#3f3f46` | `240 5% 26%` |
| subtle | `#f4f4f5` | `240 5% 96%` |
| hover | `#f1f1f3` | `240 5% 95%` |
| accent | `#18181b` | `240 6% 10%` |
| danger | `#dc2626` | `0 73% 50%` |
| warning | `#f59e0b` | `38 92% 50%` |
| success | `#16a34a` | `142 71% 36%` |
| info | `#2563eb` | `220 83% 53%` |

---

### Task 1: Replace color tokens, fonts, and base styles in `app.css`

**Files:**
- Modify: `src/app.css` (full rewrite, currently 72 lines)

**Why:** Current tokens are slate/blue (e.g. `--primary: 222.2 47.4% 11.2%`, `--accent: 210 40% 96.1%`). Design uses zinc neutrals with `#18181b` as the dark accent, plus extra tokens that don't exist yet (`sidebar-bg`, `panel-bg`, `border-muted`, `hover`, `success`, `warning`, `info`). After this task, every existing component using `bg-primary` / `text-muted-foreground` / `border-border` etc. automatically picks up the new palette.

- [ ] **Step 1: Replace `app.css` contents**

Replace the entire file contents with:

```css
@import "tailwindcss";

/* ============================================================
   shadcn-svelte / Tailwind v4 CSS variables (zinc base)
   Palette derived from _design/wnp-app/project/lib-shadcn.jsx
   ============================================================ */
:root {
  /* shadcn-svelte semantic tokens */
  --background: 0 0% 100%;
  --foreground: 240 10% 4%;
  --card: 0 0% 100%;
  --card-foreground: 240 10% 4%;
  --popover: 0 0% 100%;
  --popover-foreground: 240 10% 4%;
  --primary: 240 6% 10%;
  --primary-foreground: 0 0% 98%;
  --secondary: 240 5% 96%;
  --secondary-foreground: 240 6% 10%;
  --muted: 240 5% 96%;
  --muted-foreground: 240 4% 46%;
  --accent: 240 5% 96%;
  --accent-foreground: 240 6% 10%;
  --destructive: 0 73% 50%;
  --destructive-foreground: 0 0% 98%;
  --border: 240 6% 90%;
  --input: 240 6% 90%;
  --ring: 240 10% 4%;

  /* Extended design tokens (not in vanilla shadcn) */
  --sidebar-bg: 0 0% 98%;
  --panel-bg: 240 5% 97%;
  --border-muted: 240 5% 96%;
  --hover: 240 5% 95%;
  --muted-fg-2: 240 5% 26%;
  --success: 142 71% 36%;
  --success-foreground: 0 0% 98%;
  --warning: 38 92% 50%;
  --warning-foreground: 0 0% 98%;
  --info: 220 83% 53%;
  --info-foreground: 0 0% 98%;

  --radius: 0.375rem; /* 6px — design uses 6px buttons, 8px cards, 4px chips */
}

@theme {
  /* Font stacks — Inter-first per design (system fallbacks for offline) */
  --font-family-sans: ui-sans-serif, system-ui, -apple-system,
    BlinkMacSystemFont, "Segoe UI", Inter, Roboto, "Helvetica Neue", sans-serif;
  --font-family-mono: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
    "Liberation Mono", monospace;

  /* shadcn-svelte semantic color tokens mapped to Tailwind v4 */
  --color-background: hsl(var(--background));
  --color-foreground: hsl(var(--foreground));
  --color-card: hsl(var(--card));
  --color-card-foreground: hsl(var(--card-foreground));
  --color-popover: hsl(var(--popover));
  --color-popover-foreground: hsl(var(--popover-foreground));
  --color-primary: hsl(var(--primary));
  --color-primary-foreground: hsl(var(--primary-foreground));
  --color-secondary: hsl(var(--secondary));
  --color-secondary-foreground: hsl(var(--secondary-foreground));
  --color-muted: hsl(var(--muted));
  --color-muted-foreground: hsl(var(--muted-foreground));
  --color-accent: hsl(var(--accent));
  --color-accent-foreground: hsl(var(--accent-foreground));
  --color-destructive: hsl(var(--destructive));
  --color-destructive-foreground: hsl(var(--destructive-foreground));
  --color-border: hsl(var(--border));
  --color-input: hsl(var(--input));
  --color-ring: hsl(var(--ring));

  /* Extended tokens */
  --color-sidebar-bg: hsl(var(--sidebar-bg));
  --color-panel-bg: hsl(var(--panel-bg));
  --color-border-muted: hsl(var(--border-muted));
  --color-hover: hsl(var(--hover));
  --color-muted-fg-2: hsl(var(--muted-fg-2));
  --color-success: hsl(var(--success));
  --color-success-foreground: hsl(var(--success-foreground));
  --color-warning: hsl(var(--warning));
  --color-warning-foreground: hsl(var(--warning-foreground));
  --color-info: hsl(var(--info));
  --color-info-foreground: hsl(var(--info-foreground));

  --radius: 0.375rem;
}

@layer base {
  html,
  body {
    @apply h-full;
    font-family: var(--font-family-sans);
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    color: hsl(var(--foreground));
    background: hsl(var(--background));
  }

  * {
    box-sizing: border-box;
  }

  /* Tabular numbers everywhere counts/IDs are shown */
  .tabular-nums {
    font-variant-numeric: tabular-nums;
  }
}
```

- [ ] **Step 2: Verify the dev server still compiles**

Run: `cd "/Users/danielguimaraes/Work/10-19 Development/13 WNP/wnp-app" && bun run check`
Expected: PASS (or only warnings, no Tailwind/CSS errors). If `bun run check` doesn't exist, run `bun run build` and verify it completes.

- [ ] **Step 3: Commit**

```bash
git add src/app.css
git commit -m "Switch design tokens to zinc neutrals from Claude Design handoff

Replace slate-blue palette with zinc-based palette to match
_design/wnp-app/project/lib-shadcn.jsx. Adds extended tokens
(sidebar-bg, panel-bg, border-muted, hover, success, warning,
info) so subsequent UI work can reference them by name. Switches
font stack to Inter-first."
```

---

### Task 2: Bump Tauri window minimum width to 1440

**Files:**
- Modify: `src-tauri/tauri.conf.json:34-39` (window section)

**Why:** Design assumes 1440×900 macOS window with sidebar (248) + main + right inspector (400) — anything narrower compresses uncomfortably. User confirmed minimum 1440 is the target.

- [ ] **Step 1: Read the current window section**

Run: `grep -n -A 8 '"windows"' "/Users/danielguimaraes/Work/10-19 Development/13 WNP/wnp-app/src-tauri/tauri.conf.json"`
Expected: Block with `"width": 1400, "height": 900, "minWidth": 900, "minHeight": 600`.

- [ ] **Step 2: Edit window dimensions**

Use the Edit tool on `src-tauri/tauri.conf.json`:
- old_string: `"width": 1400,`  →  new_string: `"width": 1440,`
- old_string: `"minWidth": 900,` →  new_string: `"minWidth": 1440,`
- old_string: `"minHeight": 600` →  new_string: `"minHeight": 900`

- [ ] **Step 3: Verify the change**

Run: `grep -A 6 '"width"' "/Users/danielguimaraes/Work/10-19 Development/13 WNP/wnp-app/src-tauri/tauri.conf.json"`
Expected: width 1440, minWidth 1440, height 900, minHeight 900.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "Set Tauri window to 1440x900 minimum

Design is targeted at 1440 width; sidebar+main+inspector layouts
compress unusably below that. Bumps default and minimum to match."
```

---

### Task 3: Add `Kbd` and `KbdSeq` primitives

**Files:**
- Create: `src/lib/components/ui/kbd/Kbd.svelte`
- Create: `src/lib/components/ui/kbd/KbdSeq.svelte`
- Create: `src/lib/components/ui/kbd/index.ts`

**Why:** Design uses keyboard-shortcut pills throughout (sidebar nav, command bar, page headers, modal footers). Reference: `lib-shadcn.jsx:86-106`.

- [ ] **Step 1: Create `Kbd.svelte`**

Write to `src/lib/components/ui/kbd/Kbd.svelte`:

```svelte
<script lang="ts">
  import { cn } from "$lib/utils";

  interface Props {
    dim?: boolean;
    class?: string;
    children: import("svelte").Snippet;
  }

  let { dim = false, class: className = "", children }: Props = $props();
</script>

<span
  class={cn(
    "inline-flex items-center justify-center font-mono text-[10px] font-medium",
    "px-[5px] py-[1px] min-w-[16px] rounded border bg-background",
    "border-border leading-[1.5]",
    dim ? "text-muted-foreground" : "text-muted-fg-2",
    className,
  )}
  style="box-shadow: 0 1px 0 rgba(0,0,0,.03);"
>
  {@render children()}
</span>
```

- [ ] **Step 2: Create `KbdSeq.svelte`**

Write to `src/lib/components/ui/kbd/KbdSeq.svelte`:

```svelte
<script lang="ts">
  import Kbd from "./Kbd.svelte";

  interface Props {
    keys: string[];
    dim?: boolean;
  }

  let { keys, dim = false }: Props = $props();
</script>

<span class="inline-flex gap-[3px]">
  {#each keys as key}
    <Kbd {dim}>{key}</Kbd>
  {/each}
</span>
```

- [ ] **Step 3: Create `index.ts`**

Write to `src/lib/components/ui/kbd/index.ts`:

```ts
export { default as Kbd } from "./Kbd.svelte";
export { default as KbdSeq } from "./KbdSeq.svelte";
```

- [ ] **Step 4: Verify it compiles**

Run: `cd "/Users/danielguimaraes/Work/10-19 Development/13 WNP/wnp-app" && bun run check`
Expected: no new errors related to the kbd files.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/ui/kbd/
git commit -m "Add Kbd and KbdSeq primitives

Keyboard-shortcut pills used throughout the design (sidebar nav,
command bar, modal footers). KbdSeq renders multi-key sequences
like ⌘ K with consistent spacing."
```

---

### Task 4: Add `FilterChip` primitive

**Files:**
- Create: `src/lib/components/ui/filter-chip/FilterChip.svelte`
- Create: `src/lib/components/ui/filter-chip/index.ts`

**Why:** Active filters are shown as removable pills in the design's library view ("City: San Francisco · ✕"). Reference: `lib-shadcn.jsx:204-222`.

- [ ] **Step 1: Create `FilterChip.svelte`**

Write to `src/lib/components/ui/filter-chip/FilterChip.svelte`:

```svelte
<script lang="ts">
  import X from "@lucide/svelte/icons/x";

  interface Props {
    field: string;
    value: string;
    onRemove?: () => void;
  }

  let { field, value, onRemove }: Props = $props();
</script>

<span
  class="inline-flex items-center h-[26px] rounded-[4px] overflow-hidden border border-border text-xs bg-background"
>
  <span
    class="px-2 text-muted-foreground bg-sidebar-bg border-r border-border h-full flex items-center"
  >
    {field}
  </span>
  <span
    class="px-2 text-foreground font-medium flex items-center h-full"
  >
    {value}
  </span>
  <button
    type="button"
    class="px-[6px] pr-[6px] pl-[2px] text-muted-foreground flex items-center h-full hover:text-foreground"
    onclick={onRemove}
    aria-label="Remove {field} filter"
  >
    <X size={11} />
  </button>
</span>
```

- [ ] **Step 2: Create `index.ts`**

Write to `src/lib/components/ui/filter-chip/index.ts`:

```ts
export { default as FilterChip } from "./FilterChip.svelte";
```

- [ ] **Step 3: Verify compile**

Run: `bun run check`
Expected: no new errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/ui/filter-chip/
git commit -m "Add FilterChip primitive

Removable pill for active library filters ('City: San Francisco ×').
Used in the redesigned filter row above the grid."
```

---

### Task 5: Add `PageHeader` primitive

**Files:**
- Create: `src/lib/components/ui/page-header/PageHeader.svelte`
- Create: `src/lib/components/ui/page-header/index.ts`

**Why:** The design replaces the current 48px `TopBar` with a 56px `PageHeader` that combines title + count + subtitle + right-side actions. Used by every main view. Reference: `lib-shadcn.jsx:436-455`.

- [ ] **Step 1: Create `PageHeader.svelte`**

Write to `src/lib/components/ui/page-header/PageHeader.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    count?: number | string;
    subtitle?: string;
    right?: Snippet;
  }

  let { title, count, subtitle, right }: Props = $props();

  let countLabel = $derived(
    typeof count === "number" ? `${count.toLocaleString()} items` : count,
  );
</script>

<div
  class="h-14 px-5 flex items-center gap-3 border-b border-border bg-background flex-shrink-0"
>
  <div class="flex items-baseline gap-[10px]">
    <div class="text-base font-semibold text-foreground tracking-[-0.2px]">
      {title}
    </div>
    {#if count !== undefined}
      <div class="text-xs text-muted-foreground tabular-nums">
        {countLabel}
      </div>
    {/if}
    {#if subtitle}
      <div class="text-xs text-muted-foreground">{subtitle}</div>
    {/if}
  </div>
  <div class="flex-1"></div>
  {#if right}
    {@render right()}
  {/if}
</div>
```

- [ ] **Step 2: Create `index.ts`**

Write to `src/lib/components/ui/page-header/index.ts`:

```ts
export { default as PageHeader } from "./PageHeader.svelte";
```

- [ ] **Step 3: Verify compile**

Run: `bun run check`
Expected: no new errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/ui/page-header/
git commit -m "Add PageHeader primitive

56px header combining title + count + subtitle + right-side
actions snippet. Replaces existing TopBar in subsequent view
refactors."
```

---

### Task 6: Add `StatusBar` primitive

**Files:**
- Create: `src/lib/components/ui/status-bar/StatusBar.svelte`
- Create: `src/lib/components/ui/status-bar/index.ts`

**Why:** 28px persistent footer showing selection counts, drive status, shortcut hints. Reference: `lib-shadcn.jsx:424-432`.

- [ ] **Step 1: Create `StatusBar.svelte`**

Write to `src/lib/components/ui/status-bar/StatusBar.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();
</script>

<div
  class="h-7 border-t border-border flex items-center px-4 text-[11px] text-muted-foreground gap-4 bg-sidebar-bg tabular-nums flex-shrink-0"
>
  {@render children()}
</div>
```

- [ ] **Step 2: Create `index.ts`**

Write to `src/lib/components/ui/status-bar/index.ts`:

```ts
export { default as StatusBar } from "./StatusBar.svelte";
```

- [ ] **Step 3: Verify compile**

Run: `bun run check`
Expected: no new errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/ui/status-bar/
git commit -m "Add StatusBar primitive

28px persistent footer for selection counts, drive status, and
shortcut hints. Slot-based so each view fills it differently."
```

---

### Task 7: Add `get_recent_activity` Rust command + TS wrapper

**Files:**
- Modify: `src-tauri/src/models.rs` (add struct)
- Modify: `src-tauri/src/editor.rs` (add command, after existing `get_audit_log` at line ~114)
- Modify: `src-tauri/src/lib.rs:52` (register command in invoke_handler)
- Create: `src/lib/commands/activity.ts`

**Why:** `ActivityCard` (Task 8) needs the most recent N audit-log entries across **all** images, joined with the `images` table to get each entry's catalog number for display. The existing `get_audit_log(image_id)` is per-image — we need a global variant.

- [ ] **Step 1: Add `RecentActivityEntry` struct in `models.rs`**

Add to `src-tauri/src/models.rs` after the existing `AuditLogEntry` struct (find it via the comment "A single audit log entry" at line ~145):

```rust
/// A single recent activity entry for the sidebar ActivityCard.
/// Joined view of audit_log + images so the card can display the
/// catalog number alongside the field that changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentActivityEntry {
    pub id: i64,
    pub catalog_number: String,
    pub field_name: String,
    pub new_value: Option<String>,
    pub changed_at: String,
}
```

- [ ] **Step 2: Add the `get_recent_activity` command in `editor.rs`**

Append to `src-tauri/src/editor.rs` (after the `get_audit_log` function ending at ~line 114, before `log_image_view` at ~242):

```rust
/// Fetch the most recent audit-log entries across all images, joined
/// with the images table for the catalog number. Used by the sidebar
/// ActivityCard.
#[tauri::command]
pub fn get_recent_activity(
    limit: i64,
    state: tauri::State<AppState>,
) -> Result<Vec<RecentActivityEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT a.id, i.catalog_number, a.field_name, a.new_value, a.changed_at
             FROM audit_log a
             JOIN images i ON i.id = a.image_id
             ORDER BY a.changed_at DESC
             LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(rusqlite::params![limit], |row| {
            Ok(RecentActivityEntry {
                id: row.get(0)?,
                catalog_number: row.get(1)?,
                field_name: row.get(2)?,
                new_value: row.get(3)?,
                changed_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let entries: Vec<RecentActivityEntry> = rows.filter_map(|r| r.ok()).collect();
    Ok(entries)
}
```

Also confirm/add the import at the top of `editor.rs`:
```rust
use crate::models::{AuditLogEntry, RecentActivityEntry};
```
(Edit existing `use crate::models::{AuditLogEntry};` to include `RecentActivityEntry`.)

- [ ] **Step 3: Register the command in `lib.rs`**

Find `editor::get_audit_log,` in `src-tauri/src/lib.rs` (around line 52). Add `editor::get_recent_activity,` immediately after it. Both lines should sit inside the `tauri::generate_handler![...]` macro list.

- [ ] **Step 4: Verify Rust compiles**

Run: `cd "/Users/danielguimaraes/Work/10-19 Development/13 WNP/wnp-app/src-tauri" && cargo check`
Expected: PASS (warnings OK).

- [ ] **Step 5: Create the TS wrapper `src/lib/commands/activity.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";

export interface RecentActivityEntry {
  id: number;
  catalog_number: string;
  field_name: string;
  new_value: string | null;
  changed_at: string;
}

export async function getRecentActivity(
  limit = 5,
): Promise<RecentActivityEntry[]> {
  return invoke<RecentActivityEntry[]>("get_recent_activity", { limit });
}
```

- [ ] **Step 6: Verify TS compiles**

Run: `cd "/Users/danielguimaraes/Work/10-19 Development/13 WNP/wnp-app" && bun run check`
Expected: no new errors.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/editor.rs src-tauri/src/lib.rs src/lib/commands/activity.ts
git commit -m "Add get_recent_activity command for sidebar ActivityCard

Returns the N most recent audit_log entries joined with images
so the ActivityCard can show 'Edited city in wnp27.4283 · 2m ago'
without an extra round-trip per row."
```

---

### Task 8: Add `ActivityCard` component (single-user)

**Files:**
- Create: `src/lib/components/layout/sidebar/ActivityCard.svelte`

**Why:** Bottom-of-sidebar Apple-style card showing recent edits. Design's version has a "who" column for multi-user; we drop that for single-user and lead with the verb instead. Reference: `lib-shadcn.jsx:273-306`.

- [ ] **Step 1: Create `ActivityCard.svelte`**

Write to `src/lib/components/layout/sidebar/ActivityCard.svelte`:

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import Activity from "@lucide/svelte/icons/activity";
  import { getRecentActivity, type RecentActivityEntry } from "$lib/commands/activity";
  import { formatRelativeTime } from "$lib/utils/format";

  let entries: RecentActivityEntry[] = $state([]);

  onMount(async () => {
    try {
      entries = await getRecentActivity(3);
    } catch (e) {
      console.error("Failed to load recent activity", e);
    }
  });
</script>

<div
  class="mx-[10px] mt-2 mb-3 p-[10px] rounded-lg bg-background border border-border text-[11px] text-muted-fg-2"
>
  <div class="flex items-center gap-[6px] mb-2 font-semibold text-foreground text-[11.5px]">
    <Activity size={11} />
    <span>Recent activity</span>
    <div class="flex-1"></div>
    <span class="font-normal text-[10.5px] text-muted-foreground">See all</span>
  </div>

  {#if entries.length === 0}
    <div class="text-muted-foreground text-[11px] py-1">No recent edits.</div>
  {:else}
    {#each entries as entry, i (entry.id)}
      <div
        class="leading-[1.4] {i < entries.length - 1
          ? 'mb-2 pb-2 border-b border-border-muted'
          : ''}"
      >
        <div class="text-foreground font-medium text-[11.5px]">
          Edited <span class="text-muted-foreground">{entry.field_name}</span>
        </div>
        <div class="text-[#52525b] font-mono text-[10.5px] mt-[2px]">
          {entry.catalog_number}
        </div>
        <div class="text-muted-foreground text-[10.5px] mt-[1px]">
          {formatRelativeTime(entry.changed_at)}
        </div>
      </div>
    {/each}
  {/if}
</div>
```

- [ ] **Step 2: Confirm `formatRelativeTime` exists**

Run: `grep -n "formatRelativeTime" "/Users/danielguimaraes/Work/10-19 Development/13 WNP/wnp-app/src/lib/utils/format.ts"`
Expected: function definition.
**If it does not exist**, add it to `src/lib/utils/format.ts`:

```ts
/** Returns "2m ago", "3h ago", "Apr 22", etc. for an ISO/SQL timestamp. */
export function formatRelativeTime(iso: string): string {
  const then = new Date(iso.includes("T") ? iso : iso.replace(" ", "T") + "Z");
  const seconds = Math.floor((Date.now() - then.getTime()) / 1000);
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d ago`;
  return then.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}
```

- [ ] **Step 3: Verify compile**

Run: `bun run check`
Expected: no new errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/layout/sidebar/ActivityCard.svelte src/lib/utils/format.ts
git commit -m "Add ActivityCard component for sidebar (single-user)

Pulls the three most recent audit-log edits and shows field +
catalog + relative time. Adapted from the design's multi-user
card by dropping the actor column."
```

---

### Task 9: Add `SideGroup` and `SideItem` primitives (sidebar internals)

**Files:**
- Create: `src/lib/components/layout/sidebar/SideGroup.svelte`
- Create: `src/lib/components/layout/sidebar/SideItem.svelte`

**Why:** The design's sidebar is built from two repeated atoms — group headers (with optional action button) and clickable items (with optional icon, count, badge, kbd hint, selected state). Reference: `lib-shadcn.jsx:227-270`.

- [ ] **Step 1: Create `SideGroup.svelte`**

```svelte
<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    action?: Snippet;
    children: Snippet;
  }

  let { title, action, children }: Props = $props();
</script>

<div class="mb-[14px]">
  <div
    class="flex items-center justify-between px-[14px] pb-[6px] text-[11px] font-medium tracking-[0.3px] text-muted-foreground uppercase"
  >
    <span>{title}</span>
    {#if action}
      <span class="cursor-pointer normal-case tracking-normal text-xs text-muted-foreground flex items-center">
        {@render action()}
      </span>
    {/if}
  </div>
  <div>
    {@render children()}
  </div>
</div>
```

- [ ] **Step 2: Create `SideItem.svelte`**

```svelte
<script lang="ts">
  import type { Snippet } from "svelte";
  import { Kbd } from "$lib/components/ui/kbd";

  interface Props {
    icon?: Snippet;
    label: string;
    count?: number;
    badge?: number | string;
    kbd?: string;
    selected?: boolean;
    onclick?: () => void;
  }

  let {
    icon,
    label,
    count,
    badge,
    kbd,
    selected = false,
    onclick,
  }: Props = $props();
</script>

<button
  type="button"
  {onclick}
  class="w-[calc(100%-16px)] flex items-center gap-[10px] h-[30px] pl-3 pr-[10px] mx-2 rounded-md text-[13px] text-left transition-colors
    {selected
      ? 'bg-secondary text-foreground font-medium'
      : 'text-muted-fg-2 hover:bg-hover'}"
>
  {#if icon}
    <span class="flex {selected ? 'text-foreground' : 'text-muted-foreground'}">
      {@render icon()}
    </span>
  {/if}
  <span class="flex-1 overflow-hidden text-ellipsis whitespace-nowrap">
    {label}
  </span>
  {#if badge !== undefined}
    <span class="text-[10px] font-semibold px-[6px] py-[1px] rounded-[10px] bg-primary text-primary-foreground leading-[1.4]">
      {badge}
    </span>
  {:else if count !== undefined}
    <span class="text-[11px] text-muted-foreground tabular-nums">
      {count.toLocaleString()}
    </span>
  {/if}
  {#if kbd}
    <Kbd dim>{kbd}</Kbd>
  {/if}
</button>
```

- [ ] **Step 3: Verify compile**

Run: `bun run check`
Expected: no new errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/layout/sidebar/SideGroup.svelte src/lib/components/layout/sidebar/SideItem.svelte
git commit -m "Add SideGroup and SideItem sidebar atoms

Group renders an uppercase tracking-wide header with optional
action snippet. Item renders an icon-label-count/badge-kbd row
with selected/hover states. Used in the refactored Sidebar."
```

---

### Task 10: Extend `ViewType` and add stub views

**Files:**
- Modify: `src/lib/stores/navigation.ts:5` (extend the union)
- Create: `src/lib/components/stubs/StubView.svelte`
- Create: `src/lib/components/stubs/AuditLogStub.svelte`
- Create: `src/lib/components/stubs/BackupsStub.svelte`
- Create: `src/lib/components/stubs/ImportInboxStub.svelte`
- Create: `src/lib/components/stubs/MissingMetadataStub.svelte`
- Create: `src/lib/components/stubs/SmartCollectionsStub.svelte`

**Why:** Sidebar will link to all of these in the next task. Without stubs the router falls through and the items dead-end.

- [ ] **Step 1: Extend `ViewType` in `navigation.ts`**

Use Edit on `src/lib/stores/navigation.ts`:
- old_string: `export type ViewType = 'setup' | 'import' | 'library' | 'detail' | 'collection' | 'requests' | 'settings';`
- new_string: `export type ViewType = 'setup' | 'import' | 'library' | 'detail' | 'collection' | 'requests' | 'settings' | 'audit' | 'backups' | 'inbox' | 'missing' | 'smart-collections';`

- [ ] **Step 2: Create the shared `StubView.svelte`**

```svelte
<script lang="ts">
  import { PageHeader } from "$lib/components/ui/page-header";
  import { StatusBar } from "$lib/components/ui/status-bar";
  import Construction from "@lucide/svelte/icons/construction";

  interface Props {
    title: string;
    description: string;
  }

  let { title, description }: Props = $props();
</script>

<div class="flex-1 flex flex-col min-w-0">
  <PageHeader {title} subtitle="Coming soon" />

  <div class="flex-1 flex items-center justify-center bg-sidebar-bg p-10">
    <div
      class="w-[440px] bg-background rounded-lg border border-border p-7 text-center"
      style="box-shadow: 0 10px 40px rgba(0,0,0,0.04);"
    >
      <div
        class="w-12 h-12 rounded-lg mx-auto mb-[14px] bg-secondary border border-border flex items-center justify-center text-muted-foreground"
      >
        <Construction size={22} />
      </div>
      <div class="text-lg font-semibold text-foreground tracking-[-0.2px] mb-[6px]">
        {title}
      </div>
      <div class="text-[13.5px] text-muted-fg-2 leading-[1.55] max-w-[340px] mx-auto">
        {description}
      </div>
    </div>
  </div>

  <StatusBar>
    <span>Stub view · not yet implemented</span>
    <div class="flex-1"></div>
  </StatusBar>
</div>
```

- [ ] **Step 3: Create the five concrete stubs**

`src/lib/components/stubs/AuditLogStub.svelte`:
```svelte
<script lang="ts">
  import StubView from "./StubView.svelte";
</script>

<StubView
  title="Audit log"
  description="A team-wide log of every metadata change, approval, and import — with before/after diffs and per-user filters. Coming in a future build."
/>
```

`src/lib/components/stubs/BackupsStub.svelte`:
```svelte
<script lang="ts">
  import StubView from "./StubView.svelte";
</script>

<StubView
  title="Backups"
  description="Backblaze B2 backup monitoring — live upload progress, 30-day activity, failures, and schedule. Coming in a future build."
/>
```

`src/lib/components/stubs/ImportInboxStub.svelte`:
```svelte
<script lang="ts">
  import StubView from "./StubView.svelte";
</script>

<StubView
  title="Import inbox"
  description="Watches /_inbox/ on the archive drive and surfaces newly added images in per-batch groups for triage. Coming in a future build."
/>
```

`src/lib/components/stubs/MissingMetadataStub.svelte`:
```svelte
<script lang="ts">
  import StubView from "./StubView.svelte";
</script>

<StubView
  title="Missing metadata"
  description="A focused view of every image with empty required fields, ready for triage. Coming in a future build."
/>
```

`src/lib/components/stubs/SmartCollectionsStub.svelte`:
```svelte
<script lang="ts">
  import StubView from "./StubView.svelte";
</script>

<StubView
  title="Smart collections"
  description="Saved filter queries that auto-update as the catalog changes. Coming in a future build."
/>
```

- [ ] **Step 4: Verify compile**

Run: `bun run check`
Expected: no new errors.

- [ ] **Step 5: Commit**

```bash
git add src/lib/stores/navigation.ts src/lib/components/stubs/
git commit -m "Add stub views for not-yet-built sidebar destinations

Audit log, Backups, Import inbox, Missing metadata, and Smart
collections each get a placeholder view that uses the new
PageHeader and StatusBar primitives. Extends ViewType so the
router can switch on them in the next task."
```

---

### Task 11: Refactor `Sidebar.svelte` to match design

**Files:**
- Modify: `src/lib/components/layout/Sidebar.svelte` (full rewrite, currently 248 lines)

**Why:** Design's sidebar is structurally different from current: 248px wide (was 220), brand+logo header (was just a title), inert ⌘K launcher row, six grouped nav sections, ActivityCard pinned to bottom. Existing collections/recently-viewed logic moves into the new groups.

- [ ] **Step 1: Read the current sidebar**

Read `src/lib/components/layout/Sidebar.svelte` in full so you can map current props/handlers to the new structure (refresh handlers for collections, etc.).

- [ ] **Step 2: Replace the file with the new sidebar**

Write to `src/lib/components/layout/Sidebar.svelte`:

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { currentView, currentCollectionId } from "$lib/stores/navigation";
  import { userCollections, refreshUserCollections } from "$lib/stores/collections";
  import { ordersResponse } from "$lib/stores/orders";
  import { getCollections } from "$lib/commands/collections";
  import { Kbd, KbdSeq } from "$lib/components/ui/kbd";
  import SideGroup from "./sidebar/SideGroup.svelte";
  import SideItem from "./sidebar/SideItem.svelte";
  import ActivityCard from "./sidebar/ActivityCard.svelte";

  // Lucide icons
  import Search from "@lucide/svelte/icons/search";
  import Inbox from "@lucide/svelte/icons/inbox";
  import Upload from "@lucide/svelte/icons/upload";
  import AlertCircle from "@lucide/svelte/icons/alert-circle";
  import AlignJustify from "@lucide/svelte/icons/align-justify";
  import Clock from "@lucide/svelte/icons/clock";
  import Folder from "@lucide/svelte/icons/folder";
  import Star from "@lucide/svelte/icons/star";
  import Filter from "@lucide/svelte/icons/filter";
  import Cloud from "@lucide/svelte/icons/cloud";
  import History from "@lucide/svelte/icons/history";
  import Plus from "@lucide/svelte/icons/plus";

  interface ArchiveCollection {
    id: number;
    name: string;
    image_count: number;
  }

  let archiveCollections: ArchiveCollection[] = $state([]);

  onMount(async () => {
    await refreshUserCollections();
    const all = await getCollections();
    archiveCollections = all.filter((c: any) => c.source === "archive");
  });

  function go(view: string, collectionId: number | null = null) {
    currentView.set(view as any);
    currentCollectionId.set(collectionId);
  }

  let pendingCount = $derived($ordersResponse?.meta?.fulfillable ?? 0);
</script>

<aside
  class="w-[248px] flex-shrink-0 bg-sidebar-bg border-r border-border flex flex-col overflow-hidden"
>
  <!-- Brand header -->
  <div
    class="h-[52px] px-4 flex items-center gap-[10px] border-b border-border"
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

  <!-- ⌘K launcher (inert until command bar plan lands) -->
  <div class="p-[10px] pb-[6px]">
    <button
      type="button"
      disabled
      class="w-full flex items-center gap-2 h-[30px] px-[10px] rounded-md bg-background border border-border text-muted-foreground text-[12.5px] cursor-not-allowed opacity-90"
      title="Command bar — coming soon"
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
        label="Pending requests"
        badge={pendingCount > 0 ? pendingCount : undefined}
        selected={$currentView === "requests"}
        kbd="G Q"
        onclick={() => go("requests")}
      >
        {#snippet icon()}<Inbox size={14} />{/snippet}
      </SideItem>
      <SideItem
        label="Import inbox"
        selected={$currentView === "inbox"}
        kbd="G I"
        onclick={() => go("inbox")}
      >
        {#snippet icon()}<Upload size={14} />{/snippet}
      </SideItem>
      <SideItem
        label="Missing metadata"
        selected={$currentView === "missing"}
        onclick={() => go("missing")}
      >
        {#snippet icon()}<AlertCircle size={14} />{/snippet}
      </SideItem>
    </SideGroup>

    <SideGroup title="Library">
      <SideItem
        label="All images"
        selected={$currentView === "library" && $currentCollectionId === null}
        kbd="G A"
        onclick={() => go("library", null)}
      >
        {#snippet icon()}<AlignJustify size={14} />{/snippet}
      </SideItem>
      <SideItem
        label="Recently viewed"
        kbd="G R"
        onclick={() => go("library", -1)}
      >
        {#snippet icon()}<Clock size={14} />{/snippet}
      </SideItem>
    </SideGroup>

    <SideGroup title="Archive Collections">
      {#each archiveCollections as c (c.id)}
        <SideItem
          label={c.name}
          count={c.image_count}
          selected={$currentCollectionId === c.id}
          onclick={() => go("library", c.id)}
        >
          {#snippet icon()}<Folder size={14} />{/snippet}
        </SideItem>
      {/each}
    </SideGroup>

    <SideGroup title="Collections">
      {#snippet action()}<Plus size={13} />{/snippet}
      {#each $userCollections as c (c.id)}
        <SideItem
          label={c.name}
          count={c.image_count}
          selected={$currentCollectionId === c.id}
          onclick={() => go("library", c.id)}
        >
          {#snippet icon()}<Star size={14} />{/snippet}
        </SideItem>
      {/each}
    </SideGroup>

    <SideGroup title="Smart Collections">
      {#snippet action()}<Plus size={13} />{/snippet}
      <SideItem
        label="Smart collections"
        selected={$currentView === "smart-collections"}
        onclick={() => go("smart-collections")}
      >
        {#snippet icon()}<Filter size={14} />{/snippet}
      </SideItem>
    </SideGroup>

    <SideGroup title="Analytics">
      <SideItem
        label="Backups"
        selected={$currentView === "backups"}
        kbd="G B"
        onclick={() => go("backups")}
      >
        {#snippet icon()}<Cloud size={14} />{/snippet}
      </SideItem>
      <SideItem
        label="Audit log"
        selected={$currentView === "audit"}
        kbd="G L"
        onclick={() => go("audit")}
      >
        {#snippet icon()}<History size={14} />{/snippet}
      </SideItem>
    </SideGroup>
  </div>

  <ActivityCard />
</aside>
```

- [ ] **Step 3: Verify the import paths resolve**

Run: `grep -n "stores/orders\|stores/collections\|commands/collections" "/Users/danielguimaraes/Work/10-19 Development/13 WNP/wnp-app/src/lib/stores/" "/Users/danielguimaraes/Work/10-19 Development/13 WNP/wnp-app/src/lib/commands/" 2>/dev/null || ls "/Users/danielguimaraes/Work/10-19 Development/13 WNP/wnp-app/src/lib/stores/" "/Users/danielguimaraes/Work/10-19 Development/13 WNP/wnp-app/src/lib/commands/"`

Expected: confirm files exist with the names referenced. **If `ordersResponse` lives under a different name** (the survey reported `$ordersResponse` from `stores/orders`), update the import accordingly. **If `getCollections` exposes a different `source` property name**, adapt the `archive` filter to match the actual TS type.

- [ ] **Step 4: Verify compile**

Run: `bun run check`
Expected: no new errors. Address any import-name mismatches surfaced.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/layout/Sidebar.svelte
git commit -m "Refactor Sidebar to match Claude Design layout

Width 248px, brand+logo header, inert ⌘K launcher, six grouped
nav sections (Actions/Library/Archive Collections/Collections/
Smart Collections/Analytics), ActivityCard pinned to bottom.
Stub destinations (audit/backups/inbox/missing/smart-collections)
route to placeholder views added in the previous task."
```

---

### Task 12: Wire the new view types into the router

**Files:**
- Modify: `src/routes/+page.svelte:1-119`

**Why:** Sidebar items now dispatch to `'audit' | 'backups' | 'inbox' | 'missing' | 'smart-collections'`, but the router only switches on the original ViewType values. Without this task, clicking those items shows a blank pane.

- [ ] **Step 1: Read the current router**

Read `src/routes/+page.svelte` to find the existing view switch (likely an `{#if}/{:else if}` chain on `$currentView`).

- [ ] **Step 2: Add imports for the stub views**

In the `<script>` block, add (keeping existing imports):

```ts
import AuditLogStub from "$lib/components/stubs/AuditLogStub.svelte";
import BackupsStub from "$lib/components/stubs/BackupsStub.svelte";
import ImportInboxStub from "$lib/components/stubs/ImportInboxStub.svelte";
import MissingMetadataStub from "$lib/components/stubs/MissingMetadataStub.svelte";
import SmartCollectionsStub from "$lib/components/stubs/SmartCollectionsStub.svelte";
```

- [ ] **Step 3: Add the new branches to the view switch**

In the `{#if}/{:else if}` chain, before the final `{:else}` (or before the existing `library` branch — placement is cosmetic), add:

```svelte
{:else if $currentView === 'audit'}
  <AuditLogStub />
{:else if $currentView === 'backups'}
  <BackupsStub />
{:else if $currentView === 'inbox'}
  <ImportInboxStub />
{:else if $currentView === 'missing'}
  <MissingMetadataStub />
{:else if $currentView === 'smart-collections'}
  <SmartCollectionsStub />
```

The stubs are sized to fit a flex column already (they have their own `flex-1`), so they should slot into the existing `<div class="flex h-screen">` layout next to `<Sidebar />` without further changes.

- [ ] **Step 4: Verify compile**

Run: `bun run check`
Expected: no new errors.

- [ ] **Step 5: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "Route the new sidebar destinations to their stub views"
```

---

### Task 13: Visual verification & memory update

**Files:**
- (no edits)

**Why:** Catch regressions the type-checker can't see — wrong colors, broken layouts, missing icons, sidebar overflow at minimum window width.

- [ ] **Step 1: Start the dev server**

Run: `cd "/Users/danielguimaraes/Work/10-19 Development/13 WNP/wnp-app" && bun run tauri dev`
Wait for the Tauri window to open. Resize it to exactly 1440×900 if possible.

- [ ] **Step 2: Verify the library view**

Confirm: sidebar is 248px wide; brand row has logo+title; ⌘K launcher row visible (disabled); the three Actions items render with icons + kbd hints; archive collections render with counts; user collections render with star icons; "Smart collections" entry visible; backups/audit visible at bottom; ActivityCard renders below all groups (or shows "No recent edits" if database is empty); no blue colors anywhere — only zinc neutrals.

- [ ] **Step 3: Click each stub destination**

For each of: Pending requests (existing), Import inbox, Missing metadata, Smart collections, Backups, Audit log — click and confirm the centered "Coming soon" card renders with PageHeader at top and StatusBar at bottom. Use the back-to-library nav (sidebar "All images") to return.

- [ ] **Step 4: Window resize check**

Try to drag the window narrower than 1440px. Confirm the OS prevents it (minWidth enforced).

- [ ] **Step 5: Update auto-memory**

Update the project memory file at `/Users/danielguimaraes/.claude/projects/-Users-danielguimaraes-Work-10-19-Development-13-WNP-wnp-app/memory/MEMORY.md` and the corresponding files:

- Update the Phase 4 status note to add: "**Phase 5 (in progress)**: Design system refactor (zinc tokens, Kbd/FilterChip/PageHeader/StatusBar primitives, redesigned sidebar)."
- Fix the "outsidelands.org" reference → "OpenSFHistory" (it's the public archive site, built on Laravel; we already integrate with its API for orders).
- Add note: "Backblaze B2 is the planned S3-compatible provider for image fulfillment uploads."
- Add note: "Multi-user is future; activity card and audit log are single-user for now."

- [ ] **Step 6: Stop the dev server and commit any verification artifacts**

If the manual checks all pass, no further commit is needed (everything was committed in earlier tasks). If you discovered a small fix during verification, make a focused commit for it.

---

## Self-review

**Spec coverage:**
- ✅ Design tokens replaced (Task 1)
- ✅ Tauri minWidth → 1440 (Task 2)
- ✅ Kbd/KbdSeq primitives (Task 3)
- ✅ FilterChip (Task 4)
- ✅ PageHeader (Task 5)
- ✅ StatusBar (Task 6)
- ✅ Backend command + ActivityCard (Tasks 7–8)
- ✅ SideGroup/SideItem (Task 9)
- ✅ Stub views + ViewType extension (Task 10)
- ✅ Sidebar refactored (Task 11)
- ✅ Router wired (Task 12)
- ✅ Visual verification + memory updates (Task 13)

**Dependencies between tasks:**
- Task 8 depends on 7 (uses the new TS wrapper)
- Task 9 depends on 3 (`SideItem` imports `Kbd`)
- Task 10 depends on 5+6 (`StubView` imports `PageHeader`/`StatusBar`)
- Task 11 depends on 8+9 (uses `ActivityCard`, `SideGroup`, `SideItem`)
- Task 12 depends on 10 (imports stub views)
- Task 13 depends on everything

**Type consistency:** `ViewType` extended in Task 10 includes the exact strings used by `currentView.set(...)` calls in the Sidebar (Task 11) and the `{#if}` branches in the router (Task 12).

**Excluded scope:** Library/Detail/Requests/Settings view refactors are deliberately deferred to a follow-up plan so this one can ship cleanly. The new primitives are unused by main views after this plan — that's intentional; they get adopted in the next plan. Sidebar still works for the existing main views (library/detail/requests/settings) because their components don't depend on the sidebar's internal structure.
