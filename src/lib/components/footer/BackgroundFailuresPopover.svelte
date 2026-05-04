<script lang="ts">
  // Per-file failure detail for the BackgroundActivityIndicator pill.
  // Two simple tabs (Thumbnails | Metadata) — Plan 13's "stay lean" call
  // means we don't bother with fancy filtering; just list failed rows
  // with their error string and a retry button per tab.
  import { onMount } from "svelte";
  import {
    listThumbnailFailures,
    listMetadataFailures,
    retryFailedThumbnails,
    retryFailedMetadata,
    type FailureRecord,
  } from "$lib/commands/backgroundJobs";
  import { backgroundProgress } from "$lib/stores/backgroundProgress";
  import { Button } from "$lib/components/ui/button";
  import { RefreshCw } from "@lucide/svelte";

  type Tab = "thumbnails" | "metadata";
  let tab = $state<Tab>("thumbnails");
  let thumbFailures = $state<FailureRecord[]>([]);
  let metaFailures = $state<FailureRecord[]>([]);
  let isRetrying = $state(false);
  let loading = $state(true);

  async function refresh() {
    loading = true;
    try {
      const [t, m] = await Promise.all([
        listThumbnailFailures(50),
        listMetadataFailures(50),
      ]);
      thumbFailures = t;
      metaFailures = m;
    } catch (e) {
      console.error("Failed to load failures", e);
    } finally {
      loading = false;
    }
  }

  onMount(refresh);

  async function handleRetry() {
    isRetrying = true;
    try {
      if (tab === "thumbnails") {
        await retryFailedThumbnails();
      } else {
        await retryFailedMetadata();
      }
      await refresh();
    } catch (e) {
      console.error("Retry failed", e);
    } finally {
      isRetrying = false;
    }
  }

  let activeList = $derived(tab === "thumbnails" ? thumbFailures : metaFailures);
  let counts = $derived({
    thumbnails: $backgroundProgress.thumbnails.failed,
    metadata: $backgroundProgress.metadata.failed,
  });
</script>

<div class="flex flex-col">
  <!-- Tab switcher -->
  <div class="flex border-b border-border">
    <button
      type="button"
      onclick={() => (tab = "thumbnails")}
      class="flex-1 px-3 py-2 text-[12px] font-medium {tab === 'thumbnails'
        ? 'text-foreground border-b-2 border-primary -mb-px'
        : 'text-muted-foreground hover:text-foreground'}"
    >
      Thumbnails
      {#if counts.thumbnails > 0}
        <span class="ml-1 text-destructive">({counts.thumbnails})</span>
      {/if}
    </button>
    <button
      type="button"
      onclick={() => (tab = "metadata")}
      class="flex-1 px-3 py-2 text-[12px] font-medium {tab === 'metadata'
        ? 'text-foreground border-b-2 border-primary -mb-px'
        : 'text-muted-foreground hover:text-foreground'}"
    >
      Metadata
      {#if counts.metadata > 0}
        <span class="ml-1 text-destructive">({counts.metadata})</span>
      {/if}
    </button>
  </div>

  <!-- List -->
  <div class="max-h-[280px] overflow-auto p-2">
    {#if loading}
      <p class="px-2 py-4 text-[12px] text-muted-foreground italic">Loading…</p>
    {:else if activeList.length === 0}
      <p class="px-2 py-4 text-[12px] text-muted-foreground italic">
        No failures.
      </p>
    {:else}
      <ul class="flex flex-col gap-1.5">
        {#each activeList as failure (failure.imageId)}
          <li
            class="rounded-md border border-border bg-secondary/30 px-2.5 py-2"
          >
            <div class="text-[12px] font-medium text-foreground">
              {failure.catalogNumber}
            </div>
            <div
              class="text-[11px] text-muted-foreground break-all font-mono"
              title={failure.filePath}
            >
              {failure.filePath}
            </div>
            {#if failure.error}
              <div class="mt-1 text-[11px] text-destructive break-words">
                {failure.error}
              </div>
            {/if}
          </li>
        {/each}
      </ul>
      {#if (tab === "thumbnails" ? counts.thumbnails : counts.metadata) > activeList.length}
        <p class="px-2 pt-2 text-[11px] text-muted-foreground italic">
          Showing first {activeList.length} of
          {tab === "thumbnails" ? counts.thumbnails : counts.metadata}.
        </p>
      {/if}
    {/if}
  </div>

  <!-- Retry footer -->
  <div class="flex items-center justify-between gap-2 border-t border-border px-3 py-2.5">
    <span class="text-[11px] text-muted-foreground">
      Retry pushes failed rows back to pending.
    </span>
    <Button
      variant="outline"
      size="xs"
      disabled={isRetrying || activeList.length === 0}
      onclick={handleRetry}
    >
      <RefreshCw class={isRetrying ? "size-3 animate-spin" : "size-3"} />
      {isRetrying ? "Retrying…" : "Retry all"}
    </Button>
  </div>
</div>
