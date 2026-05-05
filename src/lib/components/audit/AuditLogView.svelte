<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import {
    getAuditLogGlobal,
    exportAuditLogCsv,
    type AuditLogGlobalEntry,
  } from "$lib/commands/activity";
  import { activityVersion } from "$lib/stores/activity";
  import { PageHeader } from "$lib/components/ui/page-header";
  import { StatusBar } from "$lib/components/ui/status-bar";
  import DriveIndicator from "$lib/components/drive/DriveIndicator.svelte";
  import BackgroundActivityIndicator from "$lib/components/footer/BackgroundActivityIndicator.svelte";
  import { Button } from "$lib/components/ui/button";
  import { Kbd } from "$lib/components/ui/kbd";
  import { openShortcutsHelp } from "$lib/stores/shortcutsHelp";
  import * as Select from "$lib/components/ui/select";
  import AuditEntryRow from "./AuditEntryRow.svelte";
  import Download from "@lucide/svelte/icons/download";
  import Check from "@lucide/svelte/icons/check";

  // ── Filter options ─────────────────────────────────────────────
  const FIELD_OPTIONS: { value: string; label: string }[] = [
    { value: "all", label: "All fields" },
    { value: "title", label: "Title" },
    { value: "description", label: "Description" },
    { value: "city", label: "City" },
    { value: "state", label: "State" },
    { value: "country", label: "Country" },
    { value: "keywords", label: "Keywords" },
    { value: "date_display", label: "Date (display)" },
    { value: "date_start", label: "Date start" },
    { value: "date_end", label: "Date end" },
    { value: "photographer", label: "Photographer" },
    { value: "donor", label: "Donor" },
    { value: "acquisition_date", label: "Acquisition date" },
    { value: "usage_rights", label: "Usage rights" },
    { value: "internal_notes", label: "Internal notes" },
  ];

  const RANGE_OPTIONS: { value: string; label: string }[] = [
    { value: "all", label: "All time" },
    { value: "7d", label: "Last 7 days" },
    { value: "30d", label: "Last 30 days" },
    { value: "90d", label: "Last 90 days" },
  ];

  // ── State ──────────────────────────────────────────────────────
  let selectedField = $state<string>("all");
  let selectedRange = $state<string>("30d");

  let entries = $state<AuditLogGlobalEntry[]>([]);
  let loading = $state(false);
  let hasMore = $state(true);
  let offset = $state(0);
  const PAGE_SIZE = 100;

  let exportStatus = $state<"idle" | "exporting" | "done">("idle");
  let scrollEl = $state<HTMLElement | undefined>();
  let sentinelEl = $state<HTMLElement | undefined>();

  // ── Filter helpers ─────────────────────────────────────────────
  function rangeSince(range: string): string | null {
    if (range === "all") return null;
    const days = range === "7d" ? 7 : range === "90d" ? 90 : 30;
    const d = new Date();
    d.setDate(d.getDate() - days);
    // Match SQLite's 'YYYY-MM-DD HH:MM:SS' format (UTC); lexicographic
    // sort matches chronological for this format.
    return d.toISOString().slice(0, 19).replace("T", " ");
  }

  function fieldFilterValue(): string | null {
    return selectedField === "all" ? null : selectedField;
  }

  let fieldLabel = $derived(
    FIELD_OPTIONS.find((o) => o.value === selectedField)?.label ?? "All fields",
  );
  let rangeLabel = $derived(
    RANGE_OPTIONS.find((o) => o.value === selectedRange)?.label ?? "All time",
  );

  // ── Fetching ───────────────────────────────────────────────────
  async function loadPage(reset: boolean) {
    if (loading) return;
    loading = true;
    try {
      const nextOffset = reset ? 0 : offset;
      const page = await getAuditLogGlobal({
        fieldName: fieldFilterValue(),
        since: rangeSince(selectedRange),
        until: null,
        limit: PAGE_SIZE,
        offset: nextOffset,
      });
      if (reset) {
        entries = page;
      } else {
        entries = [...entries, ...page];
      }
      offset = nextOffset + page.length;
      hasMore = page.length === PAGE_SIZE;
    } catch (e) {
      console.error("Failed to load audit log", e);
      hasMore = false;
    } finally {
      loading = false;
    }
  }

  // Refetch from scratch when filters change OR when an edit is made
  // elsewhere in the app (activityVersion bumps via DetailView save).
  $effect(() => {
    selectedField;
    selectedRange;
    $activityVersion;
    untrack(() => {
      offset = 0;
      hasMore = true;
      void loadPage(true);
    });
  });

  // Infinite scroll via IntersectionObserver on a sentinel below the list.
  let observer: IntersectionObserver | undefined;
  $effect(() => {
    if (!sentinelEl || !scrollEl) return;
    observer?.disconnect();
    observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting && hasMore && !loading) {
          void loadPage(false);
        }
      },
      { root: scrollEl, rootMargin: "200px" },
    );
    observer.observe(sentinelEl);
    return () => observer?.disconnect();
  });

  // ── Date grouping for sticky headers ───────────────────────────
  function dateGroupLabel(iso: string): string {
    // iso is "YYYY-MM-DD HH:MM:SS" (SQLite default); pull the date part.
    const datePart = iso.slice(0, 10);
    const now = new Date();
    const today = now.toISOString().slice(0, 10);
    const yesterday = new Date(now);
    yesterday.setDate(yesterday.getDate() - 1);
    const yIso = yesterday.toISOString().slice(0, 10);
    if (datePart === today) return "Today";
    if (datePart === yIso) return "Yesterday";
    const d = new Date(datePart + "T00:00:00");
    const opts: Intl.DateTimeFormatOptions = {
      month: "short",
      day: "numeric",
    };
    if (d.getFullYear() !== now.getFullYear()) opts.year = "numeric";
    return d.toLocaleDateString(undefined, opts);
  }

  let groups = $derived.by(() => {
    const out: { label: string; items: AuditLogGlobalEntry[] }[] = [];
    let currentLabel = "";
    for (const entry of entries) {
      const label = dateGroupLabel(entry.changed_at);
      if (label !== currentLabel) {
        out.push({ label, items: [] });
        currentLabel = label;
      }
      out[out.length - 1].items.push(entry);
    }
    return out;
  });

  // ── CSV export ─────────────────────────────────────────────────
  async function handleExport() {
    if (exportStatus === "exporting") return;
    exportStatus = "exporting";
    try {
      const stamp = new Date().toISOString().slice(0, 10);
      const path = await save({
        defaultPath: `audit-log-${stamp}.csv`,
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!path) {
        exportStatus = "idle";
        return;
      }
      await exportAuditLogCsv({
        fieldName: fieldFilterValue(),
        since: rangeSince(selectedRange),
        until: null,
        path,
      });
      exportStatus = "done";
      setTimeout(() => {
        exportStatus = "idle";
      }, 2500);
    } catch (e) {
      console.error("CSV export failed", e);
      exportStatus = "idle";
    }
  }

  onMount(() => {
    // Initial load is handled by the filter $effect on first run
  });
</script>

<div class="flex flex-1 flex-col min-w-0 min-h-0">
  <PageHeader
    title="Audit log"
    subtitle="All metadata changes, approvals, imports"
    count={entries.length}
  >
    {#snippet right()}
      <div class="flex items-center gap-2">
        <Select.Root
          type="single"
          size="xs"
          value={selectedField}
          onValueChange={(v) => v && (selectedField = v)}
        >
          <Select.Trigger>{fieldLabel}</Select.Trigger>
          <Select.Content>
            {#each FIELD_OPTIONS as opt (opt.value)}
              <Select.Item value={opt.value}>{opt.label}</Select.Item>
            {/each}
          </Select.Content>
        </Select.Root>

        <Select.Root
          type="single"
          size="xs"
          value={selectedRange}
          onValueChange={(v) => v && (selectedRange = v)}
        >
          <Select.Trigger>{rangeLabel}</Select.Trigger>
          <Select.Content>
            {#each RANGE_OPTIONS as opt (opt.value)}
              <Select.Item value={opt.value}>{opt.label}</Select.Item>
            {/each}
          </Select.Content>
        </Select.Root>

        <Button
          size="xs"
          variant="outline"
          onclick={handleExport}
          disabled={exportStatus === "exporting"}
        >
          {#if exportStatus === "done"}
            <Check />
            Saved
          {:else}
            <Download />
            {exportStatus === "exporting" ? "Saving…" : "Export CSV"}
          {/if}
        </Button>
      </div>
    {/snippet}
  </PageHeader>

  <main class="flex-1 min-h-0 overflow-hidden">
    <div
      bind:this={scrollEl}
      class="h-full w-full overflow-y-auto bg-background"
      style="scrollbar-gutter: stable;"
    >
      <!-- Custom scrollbar visuals come from app.css (global ::-webkit-
           scrollbar). scrollbar-gutter: stable here reserves the lane so
           the sticky date headers below can't paint into it. -->

      {#if entries.length === 0 && !loading}
        <div class="flex h-full items-center justify-center text-muted-foreground">
          <p class="text-sm">
            No entries match the current filters.
          </p>
        </div>
      {:else}
        {#each groups as group (group.label)}
          <div
            class="sticky top-0 px-6 py-2 text-[11px] font-semibold uppercase tracking-[0.5px] text-muted-foreground bg-sidebar-bg border-y border-border"
          >
            {group.label}
          </div>
          {#each group.items as entry (entry.id)}
            <AuditEntryRow {entry} />
          {/each}
        {/each}

        <!-- Sentinel for IntersectionObserver-driven infinite scroll -->
        <div bind:this={sentinelEl} class="h-4"></div>

        {#if loading}
          <div class="px-6 py-4 text-xs text-muted-foreground text-center">
            Loading…
          </div>
        {:else if !hasMore && entries.length > 0}
          <div class="px-6 py-6 text-xs text-muted-foreground text-center">
            End of log · {entries.length}
            {entries.length === 1 ? "entry" : "entries"}
          </div>
        {/if}
      {/if}
    </div>
  </main>

  <StatusBar>
    <span>{entries.length.toLocaleString()} loaded</span>
    {#if hasMore}
      <span class="text-border">|</span>
      <span>scroll for more</span>
    {/if}
    <span class="text-border">|</span>
    <button
      type="button"
      onclick={openShortcutsHelp}
      class="hover:text-foreground transition-colors"
    >
      Press <Kbd dim>?</Kbd> for shortcuts
    </button>
    <div class="flex-1"></div>
    <BackgroundActivityIndicator />
    <DriveIndicator />
  </StatusBar>
</div>
