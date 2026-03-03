import { writable } from 'svelte/store';

export type ViewType = 'setup' | 'import' | 'library' | 'detail' | 'collection' | 'requests' | 'settings';

export const currentView = writable<ViewType>('setup');
export const currentImageId = writable<number | null>(null);
export const currentCollectionId = writable<number | null>(null);

/** Saved scroll offset of the grid — restored when navigating back from detail view. */
export const savedScrollOffset = writable<number>(0);
