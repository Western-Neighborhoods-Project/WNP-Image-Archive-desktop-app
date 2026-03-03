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

  // API + S3 settings
  let laravelApiUrl = $state('');
  let s3Endpoint = $state('');
  let s3Bucket = $state('');
  let s3AccessKey = $state('');
  let s3SecretKey = $state('');
  let s3Region = $state('');
  let s3PublicBaseUrl = $state('');

  // Resolution settings
  let resolutionHighPx = $state('2048');
  let resolutionMediumPx = $state('1600');
  let resolutionLowPx = $state('800');

  // Save feedback
  let saveStatus = $state<'idle' | 'saving' | 'saved'>('idle');
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  onMount(async () => {
    const [
      dir,
      apiUrl,
      endpoint,
      bucket,
      accessKey,
      secretKey,
      region,
      publicBaseUrl,
      highPx,
      mediumPx,
      lowPx,
    ] = await Promise.all([
      getSetting('source_directory'),
      getSetting('laravel_api_url'),
      getSetting('s3_endpoint'),
      getSetting('s3_bucket'),
      getSetting('s3_access_key'),
      getSetting('s3_secret_key'),
      getSetting('s3_region'),
      getSetting('s3_public_base_url'),
      getSetting('resolution_high_px'),
      getSetting('resolution_medium_px'),
      getSetting('resolution_low_px'),
    ]);
    sourceDirectory = dir;
    laravelApiUrl = apiUrl ?? '';
    s3Endpoint = endpoint ?? '';
    s3Bucket = bucket ?? '';
    s3AccessKey = accessKey ?? '';
    s3SecretKey = secretKey ?? '';
    s3Region = region ?? '';
    s3PublicBaseUrl = publicBaseUrl ?? '';
    resolutionHighPx = highPx ?? '2048';
    resolutionMediumPx = mediumPx ?? '1600';
    resolutionLowPx = lowPx ?? '800';
  });

  async function saveApiSettings() {
    saveStatus = 'saving';
    clearTimeout(saveTimer);
    try {
      await Promise.all([
        setSetting('laravel_api_url', laravelApiUrl),
        setSetting('s3_endpoint', s3Endpoint),
        setSetting('s3_bucket', s3Bucket),
        setSetting('s3_access_key', s3AccessKey),
        setSetting('s3_secret_key', s3SecretKey),
        setSetting('s3_region', s3Region),
        setSetting('s3_public_base_url', s3PublicBaseUrl),
        setSetting('resolution_high_px', resolutionHighPx),
        setSetting('resolution_medium_px', resolutionMediumPx),
        setSetting('resolution_low_px', resolutionLowPx),
      ]);
      saveStatus = 'saved';
      saveTimer = setTimeout(() => (saveStatus = 'idle'), 2000);
    } catch {
      saveStatus = 'idle';
    }
  }

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

    <!-- API & Integration -->
    <div class="rounded-lg border border-gray-200 bg-white p-6">
      <h3 class="mb-4 text-sm font-medium text-gray-900">API & Integration</h3>
      <div class="space-y-4">
        <div>
          <label for="api-url" class="mb-1 block text-xs font-medium text-gray-700">Laravel API Base URL</label>
          <input
            id="api-url"
            type="url"
            bind:value={laravelApiUrl}
            placeholder="https://yoursite.com"
            class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-800 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
          />
        </div>

        <div class="border-t border-gray-100 pt-4">
          <p class="mb-3 text-xs font-medium text-gray-700">S3-Compatible Storage</p>
          <div class="space-y-3">
            <div>
              <label for="s3-endpoint" class="mb-1 block text-xs text-gray-600">Endpoint URL</label>
              <input
                id="s3-endpoint"
                type="url"
                bind:value={s3Endpoint}
                placeholder="https://s3.us-east-1.backblazeb2.com"
                class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-800 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
              />
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div>
                <label for="s3-bucket" class="mb-1 block text-xs text-gray-600">Bucket</label>
                <input
                  id="s3-bucket"
                  type="text"
                  bind:value={s3Bucket}
                  placeholder="my-bucket"
                  class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-800 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                />
              </div>
              <div>
                <label for="s3-region" class="mb-1 block text-xs text-gray-600">Region</label>
                <input
                  id="s3-region"
                  type="text"
                  bind:value={s3Region}
                  placeholder="us-east-1"
                  class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-800 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                />
              </div>
            </div>
            <div>
              <label for="s3-access-key" class="mb-1 block text-xs text-gray-600">Access Key ID</label>
              <input
                id="s3-access-key"
                type="text"
                bind:value={s3AccessKey}
                placeholder="Key ID"
                class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-800 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
              />
            </div>
            <div>
              <label for="s3-secret-key" class="mb-1 block text-xs text-gray-600">Secret Access Key</label>
              <input
                id="s3-secret-key"
                type="password"
                bind:value={s3SecretKey}
                placeholder="Secret key"
                class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-800 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
              />
            </div>
            <div>
              <label for="s3-public-url" class="mb-1 block text-xs text-gray-600">Public Base URL</label>
              <input
                id="s3-public-url"
                type="url"
                bind:value={s3PublicBaseUrl}
                placeholder="https://files.yourcdn.com"
                class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-800 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
              />
              <p class="mt-1 text-xs text-gray-400">URL prefix used to build download links (e.g. CDN hostname)</p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Export Resolutions -->
    <div class="rounded-lg border border-gray-200 bg-white p-6">
      <h3 class="mb-1 text-sm font-medium text-gray-900">Export Resolutions</h3>
      <p class="mb-4 text-xs text-gray-500">Maximum pixel dimension (longest side) for each quality tier.</p>
      <div class="grid grid-cols-3 gap-4">
        <div>
          <label for="res-high" class="mb-1 block text-xs font-medium text-gray-700">High</label>
          <div class="flex items-center gap-1.5">
            <input
              id="res-high"
              type="number"
              min="512"
              max="8000"
              bind:value={resolutionHighPx}
              class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-800 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
            />
            <span class="text-xs text-gray-400 shrink-0">px</span>
          </div>
        </div>
        <div>
          <label for="res-medium" class="mb-1 block text-xs font-medium text-gray-700">Medium</label>
          <div class="flex items-center gap-1.5">
            <input
              id="res-medium"
              type="number"
              min="512"
              max="8000"
              bind:value={resolutionMediumPx}
              class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-800 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
            />
            <span class="text-xs text-gray-400 shrink-0">px</span>
          </div>
        </div>
        <div>
          <label for="res-low" class="mb-1 block text-xs font-medium text-gray-700">Low</label>
          <div class="flex items-center gap-1.5">
            <input
              id="res-low"
              type="number"
              min="256"
              max="8000"
              bind:value={resolutionLowPx}
              class="w-full rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-800 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
            />
            <span class="text-xs text-gray-400 shrink-0">px</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Save button -->
    <div class="flex items-center gap-3">
      <button
        onclick={saveApiSettings}
        disabled={saveStatus === 'saving'}
        class="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
      >
        {saveStatus === 'saving' ? 'Saving…' : 'Save Settings'}
      </button>
      {#if saveStatus === 'saved'}
        <span class="text-sm text-green-600">Saved</span>
      {/if}
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
