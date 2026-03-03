<script lang="ts">
  import { onMount } from 'svelte';
  import { getSetting } from '$lib/commands/settings';
  import { currentView, currentImageId } from '$lib/stores/navigation';
  import type { ImageRecord } from '$lib/commands/images';

  // Layout components
  import Sidebar from '$lib/components/layout/Sidebar.svelte';
  import TopBar from '$lib/components/layout/TopBar.svelte';
  import SettingsView from '$lib/components/layout/SettingsView.svelte';

  // Setup / Import
  import SetupScreen from '$lib/components/setup/SetupScreen.svelte';
  import ImportProgress from '$lib/components/setup/ImportProgress.svelte';

  // Browsing
  import Grid from '$lib/components/browsing/Grid.svelte';

  // ── State ──────────────────────────────────────────────────────────────────
  let sourceDirectory = $state<string | null>(null);
  let importDirectory = $state<string | null>(null); // Directory selected for a new import
  let appReady = $state(false);

  // ── Boot: check if catalog is already set up ────────────────────────────────
  onMount(async () => {
    try {
      const dir = await getSetting('source_directory');
      sourceDirectory = dir;
      if (dir) {
        currentView.set('library');
      } else {
        currentView.set('setup');
      }
    } catch (e) {
      currentView.set('setup');
    } finally {
      appReady = true;
    }
  });

  // ── Navigation callbacks ───────────────────────────────────────────────────
  function handleDirectorySelected(path: string) {
    importDirectory = path;
    sourceDirectory = path;
    currentView.set('import');
  }

  function handleImportComplete() {
    currentView.set('library');
  }

  function handleImageClick(image: ImageRecord) {
    currentImageId.set(image.id);
    // Detail view will be added in Phase 2
    console.log('Image clicked:', image.catalog_number);
  }

  function handleResetComplete() {
    sourceDirectory = null;
    importDirectory = null;
    currentView.set('setup');
  }
</script>

{#if !appReady}
  <!-- Splash / loading -->
  <div class="flex h-screen items-center justify-center bg-gray-50">
    <div class="h-6 w-6 animate-spin rounded-full border-2 border-blue-600 border-t-transparent"></div>
  </div>
{:else if $currentView === 'setup'}
  <div class="h-screen">
    <SetupScreen onDirectorySelected={handleDirectorySelected} />
  </div>
{:else if $currentView === 'import'}
  <div class="h-screen">
    <ImportProgress
      sourceDirectory={importDirectory ?? sourceDirectory ?? ''}
      onComplete={handleImportComplete}
    />
  </div>
{:else}
  <!-- Main app shell: sidebar + content -->
  <div class="flex h-screen overflow-hidden">
    <Sidebar />

    <div class="flex flex-1 flex-col overflow-hidden">
      {#if $currentView === 'settings'}
        <SettingsView onResetComplete={handleResetComplete} />
      {:else}
        <!-- Library view -->
        <TopBar />
        <main class="flex-1 overflow-hidden">
          <Grid onImageClick={handleImageClick} />
        </main>
      {/if}
    </div>
  </div>
{/if}
