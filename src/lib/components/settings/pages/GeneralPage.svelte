<script lang="ts">
  import { onMount } from "svelte";
  import { getSetting, resetCatalog } from "$lib/commands/settings";
  import { Button } from "$lib/components/ui/button";
  import { driveStatus, driveStatusReady } from "$lib/stores/driveStatus";
  import { retryDriveConnection } from "$lib/commands/drive";
  import { formatFileSize, formatCount } from "$lib/utils/format";
  import { RefreshCw } from "@lucide/svelte";

  let { onResetComplete }: { onResetComplete: () => void } = $props();

  let sourceDirectory = $state<string | null>(null);
  let showResetConfirm = $state(false);
  let isResetting = $state(false);
  let error = $state<string | null>(null);
  let isRetrying = $state(false);

  onMount(async () => {
    sourceDirectory = await getSetting("source_directory");
  });

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
    error = null;
    try {
      await resetCatalog();
      onResetComplete();
    } catch (e) {
      error = String(e);
    } finally {
      isResetting = false;
      showResetConfirm = false;
    }
  }
</script>

<div class="max-w-[720px]">
  <section class="mb-7">
    <h3 class="text-[14px] font-semibold text-foreground mb-1">
      Image archive directory
    </h3>
    <p class="text-[12px] text-muted-foreground mb-3 break-all font-mono">
      {sourceDirectory ?? "Not set"}
    </p>
    <Button variant="outline" onclick={() => (showResetConfirm = true)}>
      Change source directory…
    </Button>
  </section>

  <!-- Drive status — live mount state from the Plan 6 monitor. -->
  <section class="mb-7">
    <h3 class="text-[14px] font-semibold text-foreground mb-1">Drive status</h3>
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
</div>

{#if showResetConfirm}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
  >
    <div
      class="w-full max-w-md rounded-lg bg-background border border-border p-6"
      style="box-shadow: 0 24px 64px rgba(0,0,0,0.3);"
    >
      <h3 class="mb-2 text-base font-semibold text-foreground">
        Reset catalog?
      </h3>
      <p class="mb-3 text-sm text-muted-fg-2">
        This will remove all indexed images, delete cached thumbnails, and clear
        collections. <strong class="text-foreground">The original image files are not affected.</strong>
      </p>
      <ul class="mb-4 list-disc list-inside space-y-1 text-sm text-muted-fg-2">
        <li>Remove all indexed images from the database</li>
        <li>Delete all cached thumbnails</li>
        <li>Clear all collections and audit log</li>
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
          {isResetting ? "Resetting…" : "Reset and choose new directory"}
        </Button>
      </div>
      {#if error}
        <p class="mt-3 text-sm text-destructive">{error}</p>
      {/if}
    </div>
  </div>
{/if}
