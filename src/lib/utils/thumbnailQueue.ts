import { generateFullThumbnails } from '$lib/commands/images';

type RefreshCallback = (imageId: number) => void;

/** Manages on-demand full thumbnail generation for visible grid items. */
class ThumbnailQueue {
  private queue: Set<number> = new Set();
  private timer: ReturnType<typeof setTimeout> | null = null;
  private callbacks: RefreshCallback[] = [];
  private debounceMs = 300;
  private processing = false;

  /** Register a callback to be called when thumbnails finish generating. */
  onRefresh(cb: RefreshCallback) {
    this.callbacks.push(cb);
    return () => {
      this.callbacks = this.callbacks.filter((c) => c !== cb);
    };
  }

  /** Add an image ID to the generation queue. */
  add(imageId: number) {
    if (this.queue.has(imageId)) return;
    this.queue.add(imageId);
    this.scheduleFlush();
  }

  private scheduleFlush() {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = setTimeout(() => this.flush(), this.debounceMs);
  }

  private async flush() {
    if (this.processing || this.queue.size === 0) return;
    this.processing = true;

    // Take up to 20 IDs per batch
    const batch = [...this.queue].slice(0, 20);
    batch.forEach((id) => this.queue.delete(id));

    try {
      await generateFullThumbnails(batch);
      batch.forEach((id) => this.callbacks.forEach((cb) => cb(id)));
    } catch (e) {
      console.error('Thumbnail generation failed:', e);
    } finally {
      this.processing = false;
      if (this.queue.size > 0) this.scheduleFlush();
    }
  }
}

export const thumbnailQueue = new ThumbnailQueue();
