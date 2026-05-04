<script lang="ts">
  import {
    scanDirectory,
    getScanStats,
    type ScanResult,
    type ScanStats,
  } from "$lib/commands/images";
  import { formatCount } from "$lib/utils/format";

  let { sourceDirectory, onComplete }: { sourceDirectory: string; onComplete: () => void } =
    $props();

  // Plan 13: scan returns immediately. Metadata + thumbnail extraction
  // run in the background worker now, surfaced via the footer indicator.
  // Setup is just "scan, route to library" — the user can browse while
  // the worker processes the rest.
  type Stage = "scanning" | "done" | "error";
  let stage = $state<Stage>("scanning");
  let error = $state<string | null>(null);

  let scanResult = $state<ScanResult | null>(null);
  let stats = $state<ScanStats | null>(null);

  async function runImport() {
    try {
      stage = "scanning";
      scanResult = await scanDirectory(sourceDirectory);
      try {
        stats = await getScanStats();
      } catch {
        // non-fatal
      }
      stage = "done";
    } catch (e) {
      error = String(e);
      stage = "error";
    }
  }

  // Start import on mount
  $effect(() => {
    runImport();
  });
</script>

<div class="flex h-full flex-col items-center justify-center gap-6 bg-gray-50 p-8">
  <div class="w-full max-w-lg rounded-xl border border-gray-200 bg-white p-8 shadow-sm">
    <h2 class="mb-6 text-xl font-semibold text-gray-900">Importing Archive</h2>

    <!-- Stage indicator (single stage now — scan only) -->
    <div class="mb-6 flex items-center gap-3">
      <div
        class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full {stage === 'done'
          ? 'bg-green-500'
          : stage === 'scanning'
            ? 'bg-blue-500'
            : stage === 'error'
              ? 'bg-red-500'
              : 'bg-gray-200'}"
      >
        {#if stage === "done"}
          <span class="text-xs text-white">✓</span>
        {:else if stage === "scanning"}
          <span class="h-3 w-3 animate-spin rounded-full border-2 border-white border-t-transparent"></span>
        {:else if stage === "error"}
          <span class="text-xs text-white">!</span>
        {/if}
      </div>
      <span
        class="text-sm {stage === 'done'
          ? 'text-gray-500'
          : stage === 'scanning'
            ? 'font-medium text-gray-900'
            : 'text-gray-400'}"
      >
        Scanning archive directory
      </span>
    </div>

    <!-- Live stats -->
    {#if stats}
      <div class="rounded-md bg-gray-50 px-4 py-3 text-sm text-gray-600">
        <div class="flex justify-between">
          <span>Images indexed</span>
          <span class="font-medium">{formatCount(stats.total_images)}</span>
        </div>
      </div>
    {/if}

    <!-- Stage-specific message -->
    {#if stage === "scanning"}
      <p class="mt-4 text-sm text-gray-500">Scanning directory for image files…</p>
    {:else if stage === "done"}
      <div class="mt-4 flex flex-col gap-3">
        <div class="rounded-md bg-green-50 px-4 py-3 text-sm text-green-700">
          Indexed {formatCount(scanResult?.total_files ?? 0)} images. Thumbnails
          and metadata generate in the background — watch the footer indicator
          for progress.
        </div>
        <button
          onclick={onComplete}
          class="rounded-lg bg-blue-600 px-4 py-3 text-sm font-medium text-white hover:bg-blue-700"
        >
          Browse Library
        </button>
      </div>
    {:else if stage === "error"}
      <div class="mt-4 rounded-md bg-red-50 px-4 py-3 text-sm text-red-700">
        <strong>Import failed:</strong> {error}
      </div>
    {/if}
  </div>
</div>
