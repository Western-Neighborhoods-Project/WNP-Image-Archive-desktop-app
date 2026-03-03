<script lang="ts">
  import {
    scanDirectory,
    extractMetadataBatch,
    extractExifThumbnailsBatch,
    getScanStats,
    type ScanResult,
    type MetadataImportResult,
    type ThumbnailResult,
    type ScanStats
  } from '$lib/commands/images';
  import { formatCount } from '$lib/utils/format';

  let { sourceDirectory, onComplete }: { sourceDirectory: string; onComplete: () => void } = $props();

  type Stage = 'scanning' | 'metadata' | 'thumbnails' | 'done' | 'error';
  let stage = $state<Stage>('scanning');
  let error = $state<string | null>(null);
  let metadataWarning = $state<string | null>(null);

  let scanResult = $state<ScanResult | null>(null);
  let metadataResult = $state<MetadataImportResult | null>(null);
  let thumbnailResult = $state<ThumbnailResult | null>(null);
  let stats = $state<ScanStats | null>(null);

  // Poll stats during long-running operations
  let pollInterval: ReturnType<typeof setInterval> | null = null;

  function startPolling() {
    pollInterval = setInterval(async () => {
      try {
        stats = await getScanStats();
      } catch {}
    }, 2000);
  }

  function stopPolling() {
    if (pollInterval !== null) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  }

  async function runImport() {
    try {
      // Stage 1: Scan
      stage = 'scanning';
      scanResult = await scanDirectory(sourceDirectory);
      stats = await getScanStats();

      // Stage 2: Metadata (non-fatal — exiftool may not be installed)
      stage = 'metadata';
      startPolling();
      try {
        metadataResult = await extractMetadataBatch(sourceDirectory);
      } catch (e) {
        // Metadata extraction failed (exiftool not found or errored).
        // Continue to thumbnail generation — thumbnails use the image crate,
        // not exiftool, so they'll still work.
        metadataWarning = String(e);
        console.warn('Metadata extraction failed, continuing:', e);
      }
      stopPolling();
      stats = await getScanStats();

      // Stage 3: Thumbnails
      stage = 'thumbnails';
      startPolling();
      thumbnailResult = await extractExifThumbnailsBatch();
      stopPolling();
      stats = await getScanStats();

      stage = 'done';
    } catch (e) {
      stopPolling();
      error = String(e);
      stage = 'error';
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

    <!-- Stage indicators -->
    <div class="mb-6 flex flex-col gap-3">
      {#each [
        { id: 'scanning', label: 'Scan directory', done: ['metadata','thumbnails','done'].includes(stage) },
        { id: 'metadata', label: 'Extract metadata', done: ['thumbnails','done'].includes(stage) },
        { id: 'thumbnails', label: 'Extract thumbnails', done: stage === 'done' },
      ] as step}
        <div class="flex items-center gap-3">
          <div class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full {step.done ? 'bg-green-500' : stage === step.id ? 'bg-blue-500' : 'bg-gray-200'}">
            {#if step.done}
              <span class="text-xs text-white">✓</span>
            {:else if stage === step.id}
              <span class="h-3 w-3 animate-spin rounded-full border-2 border-white border-t-transparent"></span>
            {/if}
          </div>
          <span class="text-sm {step.done ? 'text-gray-500 line-through' : stage === step.id ? 'font-medium text-gray-900' : 'text-gray-400'}">
            {step.label}
          </span>
        </div>
      {/each}
    </div>

    <!-- Live stats -->
    {#if stats}
      <div class="rounded-md bg-gray-50 px-4 py-3 text-sm text-gray-600">
        <div class="flex justify-between">
          <span>Images indexed</span>
          <span class="font-medium">{formatCount(stats.total_images)}</span>
        </div>
        <div class="mt-1 flex justify-between">
          <span>With thumbnails</span>
          <span class="font-medium">{formatCount(stats.images_with_thumbnails)}</span>
        </div>
        <div class="mt-1 flex justify-between">
          <span>Missing metadata</span>
          <span class="font-medium">{formatCount(stats.images_without_metadata)}</span>
        </div>
      </div>
    {/if}

    <!-- Metadata warning (non-fatal) -->
    {#if metadataWarning}
      <div class="mt-4 rounded-md bg-yellow-50 px-4 py-3 text-xs text-yellow-800">
        <strong>Metadata extraction skipped:</strong> ExifTool not found.
        Install it with <code class="font-mono">brew install exiftool</code> and re-import to populate metadata.
      </div>
    {/if}

    <!-- Stage-specific message -->
    {#if stage === 'scanning'}
      <p class="mt-4 text-sm text-gray-500">Scanning directory for image files…</p>
    {:else if stage === 'metadata'}
      <p class="mt-4 text-sm text-gray-500">
        Running ExifTool on {formatCount(scanResult?.total_files ?? 0)} images.
        This may take a few minutes.
      </p>
    {:else if stage === 'thumbnails'}
      <p class="mt-4 text-sm text-gray-500">Extracting embedded thumbnails…</p>
    {:else if stage === 'done'}
      <div class="mt-4 flex flex-col gap-3">
        <div class="rounded-md bg-green-50 px-4 py-3 text-sm text-green-700">
          Import complete! Found {formatCount(scanResult?.total_files ?? 0)} images
          in {formatCount(scanResult?.archive_collections_found ?? 0)} archive folders.
        </div>
        <button
          onclick={onComplete}
          class="rounded-lg bg-blue-600 px-4 py-3 text-sm font-medium text-white hover:bg-blue-700"
        >
          Browse Library
        </button>
      </div>
    {:else if stage === 'error'}
      <div class="mt-4 rounded-md bg-red-50 px-4 py-3 text-sm text-red-700">
        <strong>Import failed:</strong> {error}
      </div>
    {/if}
  </div>
</div>
