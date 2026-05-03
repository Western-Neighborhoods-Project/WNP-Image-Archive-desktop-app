<script lang="ts">
  // Login + bootstrap-admin screen (Plan 10).
  //
  // One component, two modes:
  //   - "bootstrap" — shown when no users exist (setupRequired = true).
  //     Shows "Create your first admin" form.
  //   - "login" — shown when at least one user exists. Username + password.
  //
  // Renders as a full-window blocking screen above WindowChrome's body
  // area. WindowChrome stays visible at top so the window remains
  // draggable.

  import { setupRequired } from "$lib/stores/currentUser";
  import { createFirstAdmin, login } from "$lib/commands/auth";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { Button } from "$lib/components/ui/button";
  import { Lock, UserPlus } from "@lucide/svelte";

  let username = $state("");
  let password = $state("");
  let confirmPassword = $state("");
  let submitting = $state(false);
  let error = $state<string | null>(null);

  let mode = $derived($setupRequired ? "bootstrap" : "login");

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    if (submitting) return;
    error = null;

    if (mode === "bootstrap") {
      if (password.length < 6) {
        error = "Password must be at least 6 characters";
        return;
      }
      if (password !== confirmPassword) {
        error = "Passwords don't match";
        return;
      }
    }

    submitting = true;
    try {
      if (mode === "bootstrap") {
        await createFirstAdmin(username, password);
      } else {
        await login(username, password);
      }
      // The auth:changed event will update the store; +page.svelte
      // unmounts this overlay automatically. We don't need to do
      // anything here.
      username = "";
      password = "";
      confirmPassword = "";
    } catch (e) {
      error = String(e);
    } finally {
      submitting = false;
    }
  }
</script>

<div
  class="absolute inset-0 z-50 flex items-center justify-center bg-background overflow-auto"
>
  <form
    onsubmit={handleSubmit}
    class="w-full max-w-[360px] px-8 py-10"
  >
    <!-- Icon -->
    <div
      class="mx-auto w-12 h-12 rounded-full bg-secondary text-foreground flex items-center justify-center mb-5"
    >
      {#if mode === "bootstrap"}
        <UserPlus class="size-6" />
      {:else}
        <Lock class="size-6" />
      {/if}
    </div>

    <!-- Heading -->
    <h2 class="text-[18px] font-semibold text-foreground text-center mb-1">
      {#if mode === "bootstrap"}
        Create your first admin
      {:else}
        Sign in
      {/if}
    </h2>
    <p class="text-[12.5px] text-muted-foreground text-center mb-7">
      {#if mode === "bootstrap"}
        This account will manage all other users.
      {:else}
        Enter your credentials to continue.
      {/if}
    </p>

    <!-- Fields -->
    <div class="space-y-3.5">
      <div class="space-y-1.5">
        <Label for="username">Username</Label>
        <Input
          id="username"
          type="text"
          bind:value={username}
          autocomplete="username"
          required
          autofocus
        />
      </div>
      <div class="space-y-1.5">
        <Label for="password">Password</Label>
        <Input
          id="password"
          type="password"
          bind:value={password}
          autocomplete={mode === "bootstrap" ? "new-password" : "current-password"}
          required
        />
      </div>
      {#if mode === "bootstrap"}
        <div class="space-y-1.5">
          <Label for="confirm-password">Confirm password</Label>
          <Input
            id="confirm-password"
            type="password"
            bind:value={confirmPassword}
            autocomplete="new-password"
            required
          />
        </div>
      {/if}
    </div>

    {#if error}
      <p class="mt-3 text-[12.5px] text-destructive text-center">{error}</p>
    {/if}

    <Button type="submit" class="w-full mt-5" disabled={submitting}>
      {#if submitting}
        {mode === "bootstrap" ? "Creating…" : "Signing in…"}
      {:else}
        {mode === "bootstrap" ? "Create admin & sign in" : "Sign in"}
      {/if}
    </Button>
  </form>
</div>
