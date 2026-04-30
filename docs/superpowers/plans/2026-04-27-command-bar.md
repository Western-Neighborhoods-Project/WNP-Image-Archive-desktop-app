# Plan 3 — ⌘K command bar

> **Status:** Skeleton awaiting expansion. When ready to execute, run the `superpowers:writing-plans` skill to expand into full task-by-task detail.
>
> **Position in roadmap:** Plan 3 of 9. See `2026-04-27-roadmap.md`.
>
> **Depends on:** Plan 1 (`Kbd` and `KbdSeq` primitives, sidebar launcher placeholder).

## Goal

Implement a global command palette (Raycast-style) accessible via ⌘K with image search, actions, and jump-to navigation.

## Scope

- New `CommandBar.svelte` mounted globally (in `+page.svelte` root)
- Global ⌘K keyboard shortcut to open; Esc / outside-click to close
- Three result groups: **Images** (live search via existing FTS5), **Actions** (filter by city/year/collection, share, etc.), **Go to** (sidebar destinations)
- Multi-key shortcut chains for power users (G+Q → requests, G+L → audit log, G+A → all images, etc.)
- Selected-row highlight, ↑↓ to navigate, ↵ to execute
- Wire the existing inert ⌘K launcher in the sidebar (Plan 1) to open this

## Out of scope

- Fuzzy search ranking (use existing FTS5 — already good enough)
- Every-action-imaginable coverage — pick the 10–15 most-used commands first; add more as the team asks for them

## Key files

**New:**
- `src/lib/components/command-bar/CommandBar.svelte`, `CommandRow.svelte`, `CommandGroup.svelte`
- `src/lib/stores/commandBar.ts` (open/close + query state)
- `src/lib/utils/keyboardShortcuts.ts` (global shortcut handler with chord support)

**Modify:**
- `src/routes/+page.svelte` (mount + global handler)
- `src/lib/components/layout/Sidebar.svelte` (wire launcher button click → open command bar)

## Open questions

- Use `bits-ui` Command primitive or build from scratch? (shadcn-svelte has a Command pattern but we don't have it installed.)
- Chord timeout (how long after G before C+ are still considered part of a chord)?
- Search debounce interval (probably 100–150ms)
- How to handle conflicts with browser-default ⌘K in form fields

## Verification

Press ⌘K from any view, search "sutro", confirm Images group populates. Press G then Q, confirm jumps to requests. Visual diff against artboard 1b.

## Estimated size

**M.**
