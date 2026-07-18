<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listSourceDirectories } from '$lib/commands/sources';
  import { currentView, currentImageId, windowTitle, openImageDetail } from '$lib/stores/navigation';
  import { goToAllImages } from '$lib/stores/filters';
  import type { ImageRecord } from '$lib/commands/images';
  import { installShortcuts } from '$lib/utils/keyboardShortcuts';

  // Layout components
  import Sidebar from '$lib/components/layout/Sidebar.svelte';
  import SettingsView from '$lib/components/layout/SettingsView.svelte';

  // Setup / Import
  import SetupScreen from '$lib/components/setup/SetupScreen.svelte';
  import ImportProgress from '$lib/components/setup/ImportProgress.svelte';

  // Browsing
  import LibraryView from '$lib/components/browsing/LibraryView.svelte';
  import RecentlyViewedView from '$lib/components/browsing/RecentlyViewedView.svelte';

  // Detail
  import DetailView from '$lib/components/detail/DetailView.svelte';

  // Requests
  import RequestsView from '$lib/components/requests/RequestsView.svelte';

  // Audit log (Plan 4)
  import AuditLogView from '$lib/components/audit/AuditLogView.svelte';


  // Command bar (Plan 3)
  import CommandBar from '$lib/components/command-bar/CommandBar.svelte';
  import ShortcutsHelp from '$lib/components/shortcuts/ShortcutsHelp.svelte';

  // Custom window chrome (replaces native macOS titlebar visuals)
  import WindowChrome from '$lib/components/layout/WindowChrome.svelte';

  // Drive monitoring (Plan 6) — store init + disconnect overlay
  import { driveDisconnected, initDriveStatusListener, refreshDriveStatus } from '$lib/stores/driveStatus';
  import DriveDisconnectedScreen from '$lib/components/drive/DriveDisconnectedScreen.svelte';

  // Background jobs (Plan 13) — store init for the footer indicator
  import { initBackgroundProgressListener } from '$lib/stores/backgroundProgress';

  // Per-image thumbnail-ready events → grid item refresh
  import { initThumbnailReadyListener } from '$lib/utils/thumbnailQueue';

  // Auto-updater
  import { checkForUpdates } from '$lib/updater';
  import UpdateProgressOverlay from '$lib/components/updater/UpdateProgressOverlay.svelte';

  // Local user management (Plan 10)
  import {
    currentUser,
    currentUserRole,
    authReady,
    initAuthListener,
  } from '$lib/stores/currentUser';
  import { logout } from '$lib/commands/auth';
  import LoginScreen from '$lib/components/auth/LoginScreen.svelte';
  import {
    inactivityTimeoutMinutes,
    loadInactivityTimeout,
  } from '$lib/stores/inactivityTimeout';
  import { installInactivityTimer } from '$lib/utils/inactivityTimer';
  import { get as storeGet } from 'svelte/store';

  // ── State ──────────────────────────────────────────────────────────────────
  let sourceDirectory = $state<string | null>(null);
  let importDirectory = $state<string | null>(null);
  let appReady = $state(false);
  /// Tracks whether we've decided the initial view for the active session.
  /// Reset on logout so a relogin re-fetches in case settings changed.
  let initialViewDecided = $state(false);

  // ── Boot: render is ready as soon as mount runs ────────────────────────────
  // The actual library/setup decision waits until login, since
  // get_public_setting requires an active session (Plan 11 defence in depth).
  onMount(() => {
    appReady = true;
    // Silent update check on every boot — only surfaces UI if a newer
    // version is available, otherwise no-op. Errors are swallowed so
    // an update endpoint hiccup doesn't disrupt the boot flow. The
    // user can also trigger a manual check from the user menu.
    void checkForUpdates({ interactive: false });
  });

  // ── Post-login: decide between library and setup ──────────────────────────
  // Plan 12: route to library if any source_directory is registered;
  // otherwise the first-run setup screen lets the user pick one.
  $effect(() => {
    if (!$currentUser) {
      // User logged out (or hasn't logged in yet) — reset so the next login
      // picks up any source-directory changes made elsewhere.
      initialViewDecided = false;
      sourceDirectory = null;
      return;
    }
    if (initialViewDecided) return;
    initialViewDecided = true;

    // These reads require a session (get_public_setting / get_drive_status),
    // so the pre-login fetches in onMount failed. Now that a session exists,
    // refresh them — otherwise the inactivity timeout stays at its default and
    // the drive-disconnected overlay can't fire.
    void loadInactivityTimeout();
    void refreshDriveStatus();

    (async () => {
      try {
        const sources = await listSourceDirectories();
        if (sources.length > 0) {
          sourceDirectory = sources[0]?.path ?? null;
          currentView.set('library');
        } else {
          sourceDirectory = null;
          currentView.set('setup');
        }
      } catch (e) {
        console.error('Failed to load source directories', e);
        currentView.set('setup');
      }
    })();
  });

  // ── Global keyboard shortcuts (⌘K + G-chords) ──────────────────────────────
  // Installed once at app boot, lives for the session. Each chord goes
  // straight to the corresponding view — same destinations the sidebar
  // shows kbd hints for.
  let uninstallShortcuts: (() => void) | null = null;

  onMount(() => {
    uninstallShortcuts = installShortcuts({
      chords: {
        a: () => goToAllImages(),
        r: () => currentView.set('recently-viewed'),
        q: () => currentView.set('requests'),
        l: () => currentView.set('audit'),
        s: () => currentView.set('settings'),
      },
      cmdKey: {
        // ⌘; — also opens settings. (⌘, is the macOS standard but it's
        // explicitly used by the OS for Preferences in some contexts;
        // the user picked ⌘; here for safety.)
        ';': () => currentView.set('settings'),
      },
      cmdShiftKey: {
        // ⌘⇧L — log out (Plan 10).
        l: () => {
          logout().catch((e) => console.error('logout failed', e));
        },
      },
    });
  });

  onDestroy(() => {
    uninstallShortcuts?.();
    uninstallDriveListener?.();
    uninstallBackgroundProgressListener?.();
    uninstallThumbnailReadyListener?.();
    uninstallAuthListener?.();
    uninstallInactivityTimer?.();
  });

  // ── Drive monitor subscription ─────────────────────────────────────────────
  // Subscribe to Tauri "drive:status" events for the life of the app and
  // hydrate the store with an initial fetch. Cleanup runs on destroy.
  let uninstallDriveListener: (() => void) | null = null;
  onMount(async () => {
    uninstallDriveListener = await initDriveStatusListener();
  });

  // ── Background jobs subscription (Plan 13) ─────────────────────────────────
  // Mirrors the drive-status pattern. Plumbs `background:progress` events
  // into the backgroundProgress store; the footer indicator subscribes.
  let uninstallBackgroundProgressListener: (() => void) | null = null;
  onMount(async () => {
    uninstallBackgroundProgressListener = await initBackgroundProgressListener();
  });

  // ── Per-image thumbnail-ready events ───────────────────────────────────────
  // Routes backend `thumbnail:ready` events to the grid item for each id, so
  // thumbnails pop in as soon as they're committed rather than per-batch.
  let uninstallThumbnailReadyListener: (() => void) | null = null;
  onMount(async () => {
    uninstallThumbnailReadyListener = await initThumbnailReadyListener();
  });

  // ── Auth + inactivity ─────────────────────────────────────────────────────
  let uninstallAuthListener: (() => void) | null = null;
  let uninstallInactivityTimer: (() => void) | null = null;
  onMount(async () => {
    // Hydrate auth state. The inactivity-timeout setting can't be read yet
    // (get_public_setting requires a session); the post-login effect above
    // loads it once a session exists.
    uninstallAuthListener = await initAuthListener();

    // Install global inactivity timer. Calls logout() after N minutes of no
    // mouse / keyboard activity. The timer is always running but a logout
    // when no session exists is a backend no-op.
    uninstallInactivityTimer = installInactivityTimer({
      getTimeoutMs: () => storeGet(inactivityTimeoutMinutes) * 60_000,
      onExpired: () => {
        logout().catch((e) => console.error('inactivity logout failed', e));
      },
    });
  });

  // ── Editor role guard ─────────────────────────────────────────────────────
  // Editors don't see Settings in the sidebar, but the chord shortcut G+S
  // and the ⌘; shortcut still set currentView. This effect catches that
  // and bounces them back to library.
  $effect(() => {
    if ($currentUserRole === 'editor' && $currentView === 'settings') {
      currentView.set('library');
    }
  });

  // ── Window title sync ───────────────────────────────────────────────────────
  // Our custom WindowChrome component is what users see; this effect just
  // mirrors the same string into the OS-level window title so macOS dock,
  // Cmd+Tab, and Mission Control all show the right name. Single derived
  // store (`windowTitle`) drives both, so they can't drift.
  $effect(() => {
    if (!appReady) return;
    const title = $windowTitle;
    getCurrentWindow()
      .setTitle(title)
      .catch((e) => console.error('setTitle failed', e));
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
    openImageDetail(image.id, scrollOffset);
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

<!-- Suppress the browser's default context menu globally. Custom menus
     (bits-ui ContextMenu) handle the contextmenu event earlier in the
     bubble chain and open their own UI; surfaces without a custom menu
     simply do nothing on Ctrl/right-click. -->
<svelte:window oncontextmenu={(e) => e.preventDefault()} />

{#if !appReady || !$authReady}
  <div class="flex h-screen items-center justify-center bg-gray-50">
    <div class="h-6 w-6 animate-spin rounded-full border-2 border-blue-600 border-t-transparent"></div>
  </div>

{:else if !$currentUser}
  <!-- Plan 10 — login (or first-admin bootstrap). Window chrome stays
       visible so the user can drag the window; LoginScreen owns the rest. -->
  <div class="flex flex-col h-screen overflow-hidden">
    <WindowChrome />
    <div class="flex-1 min-h-0 relative">
      <LoginScreen />
    </div>
  </div>

{:else if $currentView === 'setup'}
  <div class="flex flex-col h-screen overflow-hidden">
    <WindowChrome />
    <div class="flex-1 min-h-0">
      <SetupScreen onDirectorySelected={handleDirectorySelected} />
    </div>
  </div>

{:else if $currentView === 'import'}
  <div class="flex flex-col h-screen overflow-hidden">
    <WindowChrome />
    <div class="flex-1 min-h-0">
      <ImportProgress
        sourceDirectory={importDirectory ?? sourceDirectory ?? ''}
        onComplete={handleImportComplete}
      />
    </div>
  </div>

{:else}
  <div class="flex flex-col h-screen overflow-hidden">
    <WindowChrome />

    <div class="flex flex-1 overflow-hidden min-h-0">
      <Sidebar />

      <div class="relative flex flex-1 flex-col overflow-hidden">
      {#if $currentView === 'settings'}
        <SettingsView onResetComplete={handleResetComplete} />

      {:else if $currentView === 'requests'}
        <RequestsView />

      {:else if $currentView === 'detail' && $currentImageId !== null}
        <DetailView
          imageId={$currentImageId}
          onBack={handleBackToLibrary}
        />

      {:else if $currentView === 'audit'}
        <AuditLogView />

      {:else if $currentView === 'recently-viewed'}
        <RecentlyViewedView />

      {:else}
        <LibraryView onImageClick={handleImageClick} />
      {/if}

      <!-- Plan 6 hard-block overlay. Sidebar stays usable, Settings is
           the escape hatch (so we don't render the overlay there). -->
      {#if $driveDisconnected && $currentView !== 'settings'}
        <DriveDisconnectedScreen />
      {/if}
      </div>
    </div>
  </div>

  <!-- Command bar lives above everything; rendered as soon as the app
       is past setup so ⌘K works from any view. -->
  <CommandBar />
  <ShortcutsHelp />
{/if}

<!-- Updater progress overlay — shown only while a download or install
     is in flight. Lives outside the auth/view branching so it can
     surface even on the login screen. -->
<UpdateProgressOverlay />
