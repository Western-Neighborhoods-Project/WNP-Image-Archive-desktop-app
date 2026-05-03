<script lang="ts">
  import { windowTitle } from "$lib/stores/navigation";

  // ── Custom window chrome ────────────────────────────────────────
  // Tauri runs with `titleBarStyle: "Overlay"` + `hiddenTitle: true`,
  // which makes macOS's native titlebar invisible (the traffic lights
  // still render and stay functional, just overlaid on whatever we
  // draw in the top-left). This component fills that role: a 36px
  // bar across the top of the window, with traffic-light padding on
  // the left, the centered title in the middle, and a thin bottom
  // border to anchor it visually — matching the design exactly.
  //
  // The whole bar is a `data-tauri-drag-region` so the user can drag
  // to move the window the same way they would the native titlebar.
  // Traffic lights handle their own clicks.
</script>

<div
  data-tauri-drag-region
  class="h-9 flex-shrink-0 flex items-center justify-center bg-sidebar-bg border-b border-border"
  style="padding-left: 80px; padding-right: 80px;"
>
  <span
    data-tauri-drag-region
    class="text-[12px] font-medium text-muted-foreground select-none"
  >
    {$windowTitle}
  </span>
</div>
