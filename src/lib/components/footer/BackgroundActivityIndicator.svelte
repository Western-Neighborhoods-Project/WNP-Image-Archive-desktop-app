<script lang="ts">
  // Plan 13 footer indicator. Sits next to the drive indicator in
  // every view's StatusBar. Three visual states:
  //   - Idle (no pending, no failed): hidden entirely (avoids visual noise)
  //   - Active (pending > 0 or busy): spinner + mini progress bar +
  //     "N of M" remaining-vs-total count
  //   - Failed (failures > 0): destructive pill with count + click for popover
  //
  // The popover lists the per-file failures (one tab for thumbnails, one
  // for metadata) and offers a single "Retry all" button per tab.
  import { Popover } from "bits-ui";
  import {
    backgroundProgress,
    isProcessing,
    totalFailures,
  } from "$lib/stores/backgroundProgress";
  import { formatCount } from "$lib/utils/format";
  import { Loader2, AlertTriangle } from "@lucide/svelte";
  import BackgroundFailuresPopover from "./BackgroundFailuresPopover.svelte";

  // Counts at the image level so 82 images displays as `N / 82`,
  // not `N / 164` from summing thumbnail + metadata jobs separately.
  let totalPending = $derived($backgroundProgress.images.pending);
  let totalResolved = $derived($backgroundProgress.images.resolved);
  let totalWork = $derived($backgroundProgress.images.total);
  let percent = $derived(
    totalWork > 0 ? Math.min(100, Math.round((totalResolved / totalWork) * 100)) : 0,
  );
</script>

{#if $totalFailures > 0}
  <Popover.Root>
    <Popover.Trigger
      class="inline-flex items-center gap-1.5 cursor-pointer hover:opacity-80 transition-opacity px-1 -mx-1 rounded text-destructive"
    >
      <AlertTriangle class="size-3" />
      <span>{$totalFailures} failed</span>
    </Popover.Trigger>
    <Popover.Portal>
      <Popover.Content
        side="top"
        align="end"
        sideOffset={8}
        class="z-50 w-[420px] rounded-lg border border-border bg-popover text-popover-foreground shadow-lg outline-none"
      >
        <BackgroundFailuresPopover />
      </Popover.Content>
    </Popover.Portal>
  </Popover.Root>
{:else if $isProcessing && totalPending > 0}
  <span class="inline-flex items-center gap-1.5 text-muted-foreground">
    <Loader2 class="size-3 animate-spin flex-shrink-0" />
    <span
      class="h-1 w-16 rounded-full bg-muted overflow-hidden"
      role="progressbar"
      aria-valuenow={percent}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-label="Processing progress"
    >
      <span
        class="block h-full bg-primary transition-[width] duration-300 ease-out"
        style="width: {percent}%"
      ></span>
    </span>
    <span class="tabular-nums">
      {formatCount(totalResolved)} / {formatCount(totalWork)}
    </span>
  </span>
{:else if $isProcessing}
  <span class="inline-flex items-center gap-1.5 text-muted-foreground">
    <Loader2 class="size-3 animate-spin" />
    <span>Processing…</span>
  </span>
{/if}
