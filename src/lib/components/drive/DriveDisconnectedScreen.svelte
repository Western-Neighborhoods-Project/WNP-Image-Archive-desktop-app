<script lang="ts">
  // Drive disconnected nag screen — Plan 6.
  //
  // Covers the main content area when the archive drive is unreachable.
  // Per the Plan 6 decision (2026-05-01) this is "hard block": the only
  // ways out are the Retry button or navigating to Settings via the
  // sidebar. Sidebar stays visible so the user has an escape hatch.

  import { driveStatus } from "$lib/stores/driveStatus";
  import { retryDriveConnection } from "$lib/commands/drive";
  import { currentView } from "$lib/stores/navigation";
  import { Button } from "$lib/components/ui/button";
  import { HardDriveDownload, Settings, RefreshCw } from "@lucide/svelte";

  let retrying = $state(false);
  let lastError = $state<string | null>(null);

  async function handleRetry() {
    retrying = true;
    lastError = null;
    try {
      const result = await retryDriveConnection();
      if (!result.connected) {
        lastError = "Drive still not detected. Make sure it's plugged in and mounted.";
      }
    } catch (e) {
      lastError = String(e);
    } finally {
      retrying = false;
    }
  }

  function openSettings() {
    currentView.set("settings");
  }

  // Format a path for display: keep the basename + immediate parent.
  let pathSummary = $derived.by(() => {
    const path = $driveStatus.sourceDirectory;
    if (!path) return null;
    return path;
  });
</script>

<div
  class="absolute inset-0 z-40 flex items-center justify-center bg-background/95 backdrop-blur-sm overflow-auto"
>
  <div class="max-w-md w-full px-8 py-12 text-center">
    <!-- Icon -->
    <div
      class="mx-auto w-14 h-14 rounded-full bg-destructive/10 text-destructive flex items-center justify-center mb-5"
    >
      <HardDriveDownload class="size-7" />
    </div>

    <!-- Headline -->
    <h2 class="text-[18px] font-semibold text-foreground mb-2">
      Archive drive disconnected
    </h2>

    <p class="text-[13px] text-muted-foreground mb-1">
      The configured archive directory isn't reachable. Reconnect the drive
      and click Retry, or update the path in Settings.
    </p>

    {#if pathSummary}
      <p class="text-[11px] font-mono text-muted-foreground/80 break-all mb-6">
        {pathSummary}
      </p>
    {:else}
      <p class="text-[11px] text-muted-foreground/80 mb-6 italic">
        No archive directory configured.
      </p>
    {/if}

    <!-- Stub: last-backup info. Plan 8 will fill this in. -->
    <div
      class="text-[11px] text-muted-foreground/70 mb-6 py-2.5 px-3 rounded-md bg-muted/40 border border-border"
    >
      Backups not configured.
      <span class="text-muted-foreground/50">Plan 8 will track last-backup time here.</span>
    </div>

    <!-- Actions -->
    <div class="flex gap-2 justify-center">
      <Button
        variant="default"
        onclick={handleRetry}
        disabled={retrying}
      >
        <RefreshCw class={retrying ? "size-4 animate-spin" : "size-4"} />
        {retrying ? "Retrying…" : "Retry connection"}
      </Button>
      <Button variant="outline" onclick={openSettings}>
        <Settings class="size-4" />
        Open Settings
      </Button>
    </div>

    {#if lastError}
      <p class="mt-4 text-[12px] text-destructive">{lastError}</p>
    {/if}
  </div>
</div>
