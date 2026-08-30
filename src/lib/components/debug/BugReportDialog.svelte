<script lang="ts">
  // In-app report dialog (Settings → Debugging must be on). Opened by the
  // sidebar bug icon or ⌘⇧B; both just set the `bugReportOpen` store and
  // this component (mounted once in +page.svelte) does the rest. Submits
  // via `submit_bug_report`, which appends version/OS/view/user context
  // and files a GitHub issue.

  import { untrack } from "svelte";
  import * as Dialog from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";
  import { Textarea } from "$lib/components/ui/textarea";
  import { bugReportOpen } from "$lib/stores/debugReporting";
  import { currentView } from "$lib/stores/navigation";
  import {
    submitBugReport,
    type ReportCategory,
  } from "$lib/commands/bugReports";

  const CATEGORIES: Array<{ value: ReportCategory; label: string }> = [
    { value: "bug", label: "Bug" },
    { value: "feature", label: "Feature request" },
    { value: "idea", label: "Idea" },
    { value: "data", label: "Data issue" },
    { value: "ux", label: "UX / Polish" },
  ];

  let category = $state<ReportCategory>("bug");
  let description = $state("");
  let submitting = $state(false);
  let error = $state<string | null>(null);
  let filedIssueNumber = $state<number | null>(null);
  let closeTimer: ReturnType<typeof setTimeout> | undefined;

  // Runs when the dialog opens: cancel any auto-close still pending from
  // a recent success (reopening within 1.5s must not close the new dialog),
  // and start fresh if the last submit filed an issue. A failed or
  // abandoned submit keeps its text across close/reopen. `filedIssueNumber`
  // is read untracked — otherwise the successful submit itself would
  // re-trigger this effect and wipe the success message while still open.
  $effect(() => {
    if ($bugReportOpen) {
      clearTimeout(closeTimer);
      if (untrack(() => filedIssueNumber) !== null) {
        category = "bug";
        description = "";
        error = null;
        filedIssueNumber = null;
      }
    }
  });

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    if (!description.trim() || submitting) return;
    submitting = true;
    error = null;
    try {
      const issue = await submitBugReport(category, description, $currentView);
      filedIssueNumber = issue.number;
      clearTimeout(closeTimer);
      closeTimer = setTimeout(() => bugReportOpen.set(false), 1500);
    } catch (e) {
      error = String(e);
    } finally {
      submitting = false;
    }
  }
</script>

<Dialog.Root bind:open={$bugReportOpen}>
  <Dialog.Content class="max-w-md">
    <form onsubmit={handleSubmit}>
      <Dialog.Header>
        <Dialog.Title>Report a problem</Dialog.Title>
        <Dialog.Description>
          Goes straight to the app's GitHub issues, with version and view info
          attached.
        </Dialog.Description>
      </Dialog.Header>

      <div class="space-y-3.5 py-4">
        <div class="flex flex-wrap gap-1.5">
          {#each CATEGORIES as c (c.value)}
            <button
              type="button"
              onclick={() => (category = c.value)}
              class="px-2.5 h-[24px] rounded-full text-[12px] transition-colors border
                {category === c.value
                ? 'bg-primary text-primary-foreground border-primary'
                : 'bg-transparent text-muted-fg-2 border-border hover:bg-hover'}"
              aria-pressed={category === c.value}
            >
              {c.label}
            </button>
          {/each}
        </div>

        <Textarea
          bind:value={description}
          rows={5}
          placeholder="What went wrong? A sentence or two is plenty — the first line becomes the issue title."
          disabled={submitting || filedIssueNumber !== null}
        />
      </div>

      {#if error}
        <p class="text-[12.5px] text-destructive">{error}</p>
      {/if}
      {#if filedIssueNumber !== null}
        <p class="text-[12.5px] text-success">
          Filed issue #{filedIssueNumber}. Thanks!
        </p>
      {/if}

      <Dialog.Footer class="mt-4">
        <Button
          type="button"
          variant="outline"
          onclick={() => bugReportOpen.set(false)}
        >
          Cancel
        </Button>
        <Button
          type="submit"
          disabled={submitting || !description.trim() || filedIssueNumber !== null}
        >
          {submitting ? "Submitting…" : "Submit"}
        </Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>
