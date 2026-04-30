# Plan 2 — Existing-view refactor

> **Status:** Skeleton awaiting expansion. When ready to execute, run the `superpowers:writing-plans` skill to expand into full task-by-task detail.
>
> **Position in roadmap:** Plan 2 of 9. See `2026-04-27-roadmap.md`.
>
> **Depends on:** Plan 1 (design tokens + primitives + sidebar).

## Goal

Bring the four existing main views (Library/Grid, Detail, Requests, Settings) into design alignment using the primitives from Plan 1. After this plan ships, every existing screen visually matches the Claude Design handoff bundle.

## Scope

- **Library/Grid:** delete `TopBar.svelte`; replace with `PageHeader` + filter-chip row above the grid; rebuild grid as Lightroom-style justified-rows (target 176px row height, 6px gap); add catalog overlay + missing-metadata amber dot to thumbnails; add `StatusBar` showing selection info + drive indicator stub.
- **Detail:** new layout = main image + zoom controls overlay + filmstrip below (left column) + right inspector panel (400px, tabs Metadata/History/Usage). Inspector form fields grouped Standard/Archival, uppercase tracked labels, "Show all" toggle for advanced fields. Header bar gets back button, title+catalog, n-of-X, prev/next, Share/Export/Save.
- **Requests:** rewrite to two-pane = orders list (360px) + detail pane with per-image approve/deny grid. Order header shows status/requester/org/email/submitted; purpose/details sections below.
- **Settings:** rewrite as sub-nav layout (left list of pages) + content area. Make the **Fields** page real (Standard/Archival/EXIF groups, Visible/Advanced/Required toggles). Stub all other sub-pages (General, Collections, Import, Sharing, OpenSFHistory API, Backup, Users, Keyboard).

## Out of scope

- ⌘K command bar (Plan 3)
- Filter popover with year histogram (defer or fold in late as 2.5)
- Pending-changes pre-save card (user chose to keep save-on-blur)
- Drive indicator full implementation (Plan 6 — stub only here)

## Key files

**Modify:**
- `src/lib/components/browsing/Grid.svelte`, `GridItem.svelte`
- `src/lib/components/detail/DetailView.svelte`
- `src/lib/components/requests/RequestsView.svelte`
- `src/lib/components/layout/SettingsView.svelte`
- `src/lib/components/layout/FilterBar.svelte`
- `src/routes/+page.svelte`

**Delete:**
- `src/lib/components/layout/TopBar.svelte`

**New:**
- `src/lib/components/detail/Filmstrip.svelte`, `MetadataPanel.svelte`, `ZoomControls.svelte`
- `src/lib/components/settings/SettingsNav.svelte`, `pages/FieldsPage.svelte` + sub-page stubs
- `src/lib/components/requests/OrderList.svelte`, `OrderDetail.svelte`, `OrderImageCard.svelte`

## Open questions

- Justified-rows algorithm: client-side using the `justifyRows` helper from `_design/wnp-app/project/data.jsx`, or a library?
- Settings sub-pages: how many to make real (Fields is the design's primary; everything else is a stub for now)?
- Detail view: include the website-sync banner the design shows, or omit (Plan 9 lands sync later)?
- Filter popover with year histogram: include here or fold into a follow-up Plan 2.5?

## Verification

Visual diff against `_design/wnp-app/standalone/` artboards 1a, 2, 3a/3b, 4a. All existing functional flows still work (search, filter, edit, fulfill, etc.). Window minimum 1440px, layout doesn't break wider.

## Estimated size

**L.** Largest plan in the roadmap.
