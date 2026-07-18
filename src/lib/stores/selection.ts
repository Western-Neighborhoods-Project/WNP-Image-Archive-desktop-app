import { writable } from 'svelte/store';

// ── Library grid selection ─────────────────────────────────────────────────
// Multi-image selection state for the library grid. Used by:
//   - GridItem (visual selected state, click handling)
//   - Grid (marquee rectangle selection, drag start)
//   - Sidebar collection items (drop targets)
//
// `lastClickedId` is tracked separately so Shift-click can build a range
// from the last clicked-or-marqueed item to the just-clicked one. It
// resets to null whenever selection is cleared.

export const selectedImageIds = writable<Set<number>>(new Set());
export const lastClickedId = writable<number | null>(null);

/** Replace the selection with a single image. */
export function selectOnly(id: number): void {
  selectedImageIds.set(new Set([id]));
  lastClickedId.set(id);
}

/** Toggle a single image in/out of the selection (Cmd/Ctrl-click). */
export function toggleSelected(id: number): void {
  selectedImageIds.update((s) => {
    const next = new Set(s);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    return next;
  });
  lastClickedId.set(id);
}

/** Select an inclusive range of ids — caller passes the ordered list of
 *  visible image ids and the start/end ids to range-select between. Used
 *  by Shift-click. */
export function selectRange(orderedIds: number[], from: number, to: number): void {
  const fromIdx = orderedIds.indexOf(from);
  const toIdx = orderedIds.indexOf(to);
  if (fromIdx < 0 || toIdx < 0) {
    selectOnly(to);
    return;
  }
  const [a, b] = fromIdx < toIdx ? [fromIdx, toIdx] : [toIdx, fromIdx];
  selectedImageIds.update((s) => {
    const next = new Set(s);
    for (let i = a; i <= b; i++) next.add(orderedIds[i]);
    return next;
  });
  lastClickedId.set(to);
}

/** Replace the selection with the given set of ids. Used by the marquee
 *  during a drag — caller passes ids hit by the rect. */
export function setSelection(ids: Iterable<number>): void {
  selectedImageIds.set(new Set(ids));
}

export function clearSelection(): void {
  selectedImageIds.set(new Set());
  lastClickedId.set(null);
}
