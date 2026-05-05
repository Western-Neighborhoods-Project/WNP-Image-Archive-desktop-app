<script lang="ts">
  import ZoomIn from "@lucide/svelte/icons/zoom-in";
  import ZoomOut from "@lucide/svelte/icons/zoom-out";

  interface Props {
    zoomLevel: number;
    onZoomIn: () => void;
    onZoomOut: () => void;
    onFit: () => void;
  }

  let { zoomLevel, onZoomIn, onZoomOut, onFit }: Props = $props();
</script>

<div
  class="flex items-center gap-0.5 rounded-lg p-[3px]"
  style="background: rgba(24,24,27,0.9); backdrop-filter: blur(12px); border: 1px solid rgba(255,255,255,0.1);"
>
  <button
    type="button"
    onclick={onZoomOut}
    class="h-[26px] w-[26px] flex items-center justify-center rounded-[5px] hover:bg-white/10"
    style="color: rgba(255,255,255,0.8);"
    aria-label="Zoom out"
  >
    <ZoomOut size={14} />
  </button>
  <div
    class="px-2.5 flex items-center text-[11px] font-mono"
    style="color: rgba(255,255,255,0.8);"
  >
    {Math.round(zoomLevel * 100)}%
  </div>
  <button
    type="button"
    onclick={onZoomIn}
    class="h-[26px] w-[26px] flex items-center justify-center rounded-[5px] hover:bg-white/10"
    style="color: rgba(255,255,255,0.8);"
    aria-label="Zoom in"
  >
    <ZoomIn size={14} />
  </button>
  <div
    class="w-px self-stretch mx-0.5"
    style="background: rgba(255,255,255,0.15);"
  ></div>
  <button
    type="button"
    onclick={onFit}
    class="h-[26px] px-2.5 flex items-center rounded-[5px] hover:bg-white/10 text-[11px]"
    style="color: rgba(255,255,255,0.8);"
  >
    Fit
  </button>
</div>
