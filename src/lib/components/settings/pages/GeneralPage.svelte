<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { resetCatalog } from "$lib/commands/settings";
  import { scanDirectory } from "$lib/commands/images";
  import {
    listSourceDirectories,
    addSourceDirectory,
    removeSourceDirectory,
    renameSourceDirectory,
    type SourceDirectory,
  } from "$lib/commands/sources";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { driveStatus, driveStatusReady } from "$lib/stores/driveStatus";
  import { retryDriveConnection } from "$lib/commands/drive";
  import { formatFileSize, formatCount } from "$lib/utils/format";
  import { RefreshCw, Plus, Pencil, Trash2, Check, X } from "@lucide/svelte";

  let { onResetComplete }: { onResetComplete: () => void } = $props();

  let sources = $state<SourceDirectory[]>([]);
  // Per-source rescan/remove busy state, keyed by source id.
  let rescanState = $state<Record<number, { busy: boolean; status: string | null }>>({});
  // Inline rename: which source is being edited + the in-progress label value.
  let editingId = $state<number | null>(null);
  let editingLabel = $state<string>("");
  // Confirm-dialog state for source removal.
  let removeTarget = $state<SourceDirectory | null>(null);
  let isRemoving = $state(false);
  // Add-source state.
  let isAdding = $state(false);
  let addError = $state<string | null>(null);

  // Reset-catalog dialog (unchanged from before; just lives in the danger zone).
  let showResetConfirm = $state(false);
  let isResetting = $state(false);
  let resetError = $state<string | null>(null);

  // Drive-status retry button.
  let isRetrying = $state(false);

  async function refreshSources() {
    try {
      sources = await listSourceDirectories();
    } catch (e) {
      console.error("Failed to load source directories", e);
    }
  }

  onMount(refreshSources);

  function setRescanState(id: number, busy: boolean, status: string | null) {
    rescanState = { ...rescanState, [id]: { busy, status } };
  }

  async function handleRescan(source: SourceDirectory) {
    setRescanState(source.id, true, "Scanning…");
    try {
      const scan = await scanDirectory(source.path);
      const summary = `Scanned ${formatCount(scan.total_files)} files · ${formatCount(scan.new_files)} new`;
      const tail =
        scan.new_files > 0
          ? " · thumbnails + metadata generating in background"
          : " · nothing to import";
      setRescanState(source.id, false, `${summary}${tail}`);
      await refreshSources();
      await retryDriveConnection();
    } catch (e) {
      setRescanState(source.id, false, `Failed: ${e}`);
    }
  }

  async function handleAdd() {
    addError = null;
    let picked: string | null = null;
    try {
      const result = await open({
        directory: true,
        multiple: false,
        title: "Select archive directory",
      });
      picked = typeof result === "string" ? result : null;
    } catch (e) {
      addError = `Picker failed: ${e}`;
      return;
    }
    if (!picked) return;

    isAdding = true;
    try {
      const source = await addSourceDirectory(picked);
      await refreshSources();
      // Auto-trigger first scan so the new source actually shows images.
      await handleRescan(source);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      addError = msg;
    } finally {
      isAdding = false;
    }
  }

  function startRename(source: SourceDirectory) {
    editingId = source.id;
    editingLabel = source.label;
  }

  async function commitRename() {
    if (editingId === null) return;
    const id = editingId;
    const label = editingLabel.trim();
    editingId = null;
    if (!label) {
      await refreshSources();
      return;
    }
    try {
      await renameSourceDirectory(id, label);
      await refreshSources();
    } catch (e) {
      console.error("Rename failed", e);
      await refreshSources();
    }
  }

  function cancelRename() {
    editingId = null;
  }

  async function confirmRemove() {
    if (!removeTarget) return;
    const target = removeTarget;
    isRemoving = true;
    try {
      await removeSourceDirectory(target.id);
      removeTarget = null;
      await refreshSources();
    } catch (e) {
      console.error("Remove failed", e);
    } finally {
      isRemoving = false;
    }
  }

  async function handleRetry() {
    isRetrying = true;
    try {
      await retryDriveConnection();
    } finally {
      isRetrying = false;
    }
  }

  async function confirmReset() {
    isResetting = true;
    resetError = null;
    try {
      await resetCatalog();
      onResetComplete();
    } catch (e) {
      resetError = String(e);
    } finally {
      isResetting = false;
      showResetConfirm = false;
    }
  }
</script>

<div class="max-w-[720px]">
  <!-- Source directories — Plan 12 multi-source UI. -->
  <section class="mb-7">
    <h3 class="text-[14px] font-semibold text-foreground mb-1">
      Image archive directories
    </h3>
    <p class="text-[12px] text-muted-foreground mb-3">
      Add one or more top-level folders. Subfolders show up in the sidebar
      as a nested tree.
    </p>

    <div class="space-y-2">
      {#each sources as source (source.id)}
        {@const rescan = rescanState[source.id]}
        <div class="rounded-md border border-border bg-secondary/30 p-3">
          <div class="flex items-start justify-between gap-3">
            <div class="min-w-0 flex-1">
              {#if editingId === source.id}
                <div class="flex items-center gap-1.5 mb-1">
                  <Input
                    bind:value={editingLabel}
                    onkeydown={(e: KeyboardEvent) => {
                      if (e.key === "Enter") commitRename();
                      if (e.key === "Escape") cancelRename();
                    }}
                    autofocus
                    class="h-7 text-[13px]"
                  />
                  <button
                    type="button"
                    onclick={commitRename}
                    class="rounded p-1 text-success hover:bg-hover"
                    title="Save label"
                    aria-label="Save label"
                  >
                    <Check size={14} />
                  </button>
                  <button
                    type="button"
                    onclick={cancelRename}
                    class="rounded p-1 text-muted-foreground hover:bg-hover"
                    title="Cancel rename"
                    aria-label="Cancel rename"
                  >
                    <X size={14} />
                  </button>
                </div>
              {:else}
                <div class="flex items-center gap-1.5 mb-1">
                  <span class="text-[13px] font-medium text-foreground">
                    {source.label}
                  </span>
                  <button
                    type="button"
                    onclick={() => startRename(source)}
                    class="rounded p-0.5 text-muted-foreground hover:bg-hover hover:text-foreground"
                    title="Rename"
                    aria-label="Rename"
                  >
                    <Pencil size={11} />
                  </button>
                  <span class="ml-auto text-[11px] text-muted-foreground tabular-nums">
                    {formatCount(source.imageCount)} images
                  </span>
                </div>
              {/if}
              <p class="text-[11.5px] text-muted-foreground break-all font-mono">
                {source.path}
              </p>
            </div>
          </div>

          <div class="mt-2.5 flex items-center gap-2">
            <Button
              variant="outline"
              size="xs"
              onclick={() => handleRescan(source)}
              disabled={rescan?.busy}
            >
              <RefreshCw class={rescan?.busy ? "size-3 animate-spin" : "size-3"} />
              {rescan?.busy ? "Scanning…" : "Re-scan"}
            </Button>
            <Button
              variant="outline"
              size="xs"
              onclick={() => (removeTarget = source)}
            >
              <Trash2 class="size-3" />
              Remove
            </Button>
            {#if rescan?.status}
              <span class="text-[11px] text-muted-foreground ml-1">
                {rescan.status}
              </span>
            {/if}
          </div>
        </div>
      {/each}

      {#if sources.length === 0}
        <p class="text-[12px] text-muted-foreground italic">
          No source directories yet.
        </p>
      {/if}

      <div>
        <Button variant="outline" onclick={handleAdd} disabled={isAdding}>
          <Plus class="size-3" />
          {isAdding ? "Adding…" : "Add source directory"}
        </Button>
        {#if addError}
          <p class="text-[11.5px] text-destructive mt-2">{addError}</p>
        {/if}
      </div>
    </div>
  </section>

  <!-- Drive status — primary source only. Multi-source mount tracking is
       a future enhancement. -->
  <section class="mb-7">
    <h3 class="text-[14px] font-semibold text-foreground mb-1">Drive status</h3>
    <p class="text-[12px] text-muted-foreground mb-3">
      Live mount state for the first registered source.
    </p>
    <div
      class="rounded-md border border-border bg-secondary/40 p-3.5 text-[12px]"
    >
      {#if !$driveStatusReady}
        <div class="flex items-center gap-2 text-muted-foreground">
          <span class="w-1.5 h-1.5 rounded-full bg-muted-foreground/40"></span>
          Checking drive…
        </div>
      {:else if $driveStatus.connected}
        <div class="flex items-center justify-between gap-3 mb-2">
          <div class="flex items-center gap-2 min-w-0">
            <span class="w-1.5 h-1.5 rounded-full bg-success flex-shrink-0"></span>
            <span class="font-medium text-foreground">
              {$driveStatus.label ?? "Archive"}
            </span>
            <span class="text-muted-foreground">connected</span>
          </div>
          <Button
            variant="outline"
            size="xs"
            onclick={handleRetry}
            disabled={isRetrying}
          >
            <RefreshCw class={isRetrying ? "size-3 animate-spin" : "size-3"} />
            Refresh
          </Button>
        </div>
        <dl
          class="grid grid-cols-[max-content_1fr] gap-x-3 gap-y-1 text-[11.5px]"
        >
          <dt class="text-muted-foreground/80">Free space</dt>
          <dd class="text-foreground tabular-nums">
            {formatFileSize($driveStatus.availableBytes)}
            {#if $driveStatus.totalBytes !== null}
              <span class="text-muted-foreground/70">
                / {formatFileSize($driveStatus.totalBytes)}
              </span>
            {/if}
          </dd>
          <dt class="text-muted-foreground/80">Images indexed</dt>
          <dd class="text-foreground tabular-nums">
            {$driveStatus.imageCount !== null
              ? formatCount($driveStatus.imageCount)
              : "—"}
          </dd>
          {#if $driveStatus.mountPoint}
            <dt class="text-muted-foreground/80">Mount point</dt>
            <dd class="text-foreground font-mono text-[11px] break-all">
              {$driveStatus.mountPoint}
            </dd>
          {/if}
        </dl>
      {:else}
        <div class="flex items-center justify-between gap-3">
          <div class="flex items-center gap-2 min-w-0">
            <span class="w-1.5 h-1.5 rounded-full bg-destructive flex-shrink-0"></span>
            <span class="font-medium text-destructive">Disconnected</span>
            <span class="text-muted-foreground">
              archive directory is not reachable
            </span>
          </div>
          <Button
            variant="outline"
            size="xs"
            onclick={handleRetry}
            disabled={isRetrying}
          >
            <RefreshCw class={isRetrying ? "size-3 animate-spin" : "size-3"} />
            Retry
          </Button>
        </div>
      {/if}
    </div>
  </section>

  <!-- Danger zone -->
  <section>
    <h3 class="text-[14px] font-semibold text-destructive mb-1">Danger zone</h3>
    <p class="text-[12px] text-muted-foreground mb-3">
      Reset removes all indexed images, thumbnails, collections, and source
      directories. The original image files on disk are not affected.
    </p>
    <Button variant="outline" onclick={() => (showResetConfirm = true)}>
      Reset catalog…
    </Button>
  </section>
</div>

<!-- Remove-source confirm dialog -->
{#if removeTarget}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
    <div
      class="w-full max-w-md rounded-lg bg-background border border-border p-6"
      style="box-shadow: 0 24px 64px rgba(0,0,0,0.3);"
    >
      <h3 class="mb-2 text-base font-semibold text-foreground">
        Remove source directory?
      </h3>
      <p class="mb-3 text-sm text-muted-fg-2">
        Removing <strong class="text-foreground">{removeTarget.label}</strong>
        will also delete <strong class="text-foreground"
          >{formatCount(removeTarget.imageCount)} indexed images</strong
        > and any audit-log entries / collection memberships pointing at them.
        <strong class="text-foreground">The original image files on disk are not affected.</strong>
      </p>
      <div class="flex justify-end gap-2">
        <Button
          variant="outline"
          onclick={() => (removeTarget = null)}
          disabled={isRemoving}
        >
          Cancel
        </Button>
        <Button
          variant="destructive"
          disabled={isRemoving}
          onclick={confirmRemove}
        >
          {isRemoving ? "Removing…" : "Remove source"}
        </Button>
      </div>
    </div>
  </div>
{/if}

<!-- Reset-catalog confirm dialog -->
{#if showResetConfirm}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
    <div
      class="w-full max-w-md rounded-lg bg-background border border-border p-6"
      style="box-shadow: 0 24px 64px rgba(0,0,0,0.3);"
    >
      <h3 class="mb-2 text-base font-semibold text-foreground">
        Reset catalog?
      </h3>
      <p class="mb-3 text-sm text-muted-fg-2">
        This will remove all indexed images, delete cached thumbnails, drop
        every registered source directory, and clear collections.
        <strong class="text-foreground">The original image files are not affected.</strong>
      </p>
      <ul class="mb-4 list-disc list-inside space-y-1 text-sm text-muted-fg-2">
        <li>Remove all indexed images from the database</li>
        <li>Delete all cached thumbnails</li>
        <li>Clear all collections, smart collections, audit log</li>
        <li>Drop all source directories — you'll start over from setup</li>
      </ul>
      <div class="flex justify-end gap-2">
        <Button variant="outline" onclick={() => (showResetConfirm = false)}>
          Cancel
        </Button>
        <Button
          variant="destructive"
          disabled={isResetting}
          onclick={confirmReset}
        >
          {isResetting ? "Resetting…" : "Reset catalog"}
        </Button>
      </div>
      {#if resetError}
        <p class="mt-3 text-sm text-destructive">{resetError}</p>
      {/if}
    </div>
  </div>
{/if}
