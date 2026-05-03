<script lang="ts">
  import type { AuditLogGlobalEntry } from "$lib/commands/activity";
  import { formatRelativeTime } from "$lib/utils/format";
  import { currentImageId, currentView } from "$lib/stores/navigation";
  import { Button } from "$lib/components/ui/button";
  import ArrowRight from "@lucide/svelte/icons/arrow-right";
  import ExternalLink from "@lucide/svelte/icons/external-link";

  interface Props {
    entry: AuditLogGlobalEntry;
  }

  let { entry }: Props = $props();

  function fieldLabel(field: string): string {
    const labels: Record<string, string> = {
      title: "Title",
      description: "Description",
      city: "City",
      state: "State",
      country: "Country",
      keywords: "Keywords",
      date_display: "Date (display)",
      date_start: "Date start",
      date_end: "Date end",
      photographer: "Photographer",
      donor: "Donor",
      acquisition_date: "Acquisition date",
      usage_rights: "Usage rights",
      internal_notes: "Internal notes",
    };
    return labels[field] ?? field;
  }

  function shortTime(iso: string): string {
    // changed_at is "YYYY-MM-DD HH:MM:SS" — pull just HH:MM
    const m = iso.match(/(\d{2}):(\d{2}):(\d{2})/);
    return m ? `${m[1]}:${m[2]}` : iso;
  }

  function viewImage() {
    currentImageId.set(entry.image_id);
    currentView.set("detail");
  }

  function displayValue(v: string | null): string {
    if (v === null || v === "") return "—";
    return v;
  }
</script>

<div
  class="flex gap-[14px] px-6 py-3.5 border-b border-border-muted items-start select-text"
>
  <div
    class="w-14 flex-shrink-0 text-xs text-muted-foreground tabular-nums font-mono pt-1"
  >
    {shortTime(entry.changed_at)}
  </div>

  <div class="flex-1 min-w-0">
    <div class="text-[13px] text-foreground leading-[1.5]">
      <span class="font-medium">{entry.changed_by}</span>
      <span class="text-muted-fg-2">edited</span>
      <span class="text-muted-foreground">{fieldLabel(entry.field_name)}</span>
      on
      <span
        class="font-mono text-[12.5px] bg-secondary text-foreground px-1.5 py-[1px] rounded"
      >
        {entry.catalog_number}
      </span>
    </div>

    {#if entry.old_value !== null || entry.new_value !== null}
      <div class="mt-1.5 text-xs font-mono flex gap-2 items-center flex-wrap">
        <span
          class="px-1.5 py-0.5 rounded text-destructive line-through"
          style="background: hsl(var(--destructive) / 0.08); text-decoration-color: hsl(var(--destructive) / 0.3);"
        >
          {displayValue(entry.old_value)}
        </span>
        <ArrowRight class="size-3 text-muted-foreground flex-shrink-0" />
        <span
          class="px-1.5 py-0.5 rounded"
          style="background: hsl(var(--success) / 0.08); color: hsl(var(--success));"
        >
          {displayValue(entry.new_value)}
        </span>
      </div>
    {/if}
  </div>

  <Button variant="ghost" size="xs" onclick={viewImage}>
    View
    <ExternalLink />
  </Button>
</div>
