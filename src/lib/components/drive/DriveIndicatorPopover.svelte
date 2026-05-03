<script lang="ts">
  // Drive indicator popover content — full stats view shown when the
  // user clicks the StatusBar drive indicator. Renders inside the
  // bits-ui Popover.Content from DriveIndicator.svelte.

  import { driveStatus } from "$lib/stores/driveStatus";
  import { revealDriveInFinder } from "$lib/commands/drive";
  import { formatFileSize, formatCount } from "$lib/utils/format";
  import { FolderOpen, HardDrive } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";

  // Top 3 formats by count, sorted descending.
  let topFormats = $derived.by(() => {
    const entries = Object.entries($driveStatus.formatMix ?? {});
    entries.sort((a, b) => b[1] - a[1]);
    return entries.slice(0, 3);
  });

  // Used % for the progress bar.
  let usedPct = $derived.by(() => {
    const total = $driveStatus.totalBytes;
    const avail = $driveStatus.availableBytes;
    if (total === null || avail === null || total === 0) return null;
    return Math.min(100, Math.max(0, ((total - avail) / total) * 100));
  });

  let usedBytes = $derived(
    $driveStatus.totalBytes !== null && $driveStatus.availableBytes !== null
      ? $driveStatus.totalBytes - $driveStatus.availableBytes
      : null,
  );

  function formatMountedDuration(ms: number | null): string {
    if (ms === null) return "—";
    const elapsed = Date.now() - ms;
    const seconds = Math.floor(elapsed / 1000);
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h`;
    const days = Math.floor(hours / 24);
    return `${days}d`;
  }

  async function handleReveal() {
    try {
      await revealDriveInFinder();
    } catch (e) {
      console.error("Reveal in Finder failed", e);
    }
  }
</script>

<div class="p-4 space-y-3.5">
  <!-- Header: drive name + status -->
  <div class="flex items-start gap-2.5">
    <div
      class="w-8 h-8 rounded-md bg-success/10 text-success flex items-center justify-center flex-shrink-0"
    >
      <HardDrive class="size-4" />
    </div>
    <div class="flex-1 min-w-0">
      <div class="text-[13px] font-semibold text-foreground truncate">
        {$driveStatus.label ?? "Archive drive"}
      </div>
      <div class="text-[11px] text-muted-foreground font-mono truncate">
        {$driveStatus.mountPoint ?? $driveStatus.sourceDirectory ?? "—"}
      </div>
    </div>
  </div>

  <!-- Used / total progress -->
  {#if usedPct !== null}
    <div>
      <div class="flex justify-between text-[11px] text-muted-foreground mb-1.5 tabular-nums">
        <span>{formatFileSize(usedBytes)} used</span>
        <span>{formatFileSize($driveStatus.totalBytes)} total</span>
      </div>
      <div class="h-1.5 bg-secondary rounded-full overflow-hidden">
        <div
          class="h-full bg-foreground/60 rounded-full transition-all"
          style="width: {usedPct}%"
        ></div>
      </div>
    </div>
  {/if}

  <!-- Image stats grid -->
  <div class="grid grid-cols-2 gap-x-3 gap-y-2 text-[11px]">
    <div>
      <div class="text-muted-foreground/70 uppercase tracking-wide text-[10px] mb-0.5">
        Images
      </div>
      <div class="text-foreground tabular-nums">
        {$driveStatus.imageCount !== null
          ? formatCount($driveStatus.imageCount)
          : "—"}
      </div>
    </div>
    <div>
      <div class="text-muted-foreground/70 uppercase tracking-wide text-[10px] mb-0.5">
        Mounted
      </div>
      <div class="text-foreground tabular-nums">
        {formatMountedDuration($driveStatus.mountedAtMs)}
      </div>
    </div>
  </div>

  <!-- Format mix -->
  {#if topFormats.length > 0}
    <div>
      <div
        class="text-muted-foreground/70 uppercase tracking-wide text-[10px] mb-1"
      >
        Formats
      </div>
      <div class="flex flex-wrap gap-1.5">
        {#each topFormats as [fmt, count] (fmt)}
          <span
            class="inline-flex items-center gap-1 rounded bg-secondary px-1.5 py-0.5 text-[10.5px] tabular-nums"
          >
            <span class="font-mono uppercase text-foreground">{fmt}</span>
            <span class="text-muted-foreground">{formatCount(count)}</span>
          </span>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Actions -->
  <div class="pt-1 flex gap-2 border-t border-border -mx-4 px-4 mt-3.5 pt-3">
    <Button
      variant="outline"
      size="xs"
      onclick={handleReveal}
      class="flex-1"
    >
      <FolderOpen class="size-3" />
      Reveal in Finder
    </Button>
  </div>
</div>
