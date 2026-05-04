<script lang="ts">
  // Full-screen modal shown while an update is downloading or
  // installing. Blocks the rest of the UI so the user understands the
  // app is busy and won't try to interact with stale state.
  import { updateStatus, updateBytes } from "$lib/updater";
  import { formatFileSize } from "$lib/utils/format";
  import { Loader2 } from "@lucide/svelte";

  let visible = $derived(
    $updateStatus.kind === "downloading" || $updateStatus.kind === "installing",
  );

  let percent = $derived(
    $updateBytes.total && $updateBytes.total > 0
      ? Math.min(100, Math.round(($updateBytes.downloaded / $updateBytes.total) * 100))
      : null,
  );
</script>

{#if visible}
  <div
    class="fixed inset-0 z-[100] flex items-center justify-center bg-black/60"
    role="dialog"
    aria-live="polite"
    aria-label="Installing update"
  >
    <div
      class="w-full max-w-md rounded-lg border border-border bg-background p-6"
      style="box-shadow: 0 24px 64px rgba(0,0,0,0.4);"
    >
      <div class="flex items-center gap-3 mb-4">
        <Loader2 class="size-4 animate-spin text-primary" />
        <h3 class="text-base font-semibold text-foreground">
          {$updateStatus.kind === "downloading" ? "Downloading update" : "Installing…"}
        </h3>
      </div>

      {#if $updateStatus.kind === "downloading"}
        <div class="space-y-2">
          <div class="h-2 w-full rounded-full bg-muted overflow-hidden">
            <div
              class="h-full bg-primary transition-[width] duration-200 ease-out"
              style="width: {percent ?? 0}%"
            ></div>
          </div>
          <p class="text-[12px] text-muted-foreground tabular-nums">
            {#if $updateBytes.total}
              {formatFileSize($updateBytes.downloaded)} / {formatFileSize($updateBytes.total)}
              {#if percent !== null}· {percent}%{/if}
            {:else}
              {formatFileSize($updateBytes.downloaded)} downloaded
            {/if}
          </p>
        </div>
      {:else}
        <p class="text-[13px] text-muted-foreground">
          The app will relaunch when installation completes.
        </p>
      {/if}
    </div>
  </div>
{/if}
