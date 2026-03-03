import { writable } from 'svelte/store';

export type ViewType = 'setup' | 'import' | 'library' | 'detail' | 'collection' | 'requests' | 'settings';

export const currentView = writable<ViewType>('setup');
export const currentImageId = writable<number | null>(null);
export const currentCollectionId = writable<number | null>(null);
