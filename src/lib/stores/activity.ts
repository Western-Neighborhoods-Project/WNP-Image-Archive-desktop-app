import { writable } from 'svelte/store';

/**
 * Monotonically increasing counter that should be bumped any time
 * something happens which could change `get_recent_activity` output —
 * primarily metadata edits in the Detail view. Subscribers ($effect on
 * `$activityVersion`) re-fetch whenever it changes.
 *
 * This is the simplest cross-component invalidation primitive that doesn't
 * require an event bus or an explicit query-cache layer. Cost is one
 * extra round-trip per edit, which is fine for a single-user desktop app.
 */
export const activityVersion = writable<number>(0);

export function bumpActivity(): void {
  activityVersion.update((n) => n + 1);
}
