<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { setSetting } from '$lib/commands/settings';

  let { onDirectorySelected }: { onDirectorySelected: (path: string) => void } = $props();

  let selectedPath = $state<string | null>(null);
  let error = $state<string | null>(null);
  let isSelecting = $state(false);

  async function pickDirectory() {
    isSelecting = true;
    error = null;
    try {
      const result = await open({ directory: true, multiple: false, title: 'Select Image Archive Directory' });
      if (result) {
        selectedPath = result as string;
      }
    } catch (e) {
      error = String(e);
    } finally {
      isSelecting = false;
    }
  }

  async function startImport() {
    if (!selectedPath) return;
    try {
      await setSetting('source_directory', selectedPath);
      onDirectorySelected(selectedPath);
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="flex h-full flex-col items-center justify-center gap-8 bg-gray-50 p-8">
  <div class="text-center">
    <h1 class="text-3xl font-semibold text-gray-900">Image Archive Manager</h1>
    <p class="mt-2 text-gray-500">Select the directory containing your image archive to get started.</p>
  </div>

  <div class="w-full max-w-lg rounded-xl border border-gray-200 bg-white p-8 shadow-sm">
    <div class="flex flex-col gap-4">
      <button
        onclick={pickDirectory}
        disabled={isSelecting}
        class="rounded-lg border border-gray-300 bg-white px-4 py-3 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50 disabled:opacity-50"
      >
        {isSelecting ? 'Opening...' : 'Select Directory'}
      </button>

      {#if selectedPath}
        <div class="rounded-md bg-gray-50 px-3 py-2 text-sm text-gray-600 break-all">
          {selectedPath}
        </div>

        <button
          onclick={startImport}
          class="rounded-lg bg-blue-600 px-4 py-3 text-sm font-medium text-white shadow-sm hover:bg-blue-700"
        >
          Start Import
        </button>
      {/if}

      {#if error}
        <p class="text-sm text-red-600">{error}</p>
      {/if}
    </div>
  </div>
</div>
