import { writable, derived, type Readable } from 'svelte/store';

export type ViewType =
  | 'setup'
  | 'import'
  | 'library'
  | 'detail'
  | 'recently-viewed'
  | 'requests'
  | 'settings'
  | 'audit';

/** Sub-page within the Settings view. Lifted into the navigation store
 *  (rather than local SettingsView state) so the command bar can deep-link
 *  to a specific section. Trimmed in 2026-05 to drop pages whose plans
 *  haven't shipped (Backup, Import) or whose features moved to code-level
 *  (Fields, Collections). Add new entries here when their pages land. */
export type SettingsPageKey =
  | 'general'
  | 'sharing'
  | 'external'
  | 'users'
  | 'keyboard';

export const currentView = writable<ViewType>('setup');
export const currentImageId = writable<number | null>(null);
export const currentCollectionId = writable<number | null>(null);
/** The active smart collection id, or null when not viewing one. Set
 *  by Sidebar.applySmartCollection; cleared whenever the user navigates
 *  to All-images, an archive collection, or a regular user collection. */
export const currentSmartCollectionId = writable<number | null>(null);
export const currentSettingsPage = writable<SettingsPageKey>('general');

/** Saved scroll offset of the grid — restored when navigating back from detail view. */
export const savedScrollOffset = writable<number>(0);

/** Navigate to the detail view for an image. Grid callers pass the current
 *  scroll offset so returning restores position; audit / command-bar /
 *  filmstrip callers omit it. Single source of truth so the two-store
 *  navigation sequence can't drift across its several call sites (some used to
 *  save the scroll offset, some didn't). */
export function openImageDetail(id: number, scrollOffset?: number): void {
  if (scrollOffset !== undefined) savedScrollOffset.set(scrollOffset);
  currentImageId.set(id);
  currentView.set('detail');
}

// ── Window chrome title ────────────────────────────────────────────────────
// Single source of truth for what the macOS titlebar (and our custom
// HTML chrome) display. Most views derive their suffix from currentView.
// The detail view is special: it sets `detailWindowTitle` directly to the
// catalog + image title once an image loads, and the overall windowTitle
// derived prefers that when set.

/** Set by DetailView when an image loads, e.g. "wnp27.4283 — Sutro Baths,
 *  exterior". Reset to null by DetailView's onDestroy so subsequent views
 *  fall back to the per-view suffix derivation. */
export const detailWindowTitle = writable<string | null>(null);

/** Set by LibraryView with its current `pageTitle` — "All images",
 *  the active archive/user collection name, or the active smart
 *  collection name. Used so the window chrome reflects which slice
 *  of the catalog the user is looking at, not just the bare
 *  "All images" suffix that the static viewSuffix() returned before. */
export const libraryWindowTitle = writable<string | null>(null);

function viewSuffix(view: ViewType): string {
  switch (view) {
    case 'library':
      return 'All images';
    case 'recently-viewed':
      return 'Recently viewed';
    case 'requests':
      return 'Image requests';
    case 'audit':
      return 'Audit log';
    case 'settings':
      return 'Settings';
    case 'setup':
      return 'Setup';
    case 'import':
      return 'Importing';
    default:
      return '';
  }
}

export const windowTitle: Readable<string> = derived(
  [currentView, detailWindowTitle, libraryWindowTitle],
  ([$currentView, $detailWindowTitle, $libraryWindowTitle]) => {
    if ($currentView === 'detail' && $detailWindowTitle) {
      return $detailWindowTitle;
    }
    if ($currentView === 'library' && $libraryWindowTitle) {
      return `Image Archive Manager — ${$libraryWindowTitle}`;
    }
    const suffix = viewSuffix($currentView);
    return suffix ? `Image Archive Manager — ${suffix}` : 'Image Archive Manager';
  },
);
