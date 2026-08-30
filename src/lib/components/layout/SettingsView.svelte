<script lang="ts">
  import { PageHeader } from "$lib/components/ui/page-header";
  import {
    currentSettingsPage,
    type SettingsPageKey,
  } from "$lib/stores/navigation";
  import SettingsNav from "$lib/components/settings/SettingsNav.svelte";
  import GeneralPage from "$lib/components/settings/pages/GeneralPage.svelte";
  import ApiPage from "$lib/components/settings/pages/ApiPage.svelte";
  import SharingPage from "$lib/components/settings/pages/SharingPage.svelte";
  import UsersPage from "$lib/components/settings/pages/UsersPage.svelte";
  import KeyboardPage from "$lib/components/settings/pages/KeyboardPage.svelte";
  import DebuggingPage from "$lib/components/settings/pages/DebuggingPage.svelte";

  let { onResetComplete }: { onResetComplete: () => void } = $props();

  // Active sub-page lives in the navigation store so the command bar
  // can deep-link (⌘K → "Settings: Sharing" jumps straight there).
  const titles: Record<SettingsPageKey, string> = {
    general: "General",
    sharing: "Sharing",
    external: "External services",
    users: "Users",
    keyboard: "Keyboard shortcuts",
    debugging: "Debugging",
  };

  const subtitles: Record<SettingsPageKey, string | undefined> = {
    general: "Catalog source and reset.",
    sharing: "Resolution presets used by orders and ad-hoc shares.",
    external:
      "Credentials for OpenSFHistory's API and Backblaze B2 storage.",
    users: "Manage who can sign in and how long sessions stay active.",
    keyboard: "Every shortcut the app honors.",
    debugging: "In-app bug reports, filed straight to GitHub Issues.",
  };
</script>

<div class="flex flex-1 min-w-0 min-h-0">
  <SettingsNav
    active={$currentSettingsPage}
    onSelect={(k) => currentSettingsPage.set(k)}
  />

  <div class="flex-1 flex flex-col bg-background min-w-0 min-h-0">
    <PageHeader
      title={titles[$currentSettingsPage]}
      subtitle={subtitles[$currentSettingsPage]}
    />

    <div class="flex-1 min-h-0 overflow-y-auto px-8 py-5 select-text">
      {#if $currentSettingsPage === "general"}
        <GeneralPage {onResetComplete} />
      {:else if $currentSettingsPage === "sharing"}
        <SharingPage />
      {:else if $currentSettingsPage === "external"}
        <ApiPage />
      {:else if $currentSettingsPage === "users"}
        <UsersPage />
      {:else if $currentSettingsPage === "keyboard"}
        <KeyboardPage />
      {:else if $currentSettingsPage === "debugging"}
        <DebuggingPage />
      {/if}
    </div>
  </div>
</div>
