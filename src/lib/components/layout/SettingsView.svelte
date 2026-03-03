<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { getSetting, setSetting, resetCatalog } from '$lib/commands/settings';
  import { currentView } from '$lib/stores/navigation';
  import { onMount } from 'svelte';

  let { onResetComplete }: { onResetComplete: () => void } = $props();

  let sourceDirectory = $state<string | null>(null);
  let showResetConfirm = $state(false);
  let isResetting = $state(false);
  let error = $state<string | null>(null);

  onMount(async () => {
    sourceDirectory = await getSetting('source_directory');
  });

  async function changeDirectory() {
    showResetConfirm = true;
  }

  async function confirmReset() {
    isResetting = true;
    error = null;
    try {
      await resetCatalog();
      onResetComplete();
    } catch (e) {
      error = String(e);
    } finally {
      isResetting = false;
      showResetConfirm = false;
    }
  }
</script>

<div class="flex h-full flex-col overflow-y-auto p-8">
  <div class="mb-6">
    <h2 class="text-xl font-semibold text-gray-900">Settings</h2>
  </div>

  <div class="max-w-lg space-y-6">
    <!-- Source Directory -->
    <div class="rounded-lg border border-gray-200 bg-white p-6">
      <h3 class="mb-1 text-sm font-medium text-gray-900">Image Archive Directory</h3>
      <p class="mb-3 text-xs text-gray-500 break-all">{sourceDirectory ?? 'Not set'}</p>
      <button
        onclick={changeDirectory}
        class="rounded-md border border-gray-300 bg-white px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50"
      >
        Change Source Directory
      </button>
    </div>
  </div>

  <!-- Reset confirmation dialog -->
  {#if showResetConfirm}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div class="w-full max-w-md rounded-xl bg-white p-6 shadow-xl">
        <h3 class="mb-2 text-base font-semibold text-gray-900">⚠️ Reset Catalog?</h3>
        <p class="mb-4 text-sm text-gray-600">
          This will remove all indexed images, delete cached thumbnails, and clear all collections.
          <strong>The original image files are not affected.</strong>
        </p>
        <ul class="mb-4 list-inside list-disc space-y-1 text-sm text-gray-600">
          <li>Remove all indexed images from the database</li>
          <li>Delete all cached thumbnails</li>
          <li>Clear all collections and audit logs</li>
        </ul>
        <div class="flex justify-end gap-3">
          <button
            onclick={() => (showResetConfirm = false)}
            class="rounded-md border border-gray-300 px-4 py-2 text-sm text-gray-700 hover:bg-gray-50"
          >
            Cancel
          </button>
          <button
            onclick={confirmReset}
            disabled={isResetting}
            class="rounded-md bg-red-600 px-4 py-2 text-sm text-white hover:bg-red-700 disabled:opacity-50"
          >
            {isResetting ? 'Resetting…' : 'Reset and Choose New Directory'}
          </button>
        </div>
        {#if error}
          <p class="mt-3 text-sm text-red-600">{error}</p>
        {/if}
      </div>
    </div>
  {/if}
</div>
