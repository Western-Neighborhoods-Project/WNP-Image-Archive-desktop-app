<script lang="ts">
  // Settings → Users page (Plan 10).
  //
  // Admin-only management UI. The component itself doesn't gate on role
  // — `+page.svelte` already redirects editors away from the settings
  // view — but the backend commands (list_users, create_user, etc.)
  // each independently require admin and will return errors if a non-
  // admin somehow reaches them.

  import { onMount } from "svelte";
  import { listUsers, createUser, updateUserRole, deleteUser, type User } from "$lib/commands/users";
  import type { UserRole } from "$lib/commands/auth";
  import { currentUser } from "$lib/stores/currentUser";
  import {
    inactivityTimeoutMinutes,
    saveInactivityTimeout,
  } from "$lib/stores/inactivityTimeout";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { Button } from "$lib/components/ui/button";
  import * as Dialog from "$lib/components/ui/dialog";
  import * as Select from "$lib/components/ui/select";
  import * as AlertDialog from "$lib/components/ui/alert-dialog";
  import ChangePasswordDialog from "$lib/components/auth/ChangePasswordDialog.svelte";
  import { KeyRound, Trash2, UserPlus } from "@lucide/svelte";
  import { formatRelativeTime } from "$lib/utils/format";

  // ── State ─────────────────────────────────────────────────────────────────
  let users = $state<User[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Inactivity timeout local form state (saved on blur).
  let timeoutInput = $state(String($inactivityTimeoutMinutes));
  let timeoutSaving = $state(false);

  // Add-user dialog
  let showAddUser = $state(false);
  let newUsername = $state("");
  let newPassword = $state("");
  let newRole = $state<UserRole>("editor");
  let addError = $state<string | null>(null);
  let addSubmitting = $state(false);

  // Change-password dialog
  let pwDialogOpen = $state(false);
  let pwDialogTarget = $state<{ id: number; username: string } | null>(null);

  // Delete-confirm dialog
  let deleteTarget = $state<User | null>(null);
  let deleting = $state(false);
  let deleteError = $state<string | null>(null);

  $effect(() => {
    timeoutInput = String($inactivityTimeoutMinutes);
  });

  onMount(async () => {
    await refresh();
  });

  async function refresh() {
    loading = true;
    error = null;
    try {
      users = await listUsers();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function handleSaveTimeout() {
    const n = parseInt(timeoutInput, 10);
    if (Number.isNaN(n) || n <= 0) {
      timeoutInput = String($inactivityTimeoutMinutes);
      return;
    }
    if (n === $inactivityTimeoutMinutes) return;
    timeoutSaving = true;
    try {
      await saveInactivityTimeout(n);
    } catch (e) {
      console.error(e);
      timeoutInput = String($inactivityTimeoutMinutes);
    } finally {
      timeoutSaving = false;
    }
  }

  async function handleAddUser(e: SubmitEvent) {
    e.preventDefault();
    addError = null;
    addSubmitting = true;
    try {
      await createUser(newUsername, newPassword, newRole);
      newUsername = "";
      newPassword = "";
      newRole = "editor";
      showAddUser = false;
      await refresh();
    } catch (e) {
      addError = String(e);
    } finally {
      addSubmitting = false;
    }
  }

  async function handleRoleChange(user: User, role: UserRole) {
    if (user.role === role) return;
    try {
      await updateUserRole(user.id, role);
      await refresh();
    } catch (e) {
      // Backend rejection (e.g. last admin) — refresh to reset UI + show error
      error = String(e);
      await refresh();
    }
  }

  function openChangePassword(user: User) {
    pwDialogTarget = { id: user.id, username: user.username };
    pwDialogOpen = true;
  }

  function openDeleteConfirm(user: User) {
    deleteTarget = user;
    deleteError = null;
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    deleting = true;
    deleteError = null;
    try {
      await deleteUser(deleteTarget.id);
      deleteTarget = null;
      await refresh();
    } catch (e) {
      deleteError = String(e);
    } finally {
      deleting = false;
    }
  }
</script>

<div class="max-w-[760px]">
  <!-- Inactivity timeout -->
  <section class="mb-7">
    <h3 class="text-[14px] font-semibold text-foreground mb-1">
      Auto-logout after inactivity
    </h3>
    <p class="text-[12px] text-muted-foreground mb-3">
      The app logs out automatically when there's no mouse or keyboard
      activity for this many minutes.
    </p>
    <div class="flex items-center gap-2">
      <Input
        type="number"
        min="1"
        size="sm"
        class="w-24"
        bind:value={timeoutInput}
        onblur={handleSaveTimeout}
        disabled={timeoutSaving}
      />
      <span class="text-[12px] text-muted-foreground">minutes</span>
    </div>
  </section>

  <!-- User list -->
  <section>
    <div class="flex items-center justify-between mb-3">
      <h3 class="text-[14px] font-semibold text-foreground">Users</h3>
      <Button onclick={() => (showAddUser = true)}>
        <UserPlus class="size-3.5" />
        Add user
      </Button>
    </div>

    {#if error}
      <p class="text-[12px] text-destructive mb-3">{error}</p>
    {/if}

    {#if loading}
      <p class="text-[12px] text-muted-foreground">Loading…</p>
    {:else if users.length === 0}
      <p class="text-[12px] text-muted-foreground italic">No users yet.</p>
    {:else}
      <div class="rounded-md border border-border overflow-hidden">
        <table class="w-full text-[12.5px]">
          <thead class="bg-secondary/40">
            <tr>
              <th class="text-left px-3 py-2 font-medium">Username</th>
              <th class="text-left px-3 py-2 font-medium w-32">Role</th>
              <th class="text-left px-3 py-2 font-medium">Last login</th>
              <th class="text-right px-3 py-2 font-medium w-24">Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each users as user (user.id)}
              {@const isSelf = $currentUser?.userId === user.id}
              <tr class="border-t border-border">
                <td class="px-3 py-2.5 align-middle">
                  <span class="font-medium text-foreground">{user.username}</span>
                  {#if isSelf}
                    <span class="ml-2 text-[10.5px] text-muted-foreground">(you)</span>
                  {/if}
                </td>
                <td class="px-3 py-2 align-middle">
                  <Select.Root
                    type="single"
                    value={user.role}
                    onValueChange={(v) => v && handleRoleChange(user, v as UserRole)}
                    disabled={isSelf}
                    size="xs"
                  >
                    <Select.Trigger class="w-full">
                      <span class="capitalize">{user.role}</span>
                    </Select.Trigger>
                    <Select.Content>
                      <Select.Item value="admin">Admin</Select.Item>
                      <Select.Item value="editor">Editor</Select.Item>
                    </Select.Content>
                  </Select.Root>
                </td>
                <td class="px-3 py-2.5 align-middle text-muted-foreground">
                  {user.lastLoginAt
                    ? formatRelativeTime(user.lastLoginAt)
                    : "Never"}
                </td>
                <td class="px-3 py-2 align-middle">
                  <div class="flex justify-end gap-1">
                    <Button
                      variant="ghost"
                      size="icon-xs"
                      onclick={() => openChangePassword(user)}
                      title="Change password"
                    >
                      <KeyRound class="size-3.5" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon-xs"
                      onclick={() => openDeleteConfirm(user)}
                      disabled={isSelf}
                      title={isSelf ? "You can't delete your own account" : "Delete user"}
                      class="hover:text-destructive"
                    >
                      <Trash2 class="size-3.5" />
                    </Button>
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>
</div>

<!-- Add-user dialog -->
<Dialog.Root bind:open={showAddUser}>
  <Dialog.Content class="max-w-md">
    <form onsubmit={handleAddUser}>
      <Dialog.Header>
        <Dialog.Title>Add user</Dialog.Title>
        <Dialog.Description>
          Create a username + password for someone else. They'll be able to
          sign in immediately and you can manage them from this page.
        </Dialog.Description>
      </Dialog.Header>
      <div class="space-y-3.5 py-4">
        <div class="space-y-1.5">
          <Label for="add-username">Username</Label>
          <Input id="add-username" bind:value={newUsername} required />
        </div>
        <div class="space-y-1.5">
          <Label for="add-password">Password</Label>
          <Input
            id="add-password"
            type="password"
            bind:value={newPassword}
            autocomplete="new-password"
            required
          />
          <p class="text-[11px] text-muted-foreground">
            At least 12 characters. All-lowercase strings need both letters and digits.
          </p>
        </div>
        <div class="space-y-1.5">
          <Label for="add-role">Role</Label>
          <Select.Root
            type="single"
            value={newRole}
            onValueChange={(v) => v && (newRole = v as UserRole)}
            size="sm"
          >
            <Select.Trigger>
              <span class="capitalize">{newRole}</span>
            </Select.Trigger>
            <Select.Content>
              <Select.Item value="admin">Admin — full access</Select.Item>
              <Select.Item value="editor">
                Editor — no Settings access
              </Select.Item>
            </Select.Content>
          </Select.Root>
        </div>
      </div>
      {#if addError}
        <p class="text-[12.5px] text-destructive">{addError}</p>
      {/if}
      <Dialog.Footer class="mt-4">
        <Button
          type="button"
          variant="outline"
          onclick={() => (showAddUser = false)}
        >
          Cancel
        </Button>
        <Button type="submit" disabled={addSubmitting}>
          {addSubmitting ? "Creating…" : "Create user"}
        </Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>

<!-- Change-password dialog (shared) -->
{#if pwDialogTarget}
  <ChangePasswordDialog
    bind:open={pwDialogOpen}
    userId={pwDialogTarget.id}
    username={pwDialogTarget.username}
    onBehalfOf={$currentUser?.userId !== pwDialogTarget.id}
    onClose={() => {
      pwDialogOpen = false;
      pwDialogTarget = null;
    }}
  />
{/if}

<!-- Delete-confirm dialog -->
<AlertDialog.Root open={deleteTarget !== null}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>Delete user "{deleteTarget?.username}"?</AlertDialog.Title>
      <AlertDialog.Description>
        Their audit-log entries stay attributed to their username — only
        the user account itself is removed. They won't be able to sign in
        anymore.
      </AlertDialog.Description>
    </AlertDialog.Header>
    {#if deleteError}
      <p class="text-[12.5px] text-destructive">{deleteError}</p>
    {/if}
    <AlertDialog.Footer>
      <AlertDialog.Cancel onclick={() => (deleteTarget = null)}>
        Cancel
      </AlertDialog.Cancel>
      <AlertDialog.Action
        onclick={confirmDelete}
        disabled={deleting}
        class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
      >
        {deleting ? "Deleting…" : "Delete user"}
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
