<script lang="ts">
  // Ad-hoc share dialog (Plan 5).
  //
  // Resizes the image (via the create_share_link backend command),
  // uploads it to B2, then asks OpenSFHistory to email the recipient
  // a link via its existing Postmark integration. We never send mail
  // directly from the desktop.
  //
  // The dialog has two states:
  //   - Form: user inputs recipient + resolution + purpose, hits "Send"
  //   - Success: confirms the recipient + URL, dismisses on close
  // Errors render inline; the form stays open so the user can retry.

  import * as Dialog from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { Textarea } from "$lib/components/ui/textarea";
  import {
    createShareLink,
    type CreateShareLinkResult,
  } from "$lib/commands/sharing";
  import type { ImageRecord } from "$lib/commands/images";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { activityVersion } from "$lib/stores/activity";
  import { Send, Check, AlertTriangle } from "@lucide/svelte";

  interface Props {
    open: boolean;
    image: ImageRecord;
    onClose?: () => void;
  }

  let { open = $bindable(), image, onClose }: Props = $props();

  type Resolution = "low" | "high" | "full";

  let recipientEmail = $state("");
  let resolution = $state<Resolution>("high");
  let purpose = $state("");
  let submitting = $state(false);
  let error = $state<string | null>(null);
  let success = $state<CreateShareLinkResult | null>(null);

  // Reset on close so the next open is fresh.
  $effect(() => {
    if (!open) {
      recipientEmail = "";
      resolution = "high";
      purpose = "";
      submitting = false;
      error = null;
      success = null;
    }
  });

  // Cheap email format check — server-side validation is the real gate.
  const EMAIL_RE = /^[^@\s]+@[^@\s]+\.[^@\s]+$/;
  let emailValid = $derived(EMAIL_RE.test(recipientEmail.trim()));

  let canSubmit = $derived(
    !submitting &&
      emailValid &&
      purpose.trim().length > 0 &&
      success === null,
  );

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    if (!canSubmit) return;
    submitting = true;
    error = null;
    try {
      const result = await createShareLink({
        imageId: image.id,
        recipientEmail: recipientEmail.trim(),
        resolution,
        purpose: purpose.trim(),
      });
      success = result;
      // Bump activity so the sidebar's Recent activity card refreshes
      // (this share generates a new audit_log entry on the backend).
      activityVersion.update((n) => n + 1);
    } catch (e) {
      error = String(e);
    } finally {
      submitting = false;
    }
  }

  function handleClose() {
    open = false;
    onClose?.();
  }

  let thumbnailSrc = $derived.by(() => {
    if (!image.thumbnail_path) return null;
    return convertFileSrc(image.thumbnail_path);
  });
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="max-w-md">
    {#if success}
      <!-- ── Success state ────────────────────────────────────────── -->
      <Dialog.Header>
        <Dialog.Title>
          <span class="flex items-center gap-2">
            <Check class="size-4 text-success" />
            Sent
          </span>
        </Dialog.Title>
        <Dialog.Description>
          A link was emailed to <strong class="text-foreground"
            >{success.recipientEmail}</strong
          >.
        </Dialog.Description>
      </Dialog.Header>
      <div class="py-3 space-y-2 text-[12.5px]">
        <div>
          <span class="text-muted-foreground">Resolution:</span>
          {success.resolutionLabel}
        </div>
        <div>
          <span class="text-muted-foreground">Link:</span>
          <a
            href={success.imageUrl}
            target="_blank"
            rel="noopener"
            class="font-mono text-[11px] break-all text-info hover:underline"
          >
            {success.imageUrl}
          </a>
        </div>
        <p class="text-[11.5px] text-muted-foreground pt-1">
          Links expire after 30 days (handled by the B2 lifecycle policy).
        </p>
      </div>
      <Dialog.Footer>
        <Button onclick={handleClose}>Done</Button>
      </Dialog.Footer>
    {:else}
      <!-- ── Form state ───────────────────────────────────────────── -->
      <form onsubmit={handleSubmit}>
        <Dialog.Header>
          <Dialog.Title>Share image</Dialog.Title>
          <Dialog.Description>
            Resize and email a link via OpenSFHistory's Postmark.
          </Dialog.Description>
        </Dialog.Header>

        <!-- Image preview row -->
        <div
          class="flex items-center gap-3 my-4 p-2.5 rounded-md bg-secondary/40 border border-border"
        >
          <div
            class="w-12 h-12 flex-shrink-0 rounded bg-secondary overflow-hidden"
          >
            {#if thumbnailSrc}
              <img
                src={thumbnailSrc}
                alt={image.catalog_number}
                class="w-full h-full object-contain"
              />
            {/if}
          </div>
          <div class="flex-1 min-w-0">
            <div class="font-mono text-[11px] text-muted-foreground">
              {image.catalog_number}
            </div>
            <div class="text-[12.5px] text-foreground truncate">
              {image.title ?? "Untitled"}
            </div>
          </div>
        </div>

        <!-- Usage rights warning -->
        {#if image.usage_rights}
          <div
            class="mb-3 flex gap-2 p-2.5 rounded-md bg-warning/10 border border-warning/30 text-[11.5px] text-foreground"
          >
            <AlertTriangle class="size-3.5 mt-px flex-shrink-0 text-warning" />
            <div>
              <div class="font-medium mb-0.5">Usage rights</div>
              <div class="text-muted-foreground">{image.usage_rights}</div>
            </div>
          </div>
        {/if}

        <!-- Form fields -->
        <div class="space-y-3.5">
          <div class="space-y-1.5">
            <Label for="share-recipient">Recipient email</Label>
            <Input
              id="share-recipient"
              type="email"
              bind:value={recipientEmail}
              placeholder="alice@example.com"
              autocomplete="email"
              required
            />
          </div>

          <div class="space-y-1.5">
            <Label>Resolution</Label>
            <div class="flex gap-1.5">
              {#each [
                { value: "low" as Resolution, label: "Low", subtitle: "800px" },
                { value: "high" as Resolution, label: "High", subtitle: "2048px" },
                { value: "full" as Resolution, label: "Full", subtitle: "Original" },
              ] as opt (opt.value)}
                <button
                  type="button"
                  onclick={() => (resolution = opt.value)}
                  class="flex-1 flex flex-col items-center px-2 py-2 rounded-md border text-[11.5px] transition-colors {resolution === opt.value
                    ? 'border-primary bg-primary/5 text-foreground'
                    : 'border-border bg-background text-muted-foreground hover:bg-hover'}"
                >
                  <span class="font-medium">{opt.label}</span>
                  <span class="text-[10.5px] text-muted-foreground">
                    {opt.subtitle}
                  </span>
                </button>
              {/each}
            </div>
          </div>

          <div class="space-y-1.5">
            <Label for="share-purpose">Purpose</Label>
            <Textarea
              id="share-purpose"
              rows={3}
              bind:value={purpose}
              placeholder="Used in our newsletter, October issue"
              required
            />
          </div>
        </div>

        {#if error}
          <p class="mt-3 text-[12px] text-destructive">{error}</p>
        {/if}

        <Dialog.Footer class="mt-4">
          <Button type="button" variant="outline" onclick={handleClose}>
            Cancel
          </Button>
          <Button type="submit" disabled={!canSubmit}>
            <Send class="size-3.5" />
            {submitting ? "Sending…" : "Resize & send"}
          </Button>
        </Dialog.Footer>
      </form>
    {/if}
  </Dialog.Content>
</Dialog.Root>
