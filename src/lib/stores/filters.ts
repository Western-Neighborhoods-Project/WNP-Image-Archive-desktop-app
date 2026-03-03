import { writable } from 'svelte/store';

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

export const filters = writable<FilterState>({
  city: null,
  photographer: null,
  collectionId: null,
  yearStart: null,
  yearEnd: null,
  missingMetadata: false,
  searchQuery: null,
  sortBy: 'catalog_number',
  sortOrder: 'asc',
});
