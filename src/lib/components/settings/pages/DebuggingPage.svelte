<script lang="ts">
  // Debugging settings (in-app bug reporting → GitHub Issues).
  //
  // The toggle applies immediately (and updates the shared store so the
  // sidebar icon appears/disappears live); the token and repo save via the
  // explicit button, following the ApiPage pattern. Settings is admin-only
  // as a whole, so no extra gating here.

  import { onMount } from "svelte";
  import { getSetting, setSetting } from "$lib/commands/settings";
  import { debugReportingEnabled } from "$lib/stores/debugReporting";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { KbdSeq } from "$lib/components/ui/kbd";
  import Toggle from "$lib/components/settings/Toggle.svelte";

  const DEFAULT_REPO = "danielucas/WNP-Image-Archive-desktop-app";

  let enabled = $state(false);
  let token = $state("");
  let repo = $state("");
  /** Save and toggle stay disabled until the stored values are in — a
   *  failed load must not let Save overwrite the token with blanks. */
  let loaded = $state(false);

  let saveStatus = $state<"idle" | "saving" | "saved">("idle");
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let saveError = $state<string | null>(null);
  let toggleError = $state<string | null>(null);

  onMount(async () => {
    try {
      const [enabledValue, tokenValue, repoValue] = await Promise.all([
        getSetting("debug_reporting_enabled"),
        getSetting("github_issues_token"),
        getSetting("github_issues_repo"),
      ]);
      enabled = enabledValue === "true";
      token = tokenValue ?? "";
      repo = repoValue ?? "";
      loaded = true;
    } catch (e) {
      saveError = `Failed to load settings: ${e instanceof Error ? e.message : String(e)}`;
    }
  });

  async function handleToggle() {
    const next = !enabled;
    enabled = next; // optimistic — reverted on failure
    toggleError = null;
    try {
      await setSetting("debug_reporting_enabled", next ? "true" : "false");
      debugReportingEnabled.set(next);
    } catch (e) {
      enabled = !next;
      toggleError = e instanceof Error ? e.message : String(e);
    }
  }

  async function save() {
    saveStatus = "saving";
    saveError = null;
    clearTimeout(saveTimer);

    const fields: Array<[string, string]> = [
      ["github_issues_token", token.trim()],
      ["github_issues_repo", repo.trim()],
    ];
    for (const [key, value] of fields) {
      try {
        await setSetting(key, value);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        saveError = `Failed to save '${key}': ${msg}`;
        console.error("DebuggingPage save failed", key, e);
        saveStatus = "idle";
        return;
      }
    }

    saveStatus = "saved";
    saveTimer = setTimeout(() => (saveStatus = "idle"), 2000);
  }
</script>

<div class="max-w-[640px] space-y-6">
  <section>
    <div class="flex items-center justify-between gap-4">
      <div>
        <h3 class="text-[14px] font-semibold text-foreground mb-1">
          Turn on debugging
        </h3>
        <p class="text-[12px] text-muted-foreground">
          Shows a bug icon next to the user chip in the sidebar and enables
          <KbdSeq keys={["⌘", "⇧", "B"]} /> for every signed-in user. Reports are
          filed as GitHub issues.
        </p>
      </div>
      <Toggle
        on={enabled}
        disabled={!loaded}
        onToggle={handleToggle}
        ariaLabel="Turn on debugging"
      />
    </div>
    {#if toggleError}
      <p class="mt-2 text-[12px] text-destructive">{toggleError}</p>
    {/if}
  </section>

  <section class="border-t border-border pt-5">
    <h3 class="text-[14px] font-semibold text-foreground mb-1">GitHub</h3>
    <p class="text-[12px] text-muted-foreground mb-3">
      Reports are created as issues in the repo below using this token.
    </p>
    <div class="space-y-3.5">
      <div class="space-y-1.5">
        <label
          for="github-token"
          class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
          >Fine-grained token</label
        >
        <Input
          id="github-token"
          type="password"
          bind:value={token}
          placeholder="github_pat_…"
          autocomplete="off"
        />
        <p class="text-[11px] text-muted-foreground">
          Create one at GitHub → Settings → Developer settings → Fine-grained
          tokens, scoped to the repo below with Issues read &amp; write. The
          token stays in this app's local database.
        </p>
      </div>
      <div class="space-y-1.5">
        <label
          for="github-repo"
          class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
          >Repository</label
        >
        <Input id="github-repo" bind:value={repo} placeholder={DEFAULT_REPO} />
        <p class="text-[11px] text-muted-foreground">
          owner/repo. Leave blank to use {DEFAULT_REPO}.
        </p>
      </div>
    </div>
  </section>

  <div class="space-y-2 pt-2">
    <div class="flex items-center gap-3">
      <Button disabled={saveStatus === "saving" || !loaded} onclick={save}>
        {saveStatus === "saving" ? "Saving…" : "Save"}
      </Button>
      {#if saveStatus === "saved"}
        <span class="text-sm text-success">Saved</span>
      {/if}
    </div>
    {#if saveError}
      <p class="text-[12px] text-destructive">{saveError}</p>
    {/if}
  </div>
</div>
