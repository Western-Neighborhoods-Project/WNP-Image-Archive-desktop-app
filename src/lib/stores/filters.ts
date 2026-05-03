import { writable, derived, type Readable } from 'svelte/store';
import { currentSmartCollectionId } from '$lib/stores/navigation';
import { smartCollections } from '$lib/stores/smartCollections';

export interface FilterState {
  city: string | null;
  photographer: string | null;
  collectionId: number | null;
  yearStart: number | null;
  yearEnd: number | null;
  missingMetadata: boolean;
  searchQuery: string | null;
  sortBy: string;
  sortOrder: 'asc' | 'desc';
}

const DEFAULT_FILTERS: FilterState = {
  city: null,
  photographer: null,
  collectionId: null,
  yearStart: null,
  yearEnd: null,
  missingMetadata: false,
  searchQuery: null,
  sortBy: 'catalog_number',
  sortOrder: 'asc',
};

/** USER filters — what the FilterBar binds to. Always represents the
 *  user-set additional criteria on top of any smart collection that's
 *  active. Reset to defaults when entering / leaving a smart collection. */
export const filters = writable<FilterState>({ ...DEFAULT_FILTERS });

export function resetFilters(): void {
  filters.set({ ...DEFAULT_FILTERS });
}

// ── Locked + effective filters (smart collection support) ──────────────────
//
// When a smart collection is active, its saved filter values are
// "locked": the FilterBar disables the corresponding inputs, and the
// effective query merges locked + user (locked wins per field). For
// boolean / search / sort fields, locked wins when truthy.

/** The active SC's saved FilterState, or null. Derived from
 *  currentSmartCollectionId and the cached smartCollections list. */
export const lockedFilters: Readable<FilterState | null> = derived(
  [currentSmartCollectionId, smartCollections],
  ([$id, $list]) => {
    if ($id === null) return null;
    const sc = $list.find((s) => s.id === $id);
    if (!sc) return null;
    try {
      return JSON.parse(sc.filters) as FilterState;
    } catch {
      return null;
    }
  },
);

/** Per-field "is this locked?" flags. UI uses this to disable inputs
 *  and show locked chips. A field is locked when the SC has a non-
 *  default value for it. */
export interface FieldLocks {
  city: boolean;
  photographer: boolean;
  collectionId: boolean;
  yearStart: boolean;
  yearEnd: boolean;
  missingMetadata: boolean;
  searchQuery: boolean;
  sort: boolean;
}

export const fieldLocks: Readable<FieldLocks> = derived(
  lockedFilters,
  ($locked) => ({
    city: $locked?.city != null && $locked.city !== '',
    photographer:
      $locked?.photographer != null && $locked.photographer !== '',
    collectionId: $locked?.collectionId != null,
    yearStart: $locked?.yearStart != null,
    yearEnd: $locked?.yearEnd != null,
    missingMetadata: !!$locked?.missingMetadata,
    searchQuery: $locked?.searchQuery != null && $locked.searchQuery !== '',
    sort:
      $locked != null &&
      ($locked.sortBy !== DEFAULT_FILTERS.sortBy ||
        $locked.sortOrder !== DEFAULT_FILTERS.sortOrder),
  }),
);

/** The merged filter set sent to the backend. Locked fields override
 *  user fields. Use this for queries; UI components keep using
 *  `filters` for the user-controlled half. */
export const effectiveFilters: Readable<FilterState> = derived(
  [filters, lockedFilters, fieldLocks],
  ([$user, $locked, $locks]) => {
    if (!$locked) return $user;
    return {
      city: $locks.city ? $locked.city : $user.city,
      photographer: $locks.photographer ? $locked.photographer : $user.photographer,
      collectionId: $locks.collectionId ? $locked.collectionId : $user.collectionId,
      yearStart: $locks.yearStart ? $locked.yearStart : $user.yearStart,
      yearEnd: $locks.yearEnd ? $locked.yearEnd : $user.yearEnd,
      missingMetadata: $locks.missingMetadata
        ? $locked.missingMetadata
        : $user.missingMetadata,
      searchQuery: $locks.searchQuery ? $locked.searchQuery : $user.searchQuery,
      sortBy: $locks.sort ? $locked.sortBy : $user.sortBy,
      sortOrder: $locks.sort ? $locked.sortOrder : $user.sortOrder,
    };
  },
);
