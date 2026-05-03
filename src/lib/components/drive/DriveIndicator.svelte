<script lang="ts">
  // Drive indicator — Plan 6.
  //
  // Lives inside every view's StatusBar slot. Three visual states:
  //   - Loading (initial fetch hasn't returned): grey dot, no label
  //   - Connected: green dot + drive label + free-space text
  //   - Disconnected: red dot + "Disconnected"
  //
  // Click toggles a popover with full drive stats. The popover lives in
  // DriveIndicatorPopover.svelte and is portalled out so it can render
  // above the StatusBar's overflow clip.

  import { Popover } from "bits-ui";
  import { driveStatus, driveStatusReady } from "$lib/stores/driveStatus";
  import { formatFileSize } from "$lib/utils/format";
  import DriveIndicatorPopover from "./DriveIndicatorPopover.svelte";
</script>

<Popover.Root>
  <Popover.Trigger
    class="inline-flex items-center gap-1.5 cursor-pointer hover:text-foreground transition-colors px-1 -mx-1 rounded"
  >
    {#if !$driveStatusReady}
      <span
        class="w-1.5 h-1.5 rounded-full bg-muted-foreground/40"
        aria-hidden="true"
      ></span>
      <span class="text-muted-foreground">Checking drive…</span>
    {:else if $driveStatus.connected}
      <span
        class="w-1.5 h-1.5 rounded-full bg-success"
        aria-hidden="true"
      ></span>
      <span>
        {$driveStatus.label ?? "Archive"}
        {#if $driveStatus.availableBytes !== null}
          <span class="text-muted-foreground/70">
            · {formatFileSize($driveStatus.availableBytes)} free
          </span>
        {/if}
      </span>
    {:else}
      <span
        class="w-1.5 h-1.5 rounded-full bg-destructive"
        aria-hidden="true"
      ></span>
      <span class="text-destructive">Disconnected</span>
    {/if}
  </Popover.Trigger>

  <Popover.Portal>
    <Popover.Content
      side="top"
      align="end"
      sideOffset={8}
      class="z-50 w-[320px] rounded-lg border border-border bg-popover text-popover-foreground shadow-lg outline-none"
    >
      <DriveIndicatorPopover />
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>
