<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog';
  import * as AlertDialog from '$lib/components/ui/alert-dialog';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { createCollection, renameCollection, deleteCollection } from '$lib/commands/collections';
  import { refreshUserCollections } from '$lib/stores/collections';
  import { currentCollectionId } from '$lib/stores/navigation';
  import { filters } from '$lib/stores/filters';

  let {
    showCreate = $bindable(false),
    showRename = $bindable(false),
    showDelete = $bindable(false),
    targetCollection = $bindable<{ id: number; name: string } | null>(null),
  }: {
    showCreate: boolean;
    showRename: boolean;
    showDelete: boolean;
    targetCollection: { id: number; name: string } | null;
  } = $props();

  let inputName = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);

  // Pre-fill rename input when dialog opens; reset create input
  $effect(() => {
    if (showRename && targetCollection) inputName = targetCollection.name;
    if (showCreate) inputName = '';
    error = null;
  });

  async function handleCreate() {
    // Guard re-entrancy: the Enter-key handler calls this directly, bypassing
    // the disabled-button guard, so a fast double Enter could create two.
    if (busy) return;
    if (!inputName.trim()) return;
    busy = true;
    error = null;
    try {
      await createCollection(inputName.trim());
      await refreshUserCollections();
      showCreate = false;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function handleRename() {
    if (busy) return;
    if (!targetCollection || !inputName.trim()) return;
    busy = true;
    error = null;
    try {
      await renameCollection(targetCollection.id, inputName.trim());
      await refreshUserCollections();
      showRename = false;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function handleDelete() {
    if (busy) return;
    if (!targetCollection) return;
    busy = true;
    error = null;
    try {
      const id = targetCollection.id;
      await deleteCollection(id);
      await refreshUserCollections();
      // If the deleted collection is currently being browsed, fall back to full library
      if ($currentCollectionId === id) {
        currentCollectionId.set(null);
        filters.update((f) => ({ ...f, collectionId: null }));
      }
      showDelete = false;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<!-- Create collection dialog -->
<Dialog.Root bind:open={showCreate}>
  <Dialog.Content class="max-w-sm">
    <Dialog.Header>
      <Dialog.Title>New Collection</Dialog.Title>
    </Dialog.Header>
    <div class="space-y-2 py-3">
      <Label for="col-create-name">Name</Label>
      <Input
        id="col-create-name"
        bind:value={inputName}
        placeholder="e.g. Civil Rights Photos"
        onkeydown={(e) => { if (e.key === 'Enter') handleCreate(); }}
        autofocus
      />
      {#if error}<p class="text-xs text-red-600">{error}</p>{/if}
    </div>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (showCreate = false)}>Cancel</Button>
      <Button disabled={busy || !inputName.trim()} onclick={handleCreate}>
        {busy ? 'Creating…' : 'Create'}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<!-- Rename collection dialog -->
<Dialog.Root bind:open={showRename}>
  <Dialog.Content class="max-w-sm">
    <Dialog.Header>
      <Dialog.Title>Rename Collection</Dialog.Title>
    </Dialog.Header>
    <div class="space-y-2 py-3">
      <Label for="col-rename-name">Name</Label>
      <Input
        id="col-rename-name"
        bind:value={inputName}
        onkeydown={(e) => { if (e.key === 'Enter') handleRename(); }}
        autofocus
      />
      {#if error}<p class="text-xs text-red-600">{error}</p>{/if}
    </div>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (showRename = false)}>Cancel</Button>
      <Button disabled={busy || !inputName.trim()} onclick={handleRename}>
        {busy ? 'Renaming…' : 'Rename'}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<!-- Delete confirmation alert-dialog -->
<AlertDialog.Root bind:open={showDelete}>
  <AlertDialog.Content class="max-w-sm">
    <AlertDialog.Header>
      <AlertDialog.Title>Delete "{targetCollection?.name}"?</AlertDialog.Title>
      <AlertDialog.Description>
        This removes the collection. Images are not deleted.
      </AlertDialog.Description>
    </AlertDialog.Header>
    {#if error}<p class="px-1 text-xs text-red-600">{error}</p>{/if}
    <AlertDialog.Footer>
      <AlertDialog.Cancel onclick={() => (showDelete = false)}>Cancel</AlertDialog.Cancel>
      <AlertDialog.Action
        class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
        disabled={busy}
        onclick={handleDelete}
      >
        {busy ? 'Deleting…' : 'Delete'}
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
