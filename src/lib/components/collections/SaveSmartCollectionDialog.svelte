<script lang="ts">
  // Save the current filter state as a named smart collection.
  //
  // Caller passes the live FilterState; this dialog serializes it
  // to JSON, posts it to the backend, and refreshes the global
  // smartCollections store on success.

  import * as Dialog from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { createSmartCollection } from "$lib/commands/smartCollections";
  import { refreshSmartCollections } from "$lib/stores/smartCollections";
  import type { FilterState } from "$lib/stores/filters";

  interface Props {
    open: boolean;
    /** Snapshot of the filter state to save. Captured at the moment
     *  the dialog opens so further filter changes don't affect the
     *  pending save. */
    snapshot: FilterState;
    onClose?: () => void;
  }

  let { open = $bindable(), snapshot, onClose }: Props = $props();

  let name = $state("");
  let submitting = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    if (!open) {
      name = "";
      submitting = false;
      error = null;
    }
  });

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    if (submitting || !name.trim()) return;
    submitting = true;
    error = null;
    try {
      await createSmartCollection(name.trim(), JSON.stringify(snapshot));
      await refreshSmartCollections();
      open = false;
      onClose?.();
    } catch (e) {
      error = String(e);
    } finally {
      submitting = false;
    }
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="max-w-sm">
    <form onsubmit={handleSubmit}>
      <Dialog.Header>
        <Dialog.Title>Save filter as smart collection</Dialog.Title>
        <Dialog.Description>
          The current filter values will be saved under this name.
          Pick it from the sidebar later to re-apply.
        </Dialog.Description>
      </Dialog.Header>

      <div class="py-4 space-y-1.5">
        <Label for="smart-name">Name</Label>
        <Input
          id="smart-name"
          bind:value={name}
          placeholder="Sutro Baths · with photographer"
          autocomplete="off"
          required
          autofocus
        />
      </div>

      {#if error}
        <p class="text-[12px] text-destructive">{error}</p>
      {/if}

      <Dialog.Footer>
        <Button type="button" variant="outline" onclick={() => (open = false)}>
          Cancel
        </Button>
        <Button type="submit" disabled={submitting || !name.trim()}>
          {submitting ? "Saving…" : "Save"}
        </Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>
