<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog';
  import { Button } from '$lib/components/ui/button';
  import { userCollections, refreshUserCollections } from '$lib/stores/collections';
  import { addToCollection, removeFromCollection, getImageCollections } from '$lib/commands/collections';

  let {
    open = $bindable(false),
    imageId = $bindable<number | null>(null),
    onclose,
  }: {
    open: boolean;
    imageId: number | null;
    onclose?: () => void;
  } = $props();

  let originalIds = $state<Set<number>>(new Set());
  let checkedIds = $state<Set<number>>(new Set());
  let loading = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    if (open && imageId != null) {
      loadMemberships(imageId);
    } else if (!open) {
      error = null;
    }
  });

  async function loadMemberships(id: number) {
    loading = true;
    error = null;
    try {
      const cols = await getImageCollections(id);
      const ids = new Set(cols.map((c) => c.id));
      originalIds = new Set(ids);
      checkedIds = new Set(ids);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function toggle(id: number) {
    const next = new Set(checkedIds);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    checkedIds = next;
  }

  async function handleSave() {
    if (imageId == null) return;
    busy = true;
    error = null;
    try {
      const toAdd = [...checkedIds].filter((id) => !originalIds.has(id));
      const toRemove = [...originalIds].filter((id) => !checkedIds.has(id));
      for (const colId of toAdd) {
        await addToCollection(colId, [imageId]);
      }
      for (const colId of toRemove) {
        await removeFromCollection(colId, [imageId]);
      }
      await refreshUserCollections();
      open = false;
      onclose?.();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="max-w-sm">
    <Dialog.Header>
      <Dialog.Title>Add to Collection</Dialog.Title>
    </Dialog.Header>
    <div class="py-3">
      {#if loading}
        <p class="text-sm text-gray-400">Loading…</p>
      {:else if $userCollections.length === 0}
        <p class="text-sm text-gray-500">No collections yet. Create one from the sidebar.</p>
      {:else}
        <ul class="space-y-0.5">
          {#each $userCollections as col (col.id)}
            <li>
              <label class="flex cursor-pointer items-center gap-3 rounded-md px-2 py-1.5 hover:bg-gray-100">
                <input
                  type="checkbox"
                  class="h-4 w-4 rounded border-gray-300 accent-primary"
                  checked={checkedIds.has(col.id)}
                  onchange={() => toggle(col.id)}
                />
                <span class="flex-1 text-sm">{col.name}</span>
                <span class="text-xs text-gray-400">{col.image_count}</span>
              </label>
            </li>
          {/each}
        </ul>
      {/if}
      {#if error}<p class="mt-2 text-xs text-red-600">{error}</p>{/if}
    </div>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (open = false)}>Cancel</Button>
      <Button disabled={busy || $userCollections.length === 0} onclick={handleSave}>
        {busy ? 'Saving…' : 'Done'}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
