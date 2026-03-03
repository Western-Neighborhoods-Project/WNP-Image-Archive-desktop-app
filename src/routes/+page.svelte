<script lang="ts">
  import { onMount } from 'svelte';
  import { getSetting } from '$lib/commands/settings';
  import { currentView, currentImageId, savedScrollOffset } from '$lib/stores/navigation';
  import type { ImageRecord } from '$lib/commands/images';

  // Layout components
  import Sidebar from '$lib/components/layout/Sidebar.svelte';
  import TopBar from '$lib/components/layout/TopBar.svelte';
  import FilterBar from '$lib/components/layout/FilterBar.svelte';
  import SettingsView from '$lib/components/layout/SettingsView.svelte';

  // Setup / Import
  import SetupScreen from '$lib/components/setup/SetupScreen.svelte';
  import ImportProgress from '$lib/components/setup/ImportProgress.svelte';

  // Browsing
  import Grid from '$lib/components/browsing/Grid.svelte';

  // Detail
  import DetailView from '$lib/components/detail/DetailView.svelte';

  // Requests
  import RequestsView from '$lib/components/requests/RequestsView.svelte';

  // ── State ──────────────────────────────────────────────────────────────────
  let sourceDirectory = $state<string | null>(null);
  let importDirectory = $state<string | null>(null);
  let appReady = $state(false);

  // ── Boot: check if catalog is already set up ────────────────────────────────
  onMount(async () => {
    try {
      const dir = await getSetting('source_directory');
      sourceDirectory = dir;
      currentView.set(dir ? 'library' : 'setup');
    } catch {
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

  function handleImageClick(image: ImageRecord, scrollOffset: number) {
    savedScrollOffset.set(scrollOffset);
    currentImageId.set(image.id);
    currentView.set('detail');
  }

  function handleBackToLibrary() {
    currentView.set('library');
    // Scroll restore is handled by Grid reading savedScrollOffset on mount
  }

  function handleResetComplete() {
    sourceDirectory = null;
    importDirectory = null;
    currentView.set('setup');
  }
</script>

{#if !appReady}
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
  <div class="flex h-screen overflow-hidden">
    <Sidebar />

    <div class="flex flex-1 flex-col overflow-hidden">
      {#if $currentView === 'settings'}
        <SettingsView onResetComplete={handleResetComplete} />

      {:else if $currentView === 'requests'}
        <RequestsView />

      {:else if $currentView === 'detail' && $currentImageId !== null}
        <DetailView
          imageId={$currentImageId}
          onBack={handleBackToLibrary}
        />

      {:else}
        <!-- Library view: top bar + filter bar + grid -->
        <TopBar />
        <FilterBar />
        <main class="flex-1 overflow-hidden">
          <Grid onImageClick={handleImageClick} />
        </main>
      {/if}
    </div>
  </div>
{/if}
