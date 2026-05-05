<script lang="ts">
  // Multi-image add-to-collection dialog (Plan 11).
  //
  // Distinct from AddToCollectionDialog (which is single-image with
  // mixed add/remove semantics based on each image's existing
  // memberships). When N images are selected, the "current memberships"
  // model breaks down — instead we just let the user pick ONE collection
  // and add all selected images to it.

  import * as Dialog from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";
  import {
    userCollections,
    refreshUserCollections,
  } from "$lib/stores/collections";
  import { addToCollection } from "$lib/commands/collections";

  let {
    open = $bindable(false),
    imageIds = $bindable<number[]>([]),
    onclose,
  }: {
    open: boolean;
    imageIds: number[];
    onclose?: () => void;
  } = $props();

  let selectedCollectionId = $state<number | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!open) {
      selectedCollectionId = null;
      error = null;
    }
  });

  async function handleSave() {
    if (selectedCollectionId === null || imageIds.length === 0) return;
    busy = true;
    error = null;
    try {
      await addToCollection(selectedCollectionId, imageIds);
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
      <Dialog.Title>
        Add {imageIds.length}
        {imageIds.length === 1 ? "image" : "images"} to Collection
      </Dialog.Title>
      <Dialog.Description>
        Pick a collection. The images will be added to it; existing
        members are unchanged.
      </Dialog.Description>
    </Dialog.Header>
    <div class="py-3">
      {#if $userCollections.length === 0}
        <p class="text-xs text-muted-foreground">
          No collections yet. Create one from the sidebar.
        </p>
      {:else}
        <ul class="space-y-0.5 max-h-[280px] overflow-y-auto">
          {#each $userCollections as col (col.id)}
            <li>
              <label
                class="flex cursor-pointer items-center gap-3 rounded-md px-2 py-1.5 hover:bg-hover"
              >
                <input
                  type="radio"
                  name="collection"
                  class="h-3.5 w-3.5 accent-primary"
                  checked={selectedCollectionId === col.id}
                  onchange={() => (selectedCollectionId = col.id)}
                />
                <span class="flex-1 text-xs">{col.name}</span>
                <span class="text-[11px] text-muted-foreground tabular-nums">
                  {col.image_count}
                </span>
              </label>
            </li>
          {/each}
        </ul>
      {/if}
      {#if error}
        <p class="mt-2 text-[11.5px] text-destructive">{error}</p>
      {/if}
    </div>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (open = false)}>Cancel</Button>
      <Button
        disabled={busy ||
          selectedCollectionId === null ||
          $userCollections.length === 0}
        onclick={handleSave}
      >
        {busy ? "Adding…" : "Add"}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
