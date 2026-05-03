<script lang="ts">
  // Sidebar footer user chip + dropdown (Plan 10).
  //
  // Renders the currently signed-in user as a clickable strip at the
  // very bottom of the sidebar. Click opens a dropdown with Change
  // password + Log out. ⌘⇧L globally also triggers logout (wired in
  // +page.svelte, not here).

  import { currentUser } from "$lib/stores/currentUser";
  import { logout } from "$lib/commands/auth";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
  import { DropdownMenuPrimitive } from "$lib/components/ui/dropdown-menu";
  import ChangePasswordDialog from "./ChangePasswordDialog.svelte";
  import { Kbd, KbdSeq } from "$lib/components/ui/kbd";
  import { LogOut, KeyRound } from "@lucide/svelte";

  let showChangePassword = $state(false);

  function initials(username: string): string {
    const parts = username.trim().split(/[\s._-]+/).filter(Boolean);
    if (parts.length === 0) return "?";
    if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
    return (parts[0][0] + parts[1][0]).toUpperCase();
  }

  async function handleLogout() {
    try {
      await logout();
    } catch (e) {
      console.error("Logout failed", e);
    }
  }
</script>

{#if $currentUser}
  <div class="border-t border-border px-2 py-2">
    <DropdownMenu.Root>
      <DropdownMenuPrimitive.Trigger>
        {#snippet child({ props })}
          <button
            {...props}
            class="w-full flex items-center gap-2.5 px-2 py-1.5 rounded-md hover:bg-hover transition-colors text-left"
          >
            <span
              class="w-7 h-7 rounded-full bg-primary text-primary-foreground flex items-center justify-center text-[11px] font-semibold flex-shrink-0"
            >
              {initials($currentUser.username)}
            </span>
            <span class="flex-1 min-w-0">
              <span
                class="block text-[12.5px] font-medium text-foreground truncate"
              >
                {$currentUser.username}
              </span>
              <span class="block text-[10.5px] text-muted-foreground capitalize">
                {$currentUser.role}
              </span>
            </span>
          </button>
        {/snippet}
      </DropdownMenuPrimitive.Trigger>
      <DropdownMenu.Content align="end" side="top">
        <DropdownMenu.Item onclick={() => (showChangePassword = true)}>
          <KeyRound class="size-3.5" />
          Change password
        </DropdownMenu.Item>
        <DropdownMenu.Separator />
        <DropdownMenu.Item onclick={handleLogout}>
          <LogOut class="size-3.5" />
          <span class="flex-1">Log out</span>
          <KbdSeq keys={["⌘", "⇧", "L"]} />
        </DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Root>

    <ChangePasswordDialog
      bind:open={showChangePassword}
      userId={$currentUser.userId}
      username={$currentUser.username}
      onClose={() => (showChangePassword = false)}
    />
  </div>
{/if}
