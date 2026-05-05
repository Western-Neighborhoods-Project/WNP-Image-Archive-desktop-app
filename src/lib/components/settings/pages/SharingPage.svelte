<script lang="ts">
  import { onMount } from "svelte";
  import { getPublicSetting, setSetting } from "$lib/commands/settings";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";

  let resolutionHighPx = $state("2048");
  let resolutionMediumPx = $state("1600");
  let resolutionLowPx = $state("800");

  let saveStatus = $state<"idle" | "saving" | "saved">("idle");
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  onMount(async () => {
    const [hi, med, lo] = await Promise.all([
      getPublicSetting("resolution_high_px"),
      getPublicSetting("resolution_medium_px"),
      getPublicSetting("resolution_low_px"),
    ]);
    resolutionHighPx = hi ?? "2048";
    resolutionMediumPx = med ?? "1600";
    resolutionLowPx = lo ?? "800";
  });

  async function save() {
    saveStatus = "saving";
    clearTimeout(saveTimer);
    try {
      await Promise.all([
        setSetting("resolution_high_px", resolutionHighPx),
        setSetting("resolution_medium_px", resolutionMediumPx),
        setSetting("resolution_low_px", resolutionLowPx),
      ]);
      saveStatus = "saved";
      saveTimer = setTimeout(() => (saveStatus = "idle"), 2000);
    } catch {
      saveStatus = "idle";
    }
  }
</script>

<div class="max-w-[640px] space-y-6">
  <section>
    <h3 class="text-[14px] font-semibold text-foreground mb-1">
      Export resolutions
    </h3>
    <p class="text-[12px] text-muted-foreground mb-3">
      Maximum pixel dimension (longest side) for each quality tier when
      fulfilling orders or sharing images.
    </p>
    <div class="grid grid-cols-3 gap-4">
      <div class="space-y-1.5">
        <label
          for="res-high"
          class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
          >High</label
        >
        <div class="flex items-center gap-1.5">
          <Input
            id="res-high"
            type="number"
            min="512"
            max="8000"
            bind:value={resolutionHighPx}
          />
          <span class="text-xs text-muted-foreground">px</span>
        </div>
      </div>
      <div class="space-y-1.5">
        <label
          for="res-medium"
          class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
          >Medium</label
        >
        <div class="flex items-center gap-1.5">
          <Input
            id="res-medium"
            type="number"
            min="512"
            max="8000"
            bind:value={resolutionMediumPx}
          />
          <span class="text-xs text-muted-foreground">px</span>
        </div>
      </div>
      <div class="space-y-1.5">
        <label
          for="res-low"
          class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
          >Low</label
        >
        <div class="flex items-center gap-1.5">
          <Input
            id="res-low"
            type="number"
            min="256"
            max="8000"
            bind:value={resolutionLowPx}
          />
          <span class="text-xs text-muted-foreground">px</span>
        </div>
      </div>
    </div>
  </section>

  <div class="flex items-center gap-3 pt-2">
    <Button disabled={saveStatus === "saving"} onclick={save}>
      {saveStatus === "saving" ? "Saving…" : "Save"}
    </Button>
    {#if saveStatus === "saved"}
      <span class="text-sm text-success">Saved</span>
    {/if}
  </div>
</div>
