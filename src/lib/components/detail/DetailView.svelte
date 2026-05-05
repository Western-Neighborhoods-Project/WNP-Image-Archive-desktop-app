<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { detailWindowTitle } from "$lib/stores/navigation";
  import {
    getImage,
    updateImageMetadata,
    writeMetadataToFile,
    logImageView,
    getRecentlyViewed,
    getAuditLog,
    type ImageRecord,
    type FieldChange,
    type AuditLogEntry,
  } from "$lib/commands/images";
  import { parseKeywords, formatFileSize, formatRelativeTime } from "$lib/utils/format";
  import * as Dialog from "$lib/components/ui/dialog";
  import * as Select from "$lib/components/ui/select";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Textarea } from "$lib/components/ui/textarea";
  import { Badge } from "$lib/components/ui/badge";
  import { Kbd } from "$lib/components/ui/kbd";
  import { getImageCollections } from "$lib/commands/collections";
  import { syncImageFromOpensf } from "$lib/commands/opensfSync";
  import type { Collection } from "$lib/commands/collections";
  import AddToCollectionDialog from "$lib/components/collections/AddToCollectionDialog.svelte";
  import ShareDialog from "$lib/components/sharing/ShareDialog.svelte";
  import { bumpActivity } from "$lib/stores/activity";
  import Filmstrip from "./Filmstrip.svelte";
  import ZoomControls from "./ZoomControls.svelte";

  // Lucide icons
  import ChevronLeft from "@lucide/svelte/icons/chevron-left";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import Share2 from "@lucide/svelte/icons/share-2";
  import Download from "@lucide/svelte/icons/download";
  import Save from "@lucide/svelte/icons/save";
  import EyeOff from "@lucide/svelte/icons/eye-off";
  import X from "@lucide/svelte/icons/x";
  import ArrowRight from "@lucide/svelte/icons/arrow-right";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import ExternalLink from "@lucide/svelte/icons/external-link";

  // ── Props ────────────────────────────────────────────────────
  let { imageId, onBack }: { imageId: number; onBack: () => void } = $props();

  // ── State ────────────────────────────────────────────────────
  let image = $state<ImageRecord | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Tab state
  type TabKey = "metadata" | "history" | "usage";
  let activeTab = $state<TabKey>("metadata");

  // Form fields
  let title = $state("");
  let description = $state("");
  let city = $state("");
  let stateField = $state("");
  let country = $state("");
  let keywords = $state("");
  let dateDisplay = $state("");
  let dateStart = $state("");
  let dateEnd = $state("");
  let photographer = $state("");
  let donor = $state("");
  let acquisitionDate = $state("");
  let usageRights = $state("");
  let internalNotes = $state("");

  // Show all advanced fields toggle (right pane)
  let showAdvanced = $state(false);

  // Filmstrip + nav
  let recentImages = $state<ImageRecord[]>([]);

  // History tab
  let auditEntries = $state<AuditLogEntry[]>([]);
  let auditLoaded = $state(false);

  // Usage tab
  let imageCollections = $state<Collection[]>([]);
  let showAddToCollection = $state(false);

  // Share dialog (Plan 5)
  let showShareDialog = $state(false);

  // Save dialog
  let showSaveDialog = $state(false);
  let pendingChanges = $state<FieldChange[]>([]);
  let saving = $state(false);

  // ⌘⇧S → open share dialog while DetailView is mounted. Local
  // listener (rather than extending the global +page.svelte
  // shortcut map) so it auto-cleans up when the view unmounts.
  onMount(() => {
    function onKeydown(e: KeyboardEvent) {
      if (
        (e.metaKey || e.ctrlKey) &&
        e.shiftKey &&
        e.key.toLowerCase() === "s"
      ) {
        // Don't fire while typing in a field — avoid hijacking ⌘⇧S
        // if a future input ever uses it.
        const t = e.target as HTMLElement | null;
        const tag = t?.tagName;
        if (tag === "INPUT" || tag === "TEXTAREA" || t?.isContentEditable) {
          return;
        }
        e.preventDefault();
        showShareDialog = true;
      }
    }
    window.addEventListener("keydown", onKeydown);
    return () => window.removeEventListener("keydown", onKeydown);
  });

  // Write to File
  let writeStatus = $state<"idle" | "writing" | "success" | "error">("idle");
  let writeError = $state<string | null>(null);

  // Zoom + pan
  let zoomLevel = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let isPanning = $state(false);
  let panStart = { mouseX: 0, mouseY: 0, baseX: 0, baseY: 0 };

  function fitImage() {
    zoomLevel = 1;
    panX = 0;
    panY = 0;
  }

  function setZoom(next: number) {
    const clamped = Math.max(0.25, Math.min(next, 4));
    if (clamped <= 1) {
      // Reset pan when fully zoomed out (prevents drifted-out-of-view state)
      panX = 0;
      panY = 0;
    }
    zoomLevel = clamped;
  }

  function onImageMouseDown(e: MouseEvent) {
    if (zoomLevel <= 1) return;
    isPanning = true;
    panStart = {
      mouseX: e.clientX,
      mouseY: e.clientY,
      baseX: panX,
      baseY: panY,
    };
    e.preventDefault();
  }

  function onImageMouseMove(e: MouseEvent) {
    if (!isPanning) return;
    panX = panStart.baseX + (e.clientX - panStart.mouseX);
    panY = panStart.baseY + (e.clientY - panStart.mouseY);
  }

  function onImageMouseUp() {
    isPanning = false;
  }

  function onImageWheel(e: WheelEvent) {
    if (zoomLevel <= 1) return;
    e.preventDefault();
    panX -= e.deltaX;
    panY -= e.deltaY;
  }

  const USAGE_RIGHTS_OPTIONS = [
    "Public Domain",
    "Editorial Only",
    "No Commercial Use",
    "Contact for Permission",
    "Unknown",
  ];

  // ── Load image ───────────────────────────────────────────────
  // Plan 9: read local first (fast), drop `loading`, render UI, THEN
  // kick off the OpenSFHistory sync as a fire-and-forget background
  // task. The sync swaps the displayed record in place when it
  // returns. This way the detail view never blocks on the network —
  // local data shows immediately, synced fields update whenever the
  // API responds.
  let resyncing = $state(false);

  async function loadImage() {
    loading = true;
    error = null;
    try {
      image = await getImage(imageId);
      populateForm(image);
      logImageView(imageId).catch(() => {});
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
    // Background sync — not awaited. Errors logged, never bubble up.
    void backgroundSync(imageId, false);
  }

  async function backgroundSync(targetImageId: number, force: boolean) {
    if (resyncing) return; // dedupe rapid clicks on Re-sync
    resyncing = true;
    try {
      const synced = await syncImageFromOpensf({
        imageId: targetImageId,
        force,
      });
      // Only apply if the user is still viewing the same image.
      if (synced.id === imageId) {
        image = synced;
        populateForm(synced);
      }
    } catch (e) {
      console.warn("OpenSFHistory sync skipped:", e);
    } finally {
      resyncing = false;
    }
  }

  // Plan 9: helpers for the OpenSFHistory mirror columns. Backend
  // stores neighborhoods, photosets, osf_collections as JSON text;
  // these parse them defensively so a malformed payload doesn't
  // break the detail view.
  function safeParseArray(json: string | null): string[] {
    if (!json) return [];
    try {
      const v = JSON.parse(json);
      return Array.isArray(v) ? v.map(String) : [];
    } catch {
      return [];
    }
  }
  function safeParseObject(json: string | null): Record<string, string> {
    if (!json) return {};
    try {
      const v = JSON.parse(json);
      if (v && typeof v === "object" && !Array.isArray(v)) {
        const out: Record<string, string> = {};
        for (const [k, val] of Object.entries(v)) out[k] = String(val);
        return out;
      }
      return {};
    } catch {
      return {};
    }
  }

  function populateForm(img: ImageRecord) {
    title = img.title ?? "";
    description = img.description ?? "";
    city = img.city ?? "";
    stateField = img.state ?? "";
    country = img.country ?? "";
    keywords = parseKeywords(img.keywords).join(", ");
    dateDisplay = img.date_display ?? "";
    dateStart = img.date_start ?? "";
    dateEnd = img.date_end ?? "";
    photographer = img.photographer ?? "";
    donor = img.donor ?? "";
    acquisitionDate = img.acquisition_date ?? "";
    usageRights = img.usage_rights ?? "";
    internalNotes = img.internal_notes ?? "";
  }

  $effect(() => {
    if (imageId) {
      loadImage();
      loadFilmstrip();
      loadCollections();
      auditLoaded = false; // re-fetch when tab is opened
      fitImage(); // reset zoom/pan whenever the visible image changes
    }
  });

  // Window title — design pattern is "<catalog> — <title>" while in detail
  // view (e.g. 'wnp27.4283 — Sutro Baths, exterior'). Push it into the
  // shared store so WindowChrome (custom) and the OS-level setTitle
  // (mirrored in +page.svelte) both stay in sync. Reset on unmount so
  // returning to library falls back to the per-view suffix.
  $effect(() => {
    if (!image) return;
    const titlePart = image.title || "(untitled)";
    detailWindowTitle.set(`${image.catalog_number} — ${titlePart}`);
  });

  onDestroy(() => {
    detailWindowTitle.set(null);
  });

  async function loadFilmstrip() {
    try {
      recentImages = await getRecentlyViewed();
    } catch {}
  }

  async function loadCollections() {
    try {
      imageCollections = await getImageCollections(imageId);
    } catch {}
  }

  async function loadAudit() {
    if (auditLoaded) return;
    try {
      auditEntries = await getAuditLog(imageId);
      auditLoaded = true;
    } catch {}
  }

  $effect(() => {
    if (activeTab === "history") {
      loadAudit();
    }
  });

  // ── Diff computation ─────────────────────────────────────────
  function fieldVal(v: string): string | null {
    return v.trim() === "" ? null : v.trim();
  }

  function keywordsToJson(raw: string): string | null {
    const kws = raw.split(",").map((k) => k.trim()).filter(Boolean);
    return kws.length > 0 ? JSON.stringify(kws) : null;
  }

  function computeChanges(): FieldChange[] {
    if (!image) return [];
    const changes: FieldChange[] = [];
    const check = (field: string, original: string | null, current: string | null) => {
      if (original !== current)
        changes.push({ field, old_value: original, new_value: current });
    };
    check("title", image.title, fieldVal(title));
    check("description", image.description, fieldVal(description));
    check("city", image.city, fieldVal(city));
    check("state", image.state, fieldVal(stateField));
    check("country", image.country, fieldVal(country));
    check("keywords", image.keywords, keywordsToJson(keywords));
    check("date_display", image.date_display, fieldVal(dateDisplay));
    check("date_start", image.date_start, fieldVal(dateStart));
    check("date_end", image.date_end, fieldVal(dateEnd));
    check("photographer", image.photographer, fieldVal(photographer));
    check("donor", image.donor, fieldVal(donor));
    check("acquisition_date", image.acquisition_date, fieldVal(acquisitionDate));
    check("usage_rights", image.usage_rights, fieldVal(usageRights));
    check("internal_notes", image.internal_notes, fieldVal(internalNotes));
    return changes;
  }

  let isDirty = $derived.by(() => {
    if (!image) return false;
    return computeChanges().length > 0;
  });

  // ── Missing fields list (for the amber pill) ────────────────
  let missingFields = $derived.by(() => {
    if (!image) return [];
    const m: string[] = [];
    if (!image.title) m.push("title");
    if (!image.city) m.push("city");
    if (!image.state) m.push("state");
    if (!image.date_display) m.push("date");
    return m;
  });

  // ── Save ─────────────────────────────────────────────────────
  function handleSaveClick() {
    pendingChanges = computeChanges();
    if (pendingChanges.length === 0) return;
    showSaveDialog = true;
  }

  async function confirmSave() {
    saving = true;
    try {
      await updateImageMetadata({ image_id: imageId, changes: pendingChanges });
      showSaveDialog = false;
      await loadImage();
      auditLoaded = false; // refetch on next History view
      bumpActivity(); // tell sidebar's ActivityCard to refresh
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function handleWriteToFile() {
    writeStatus = "writing";
    writeError = null;
    try {
      await writeMetadataToFile(imageId);
      writeStatus = "success";
      await loadImage();
      setTimeout(() => {
        writeStatus = "idle";
      }, 3000);
    } catch (e) {
      writeStatus = "error";
      writeError = String(e);
    }
  }

  // ── Filmstrip nav ────────────────────────────────────────────
  function selectImage(img: ImageRecord) {
    // Reuse onBack contract — caller can rewire to update currentImageId
    // but DetailView's "imageId" is reactive, so we trigger via parent.
    // Easiest: dispatch a custom event to switch via the navigation store directly.
    // Since DetailView's imageId comes from a store, we can't change it from here
    // without adding a callback. Instead, just dispatch the same callback as the grid did.
    if (img.id !== imageId) {
      // Use the navigation store directly to switch
      import("$lib/stores/navigation").then(({ currentImageId }) => {
        currentImageId.set(img.id);
      });
    }
  }

  let prevImage = $derived.by(() => {
    if (!image) return null;
    const i = recentImages.findIndex((r) => r.id === image!.id);
    return i > 0 ? recentImages[i - 1] : null;
  });
  let nextImage = $derived.by(() => {
    if (!image) return null;
    const i = recentImages.findIndex((r) => r.id === image!.id);
    return i >= 0 && i < recentImages.length - 1 ? recentImages[i + 1] : null;
  });
  let positionLabel = $derived.by(() => {
    if (!image) return "";
    const i = recentImages.findIndex((r) => r.id === image!.id);
    return i >= 0 ? `${i + 1} of ${recentImages.length}` : "";
  });

  // ── Helpers ──────────────────────────────────────────────────
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

  function formatValue(v: string | null): string {
    return v ?? "(empty)";
  }
</script>

{#if loading}
  <div class="flex h-full items-center justify-center text-muted-foreground">
    Loading…
  </div>
{:else if error}
  <div
    class="flex h-full flex-col items-center justify-center gap-4 p-8 text-center"
  >
    <p class="text-destructive">{error}</p>
    <Button variant="outline" onclick={onBack}>← Back to Library</Button>
  </div>
{:else if image}
  <div class="flex h-full flex-col overflow-hidden bg-background min-w-0 select-text">
    <!-- ── Header bar ─────────────────────────────────────────── -->
    <div
      class="h-14 px-5 flex items-center gap-3 border-b border-border bg-background flex-shrink-0"
    >
      <Button variant="ghost" size="xs" onclick={onBack}>
        <ChevronLeft class="size-3.5" />
        Back
      </Button>
      <div class="w-px h-5 bg-border mx-1"></div>
      <div class="flex items-baseline gap-[10px] min-w-0">
        <div
          class="text-[15px] font-semibold text-foreground tracking-[-0.2px] truncate"
        >
          {image.title || "(untitled)"}
        </div>
        <div class="font-mono text-xs text-muted-foreground flex-shrink-0">
          {image.catalog_number}
        </div>
      </div>
      <div class="flex-1"></div>

      {#if positionLabel}
        <span class="text-[11.5px] text-muted-foreground tabular-nums">
          {positionLabel}
        </span>
      {/if}

      <div class="flex gap-0.5">
        <Button
          variant="ghost"
          size="icon-xs"
          disabled={!prevImage}
          onclick={() => prevImage && selectImage(prevImage)}
          aria-label="Previous"
        >
          <ChevronLeft />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          disabled={!nextImage}
          onclick={() => nextImage && selectImage(nextImage)}
          aria-label="Next"
        >
          <ChevronRight />
        </Button>
      </div>

      <div class="w-px h-5 bg-border mx-1"></div>

      <Button
        size="xs"
        variant="outline"
        onclick={() => (showShareDialog = true)}
        title="Share image (⌘⇧S)"
      >
        <Share2 class="size-3.5" />
        Share
      </Button>
      <Button
        size="xs"
        variant="outline"
        disabled={writeStatus === "writing"}
        onclick={handleWriteToFile}
      >
        <Download class="size-3.5" />
        {#if writeStatus === "writing"}Writing…
        {:else if writeStatus === "success"}Written
        {:else}Export
        {/if}
      </Button>
      <Button
        size="xs"
        disabled={!isDirty || saving}
        onclick={handleSaveClick}
      >
        <Save class="size-3.5" />
        {saving ? "Saving…" : "Save changes"}
      </Button>
    </div>

    {#if writeStatus === "error" && writeError}
      <div
        class="shrink-0 bg-destructive/10 text-destructive px-5 py-2 text-xs"
      >
        Write to file failed: {writeError}
      </div>
    {/if}

    <!-- ── Body: image column + right inspector ─────────────── -->
    <div class="flex flex-1 min-h-0">
      <!-- Image column -->
      <div
        class="flex flex-1 flex-col min-w-0 relative"
        style="background: #09090b;"
      >
        <!-- Main image viewport — overflow-hidden so zoomed transforms
             don't bleed over the right inspector or the filmstrip below.
             When zoomed > 1, mouse-drag and wheel scroll pan the image. -->
        <div
          class="flex-1 relative flex items-center justify-center min-h-0 overflow-hidden select-none"
          style="cursor: {zoomLevel > 1 ? (isPanning ? 'grabbing' : 'grab') : 'default'};"
          onmousedown={onImageMouseDown}
          onmousemove={onImageMouseMove}
          onmouseup={onImageMouseUp}
          onmouseleave={onImageMouseUp}
          onwheel={onImageWheel}
          role="presentation"
        >
          <img
            src={convertFileSrc(image.file_path)}
            alt={image.catalog_number}
            draggable="false"
            class="max-h-full max-w-full object-contain rounded-[2px] pointer-events-none"
            style="box-shadow: 0 8px 48px rgba(0,0,0,.4); transform: translate({panX}px, {panY}px) scale({zoomLevel}); transform-origin: center; transition: {isPanning ? 'none' : 'transform 80ms ease-out'};"
            onerror={(e) => {
              (e.currentTarget as HTMLImageElement).style.display = "none";
            }}
          />

          {#if missingFields.length > 0}
            <div
              class="absolute top-4 left-4 px-2.5 py-1 rounded-2xl text-[11px] font-medium"
              style="background: rgba(245,158,11,0.15); color: #fbbf24; border: 1px solid rgba(245,158,11,0.3);"
            >
              Missing: {missingFields.join(", ")}
            </div>
          {/if}

          <div class="absolute bottom-4 left-1/2 -translate-x-1/2">
            <ZoomControls
              {zoomLevel}
              onZoomIn={() => setZoom(zoomLevel + 0.25)}
              onZoomOut={() => setZoom(zoomLevel - 0.25)}
              onFit={fitImage}
            />
          </div>
        </div>

        <!-- Filmstrip -->
        {#if recentImages.length > 0}
          <Filmstrip
            images={recentImages}
            currentId={imageId}
            onSelect={selectImage}
          />
        {/if}
      </div>

      <!-- Right inspector -->
      <div
        class="w-[400px] flex-shrink-0 flex flex-col min-h-0 border-l border-border bg-sidebar-bg"
      >
        <!-- Tabs -->
        <div
          class="flex items-end px-4 gap-5 border-b border-border bg-background flex-shrink-0"
        >
          {#each [{ key: "metadata", label: "Metadata" }, { key: "history", label: "History" }, { key: "usage", label: "Usage" }] as tab}
            <button
              type="button"
              onclick={() => (activeTab = tab.key as TabKey)}
              class="py-3.5 flex items-center gap-1.5 text-[13px] font-medium border-b-2 -mb-px transition-colors
                {activeTab === tab.key
                ? 'text-foreground border-foreground'
                : 'text-muted-foreground border-transparent hover:text-foreground'}"
            >
              {tab.label}
              {#if tab.key === "history" && auditEntries.length > 0}
                <Badge variant="secondary">{auditEntries.length}</Badge>
              {/if}
              {#if tab.key === "usage" && imageCollections.length > 0}
                <Badge variant="secondary">{imageCollections.length}</Badge>
              {/if}
            </button>
          {/each}
        </div>

        <div class="flex-1 overflow-y-auto p-5">
          {#if activeTab === "metadata"}
            <!-- Standard group -->
            <div class="space-y-3.5">
              <div class="space-y-1.5">
                <div
                  class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
                >
                  Title
                </div>
                <Input bind:value={title} placeholder="Image title" disabled />
              </div>

              <div class="space-y-1.5">
                <div
                  class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
                >
                  Description
                </div>
                <Textarea
                  bind:value={description}
                  placeholder="Caption or description"
                  rows={3}
                  disabled
                />
              </div>

              <div class="grid grid-cols-2 gap-3">
                <div class="space-y-1.5">
                  <div
                    class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
                  >
                    City
                  </div>
                  <Input bind:value={city} placeholder="City" disabled />
                </div>
                <div class="space-y-1.5">
                  <div
                    class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
                  >
                    State
                  </div>
                  <Input bind:value={stateField} placeholder="State" disabled />
                </div>
              </div>

              <div class="grid grid-cols-2 gap-3">
                <div class="space-y-1.5">
                  <div
                    class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
                  >
                    Date (display)
                  </div>
                  <Input bind:value={dateDisplay} placeholder="ca. 1920" disabled />
                </div>
                <div class="space-y-1.5">
                  <div
                    class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground flex items-center gap-1.5"
                  >
                    Date range
                    <span class="text-[10.5px] normal-case tracking-normal text-muted-foreground/70">for filtering</span>
                  </div>
                  <div class="flex gap-1.5 items-center">
                    <Input bind:value={dateStart} placeholder="YYYY-MM-DD" size="xs" class="font-mono" disabled />
                    <span class="text-muted-foreground">–</span>
                    <Input bind:value={dateEnd} placeholder="YYYY-MM-DD" size="xs" class="font-mono" />
                  </div>
                </div>
              </div>

              <div class="space-y-1.5">
                <div
                  class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
                >
                  Keywords
                </div>
                <Input
                  bind:value={keywords}
                  placeholder="Comma-separated"
                />
                {#if keywords.trim()}
                  <div class="flex flex-wrap gap-1 pt-1">
                    {#each keywords.split(",").map((k) => k.trim()).filter(Boolean) as kw}
                      <Badge variant="secondary">{kw}</Badge>
                    {/each}
                  </div>
                {/if}
              </div>

              <div class="space-y-1.5">
                <div
                  class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
                >
                  Photographer
                </div>
                <Input bind:value={photographer} placeholder="Unknown" disabled />
              </div>

              {#if showAdvanced}
                <div class="space-y-1.5">
                  <div
                    class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
                  >
                    Country
                  </div>
                  <Input bind:value={country} placeholder="Country" disabled />
                </div>
              {/if}
            </div>

            <!-- Archival group -->
            <div class="mt-5 mb-3 pt-3.5 border-t border-border">
              <div
                class="text-[11px] font-semibold uppercase tracking-[0.4px] text-muted-foreground mb-3"
              >
                Archival
              </div>

              <div class="space-y-3.5">
                <div class="grid grid-cols-2 gap-3">
                  <div class="space-y-1.5">
                    <div
                      class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
                    >
                      Donor
                    </div>
                    <Input bind:value={donor} size="xs" class="font-mono" />
                  </div>
                  <div class="space-y-1.5">
                    <div
                      class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
                    >
                      Acquired
                    </div>
                    <Input
                      bind:value={acquisitionDate}
                      placeholder="YYYY-MM-DD"
                      size="xs"
                      class="font-mono"
                    />
                  </div>
                </div>

                {#if image.archival_collection}
                  <div class="space-y-1.5">
                    <div
                      class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
                    >
                      Archival collection
                    </div>
                    <div
                      class="text-[13px] text-muted-fg-2 font-mono py-2"
                    >
                      {image.archival_collection}
                    </div>
                  </div>
                {/if}

                <div class="space-y-1.5">
                  <div
                    class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground"
                  >
                    Usage rights
                  </div>
                  <Select.Root
                    type="single"
                    value={usageRights}
                    onValueChange={(v) => (usageRights = v ?? "")}
                    disabled
                  >
                    <Select.Trigger class="w-full">
                      {usageRights || "— Select —"}
                    </Select.Trigger>
                    <Select.Content>
                      {#each USAGE_RIGHTS_OPTIONS as opt}
                        <Select.Item value={opt}>{opt}</Select.Item>
                      {/each}
                    </Select.Content>
                  </Select.Root>
                </div>

                <div class="space-y-1.5">
                  <div
                    class="text-[11.5px] font-medium uppercase tracking-[0.4px] text-muted-foreground flex items-center gap-1.5"
                  >
                    Internal notes
                    <span class="text-[10.5px] normal-case tracking-normal text-muted-foreground/70">never written to file</span>
                  </div>
                  <Textarea
                    bind:value={internalNotes}
                    rows={3}
                    placeholder="Working notes — not shared externally"
                  />
                </div>
              </div>
            </div>

            <!-- Plan 9: OpenSFHistory mirror data. Always rendered so
                 the Re-sync control is reachable even before the first
                 successful sync. Read-only since the API is the source
                 of truth; a future plan lifts the lock on the inputs
                 above when push-back is wired. -->
            <div class="mt-5 pt-4 border-t border-border space-y-3.5">
              <div class="flex items-center justify-between gap-2">
                <div
                  class="text-[11px] font-semibold uppercase tracking-[0.4px] text-muted-foreground"
                >
                  From OpenSFHistory
                </div>
                <div class="flex items-center gap-2 text-[10.5px] text-muted-foreground">
                  {#if resyncing}
                    <span>Syncing…</span>
                  {:else if image.last_synced_at}
                    <span>Synced {formatRelativeTime(image.last_synced_at)}</span>
                  {:else}
                    <span class="italic">Never synced</span>
                  {/if}
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    onclick={() => backgroundSync(imageId, true)}
                    disabled={resyncing}
                    title="Re-sync from OpenSFHistory"
                  >
                    <RefreshCw class={resyncing ? "size-3 animate-spin" : "size-3"} />
                  </Button>
                </div>
              </div>

                {#if image.caption}
                  <div class="space-y-1">
                    <div class="text-[11px] uppercase tracking-[0.4px] text-muted-foreground">
                      Caption
                    </div>
                    <p class="text-[12.5px] text-foreground">{image.caption}</p>
                  </div>
                {/if}

                {#if image.citation}
                  <div class="space-y-1">
                    <div class="text-[11px] uppercase tracking-[0.4px] text-muted-foreground">
                      Citation
                    </div>
                    <p class="text-[12px] text-muted-fg-2 italic leading-snug">
                      {image.citation}
                    </p>
                  </div>
                {/if}

                {#if image.publisher}
                  <div class="space-y-1">
                    <div class="text-[11px] uppercase tracking-[0.4px] text-muted-foreground">
                      Publisher
                    </div>
                    <p class="text-[12.5px]">{image.publisher}</p>
                  </div>
                {/if}

                {#if image.dimensions || image.format || image.download_permitted === 0}
                  <div class="flex flex-wrap gap-1.5">
                    {#if image.dimensions}
                      <Badge variant="outline">{image.dimensions}</Badge>
                    {/if}
                    {#if image.format}
                      <Badge variant="outline">{image.format}</Badge>
                    {/if}
                    {#if image.download_permitted === 0}
                      <Badge variant="destructive">Download restricted</Badge>
                    {/if}
                  </div>
                {/if}

                {#if image.neighborhoods}
                  {@const items = safeParseArray(image.neighborhoods)}
                  {#if items.length > 0}
                    <div class="space-y-1">
                      <div class="text-[11px] uppercase tracking-[0.4px] text-muted-foreground">
                        Neighborhoods
                      </div>
                      <div class="flex flex-wrap gap-1">
                        {#each items as slug (slug)}
                          <span class="px-2 py-0.5 rounded bg-secondary text-[11px]">
                            {slug}
                          </span>
                        {/each}
                      </div>
                    </div>
                  {/if}
                {/if}

                {#if image.photosets}
                  {@const sets = safeParseObject(image.photosets)}
                  {#if Object.keys(sets).length > 0}
                    <div class="space-y-1">
                      <div class="text-[11px] uppercase tracking-[0.4px] text-muted-foreground">
                        Photosets
                      </div>
                      <div class="flex flex-wrap gap-1">
                        {#each Object.entries(sets) as [id, title] (id)}
                          <span class="px-2 py-0.5 rounded bg-secondary text-[11px]">
                            {title}
                          </span>
                        {/each}
                      </div>
                    </div>
                  {/if}
                {/if}

                {#if image.osf_collections}
                  {@const items = safeParseArray(image.osf_collections)}
                  {#if items.length > 0}
                    <div class="space-y-1">
                      <div class="text-[11px] uppercase tracking-[0.4px] text-muted-foreground">
                        OSF Collections
                      </div>
                      <div class="flex flex-wrap gap-1">
                        {#each items as name (name)}
                          <span class="px-2 py-0.5 rounded bg-secondary text-[11px]">
                            {name}
                          </span>
                        {/each}
                      </div>
                    </div>
                  {/if}
                {/if}

                {#if image.osf_page_url}
                  <a
                    href={image.osf_page_url}
                    target="_blank"
                    rel="noopener"
                    class="inline-flex items-center gap-1 text-[12px] text-info hover:underline"
                  >
                    <ExternalLink class="size-3" />
                    View on OpenSFHistory
                  </a>
                {/if}

                {#if !image.last_synced_at && !image.caption && !image.citation && !image.publisher && !image.dimensions && !image.format && !image.osf_page_url}
                  <p class="text-[11.5px] text-muted-foreground italic">
                    No synced data yet. Click the refresh icon to fetch
                    metadata from OpenSFHistory.
                  </p>
                {/if}
              </div>

            <!-- File info (collapsed under Advanced) -->
            {#if showAdvanced}
              <div class="mt-5 pt-3.5 border-t border-border space-y-2 text-xs">
                <div
                  class="text-[11px] font-semibold uppercase tracking-[0.4px] text-muted-foreground mb-2"
                >
                  File
                </div>
                <div class="flex gap-2">
                  <span class="w-32 shrink-0 text-muted-foreground">Path</span>
                  <span class="break-all text-muted-fg-2 font-mono text-[11px]"
                    >{image.file_path}</span
                  >
                </div>
                <div class="flex gap-2">
                  <span class="w-32 shrink-0 text-muted-foreground">Size</span>
                  <span class="text-muted-fg-2"
                    >{image.file_size != null
                      ? formatFileSize(image.file_size)
                      : "—"}</span
                  >
                </div>
                <div class="flex gap-2">
                  <span class="w-32 shrink-0 text-muted-foreground">Modified</span>
                  <span class="text-muted-fg-2"
                    >{image.file_modified ?? "—"}</span
                  >
                </div>
                <div class="flex gap-2">
                  <span class="w-32 shrink-0 text-muted-foreground">Synced to file</span>
                  <span class="text-muted-fg-2"
                    >{image.metadata_synced ? "Yes" : "No — local only"}</span
                  >
                </div>
              </div>
            {/if}

            <div
              class="mt-3.5 pt-3.5 border-t border-border flex items-center gap-1.5 text-[11.5px]"
            >
              <EyeOff class="size-3 text-muted-foreground" />
              <span class="text-muted-foreground">
                {showAdvanced ? "Advanced fields shown" : "Country + file info hidden"}
              </span>
              <span class="flex-1"></span>
              <button
                type="button"
                onclick={() => (showAdvanced = !showAdvanced)}
                class="text-foreground font-medium hover:underline"
              >
                {showAdvanced ? "Hide" : "Show all"}
              </button>
            </div>
          {:else if activeTab === "history"}
            {#if auditEntries.length === 0}
              <p class="text-muted-foreground text-sm">No edits recorded yet.</p>
            {:else}
              <div class="space-y-3">
                {#each auditEntries as entry (entry.id)}
                  <div
                    class="border border-border rounded-md p-3 bg-background"
                  >
                    <div class="flex items-baseline gap-2 mb-1">
                      <span class="text-[12.5px] font-medium text-foreground">
                        {fieldLabel(entry.field_name)}
                      </span>
                      <span class="text-[11px] text-muted-foreground">
                        {formatRelativeTime(entry.changed_at)}
                      </span>
                    </div>
                    <div
                      class="flex gap-1.5 items-center text-[11.5px] font-mono"
                    >
                      <span
                        class="px-1.5 py-0.5 rounded text-destructive"
                        style="background: hsl(var(--destructive) / 0.08); text-decoration: line-through; text-decoration-color: hsl(var(--destructive) / 0.3);"
                        >{formatValue(entry.old_value)}</span
                      >
                      <ArrowRight
                        class="size-3 text-muted-foreground flex-shrink-0"
                      />
                      <span
                        class="px-1.5 py-0.5 rounded"
                        style="background: hsl(var(--success) / 0.08); color: hsl(var(--success));"
                        >{formatValue(entry.new_value)}</span
                      >
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          {:else if activeTab === "usage"}
            <div
              class="text-[11px] font-semibold uppercase tracking-[0.4px] text-muted-foreground mb-3"
            >
              Collections
            </div>
            {#if imageCollections.length === 0}
              <p class="text-muted-foreground text-sm">
                Not in any user collections yet.
              </p>
            {:else}
              <div class="flex flex-wrap gap-1.5 mb-4">
                {#each imageCollections as col (col.id)}
                  <Badge variant="secondary">{col.name}</Badge>
                {/each}
              </div>
            {/if}
            <Button
              variant="outline"
              size="xs"
              onclick={() => (showAddToCollection = true)}
            >
              Manage collections…
            </Button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Add to Collection -->
<AddToCollectionDialog
  bind:open={showAddToCollection}
  imageId={imageId}
  onclose={loadCollections}
/>

<!-- Plan 5: Share dialog -->
{#if image}
  <ShareDialog bind:open={showShareDialog} {image} />
{/if}

<!-- Save confirmation dialog -->
<Dialog.Root bind:open={showSaveDialog}>
  <Dialog.Content class="max-w-md">
    <Dialog.Header>
      <Dialog.Title>Save changes?</Dialog.Title>
      <Dialog.Description>The following fields will be updated:</Dialog.Description>
    </Dialog.Header>
    <div class="my-4 max-h-64 space-y-2 overflow-y-auto">
      {#each pendingChanges as change}
        <div class="rounded-md bg-secondary px-3 py-2 text-sm">
          <span class="font-medium">{fieldLabel(change.field)}</span>
          <div class="mt-1 flex gap-2 text-xs items-center">
            <span class="text-destructive line-through font-mono">
              {formatValue(change.old_value)}
            </span>
            <X class="size-3 text-muted-foreground" />
            <span class="font-mono" style="color: hsl(var(--success));">
              {formatValue(change.new_value)}
            </span>
          </div>
        </div>
      {/each}
    </div>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (showSaveDialog = false)}>
        <Kbd dim>Esc</Kbd>
        Cancel
      </Button>
      <Button disabled={saving} onclick={confirmSave}>
        {saving ? "Saving…" : "Save"}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
