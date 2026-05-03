<script lang="ts">
  // Change-password dialog. Used for both the user changing their own
  // password (UserMenu) and admins changing another user's (UsersPage).
  // The backend gate (`update_user_password` checks current session role)
  // already enforces the security boundary; this UI just collects input.

  import * as Dialog from "$lib/components/ui/dialog";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { Button } from "$lib/components/ui/button";
  import { updateUserPassword } from "$lib/commands/users";

  interface Props {
    open: boolean;
    userId: number;
    username: string;
    /** True when an admin is changing someone else's password — affects copy. */
    onBehalfOf?: boolean;
    onClose: () => void;
  }

  let {
    open = $bindable(),
    userId,
    username,
    onBehalfOf = false,
    onClose,
  }: Props = $props();

  let newPassword = $state("");
  let confirmPassword = $state("");
  let submitting = $state(false);
  let error = $state<string | null>(null);

  function reset() {
    newPassword = "";
    confirmPassword = "";
    submitting = false;
    error = null;
  }

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    error = null;
    if (newPassword.length < 6) {
      error = "Password must be at least 6 characters";
      return;
    }
    if (newPassword !== confirmPassword) {
      error = "Passwords don't match";
      return;
    }
    submitting = true;
    try {
      await updateUserPassword(userId, newPassword);
      reset();
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      submitting = false;
    }
  }

  function handleCancel() {
    reset();
    onClose();
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="max-w-md">
    <form onsubmit={handleSubmit}>
      <Dialog.Header>
        <Dialog.Title>
          {onBehalfOf ? `Change password for ${username}` : "Change your password"}
        </Dialog.Title>
        <Dialog.Description>
          {onBehalfOf
            ? "The user will need this new password on their next sign in."
            : "You'll continue to be signed in. Use the new password next time."}
        </Dialog.Description>
      </Dialog.Header>

      <div class="space-y-3.5 py-4">
        <div class="space-y-1.5">
          <Label for="new-password">New password</Label>
          <Input
            id="new-password"
            type="password"
            bind:value={newPassword}
            autocomplete="new-password"
            required
          />
        </div>
        <div class="space-y-1.5">
          <Label for="confirm-new-password">Confirm new password</Label>
          <Input
            id="confirm-new-password"
            type="password"
            bind:value={confirmPassword}
            autocomplete="new-password"
            required
          />
        </div>
      </div>

      {#if error}
        <p class="text-[12.5px] text-destructive">{error}</p>
      {/if}

      <Dialog.Footer class="mt-4">
        <Button type="button" variant="outline" onclick={handleCancel}>
          Cancel
        </Button>
        <Button type="submit" disabled={submitting}>
          {submitting ? "Saving…" : "Save password"}
        </Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>
