import { generateFullThumbnails } from '$lib/commands/images';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** Manages on-demand thumbnail generation for visible grid items, and routes
 *  per-image `thumbnail:ready` events (the backend emits one as each thumbnail
 *  commits, from either the background worker or the on-demand path) to the
 *  specific grid item so it can refresh the instant its thumbnail is ready —
 *  rather than all-at-once when a whole batch finishes. */
class ThumbnailQueue {
  private queue = new Set<number>();
  private timer: ReturnType<typeof setTimeout> | null = null;
  private debounceMs = 300;
  private processing = false;

  // id → refresh callbacks (one per mounted GridItem showing that id). Keyed so
  // a `thumbnail:ready` event dispatches in O(1) even during a bulk import that
  // emits thousands of them.
  private readyCallbacks = new Map<number, Set<() => void>>();

  /** Register a refresh callback for a specific image id; returns an
   *  unregister fn. Called by GridItem on mount. */
  onReady(id: number, cb: () => void): () => void {
    let set = this.readyCallbacks.get(id);
    if (!set) {
      set = new Set();
      this.readyCallbacks.set(id, set);
    }
    set.add(cb);
    return () => {
      const s = this.readyCallbacks.get(id);
      if (!s) return;
      s.delete(cb);
      if (s.size === 0) this.readyCallbacks.delete(id);
    };
  }

  /** Fire the refresh callbacks for one id. Called by the `thumbnail:ready`
   *  event listener (see initThumbnailReadyListener). */
  notifyReady(id: number) {
    const set = this.readyCallbacks.get(id);
    if (set) for (const cb of [...set]) cb();
  }

  /** Add an image id to the on-demand (visible-priority) generation queue. */
  add(id: number) {
    if (this.queue.has(id)) return;
    this.queue.add(id);
    this.scheduleFlush();
  }

  private scheduleFlush() {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = setTimeout(() => this.flush(), this.debounceMs);
  }

  private async flush() {
    if (this.processing || this.queue.size === 0) return;
    this.processing = true;

    // Take up to 20 IDs per batch.
    const batch = [...this.queue].slice(0, 20);
    batch.forEach((id) => this.queue.delete(id));

    try {
      // The backend commits each thumbnail as it's decoded and emits a
      // `thumbnail:ready` event per success; the grid refreshes from those
      // events, not from this promise resolving.
      await generateFullThumbnails(batch);
    } catch (e) {
      console.error('Thumbnail generation failed:', e);
    } finally {
      this.processing = false;
      if (this.queue.size > 0) this.scheduleFlush();
    }
  }
}

export const thumbnailQueue = new ThumbnailQueue();

/** Subscribe to backend `thumbnail:ready` events and route each to the grid
 *  item(s) for that id. Call once from `+page.svelte` onMount; returns a
 *  cleanup fn. */
export async function initThumbnailReadyListener(): Promise<() => void> {
  let unlisten: UnlistenFn | null = null;
  try {
    unlisten = await listen<number>('thumbnail:ready', (event) => {
      thumbnailQueue.notifyReady(event.payload);
    });
  } catch (e) {
    console.error('Failed to subscribe to thumbnail:ready events', e);
  }
  return () => {
    if (unlisten) unlisten();
  };
}
