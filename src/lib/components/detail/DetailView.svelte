<script lang="ts">
  import { convertFileSrc } from '@tauri-apps/api/core';
  import {
    getImage,
    updateImageMetadata,
    writeMetadataToFile,
    logImageView,
    type ImageRecord,
    type FieldChange
  } from '$lib/commands/images';
  import { parseKeywords, formatFileSize } from '$lib/utils/format';
  import * as Dialog from '$lib/components/ui/dialog';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Textarea } from '$lib/components/ui/textarea';
  import { Separator } from '$lib/components/ui/separator';
  import { Badge } from '$lib/components/ui/badge';

  // ── Props ────────────────────────────────────────────────────
  let { imageId, onBack }: { imageId: number; onBack: () => void } = $props();

  // ── State ────────────────────────────────────────────────────
  let image = $state<ImageRecord | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Form fields — editable copy of image data
  let title = $state('');
  let description = $state('');
  let city = $state('');
  let stateField = $state('');
  let country = $state('');
  let keywords = $state(''); // comma-separated in the UI
  let dateDisplay = $state('');
  let dateStart = $state('');
  let dateEnd = $state('');
  let photographer = $state('');
  let donor = $state('');
  let acquisitionDate = $state('');
  let usageRights = $state('');
  let internalNotes = $state('');

  // Collapsible sections
  let archivalOpen = $state(true);
  let notesOpen = $state(true);
  let fileInfoOpen = $state(false);

  // Save dialog
  let showSaveDialog = $state(false);
  let pendingChanges = $state<FieldChange[]>([]);
  let saving = $state(false);

  // Write to File
  let writeStatus = $state<'idle' | 'writing' | 'success' | 'error'>('idle');
  let writeError = $state<string | null>(null);

  // Zoom state
  let zoomed = $state(false);

  // ── Load image ───────────────────────────────────────────────
  async function loadImage() {
    loading = true;
    error = null;
    try {
      image = await getImage(imageId);
      populateForm(image);
      // Log this view (non-fatal)
      logImageView(imageId).catch(() => {});
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function populateForm(img: ImageRecord) {
    title = img.title ?? '';
    description = img.description ?? '';
    city = img.city ?? '';
    stateField = img.state ?? '';
    country = img.country ?? '';
    keywords = parseKeywords(img.keywords).join(', ');
    dateDisplay = img.date_display ?? '';
    dateStart = img.date_start ?? '';
    dateEnd = img.date_end ?? '';
    photographer = img.photographer ?? '';
    donor = img.donor ?? '';
    acquisitionDate = img.acquisition_date ?? '';
    usageRights = img.usage_rights ?? '';
    internalNotes = img.internal_notes ?? '';
  }

  $effect(() => {
    if (imageId) loadImage();
  });

  // ── Diff computation ─────────────────────────────────────────
  function fieldVal(v: string): string | null {
    return v.trim() === '' ? null : v.trim();
  }

  function keywordsToJson(raw: string): string | null {
    const kws = raw.split(',').map(k => k.trim()).filter(Boolean);
    return kws.length > 0 ? JSON.stringify(kws) : null;
  }

  function computeChanges(): FieldChange[] {
    if (!image) return [];
    const changes: FieldChange[] = [];
    const check = (field: string, original: string | null, current: string | null) => {
      if (original !== current) changes.push({ field, old_value: original, new_value: current });
    };
    check('title',            image.title,            fieldVal(title));
    check('description',      image.description,      fieldVal(description));
    check('city',             image.city,             fieldVal(city));
    check('state',            image.state,            fieldVal(stateField));
    check('country',          image.country,          fieldVal(country));
    check('keywords',         image.keywords,         keywordsToJson(keywords));
    check('date_display',     image.date_display,     fieldVal(dateDisplay));
    check('date_start',       image.date_start,       fieldVal(dateStart));
    check('date_end',         image.date_end,         fieldVal(dateEnd));
    check('photographer',     image.photographer,     fieldVal(photographer));
    check('donor',            image.donor,            fieldVal(donor));
    check('acquisition_date', image.acquisition_date, fieldVal(acquisitionDate));
    check('usage_rights',     image.usage_rights,     fieldVal(usageRights));
    check('internal_notes',   image.internal_notes,   fieldVal(internalNotes));
    return changes;
  }

  let isDirty = $derived.by(() => {
    if (!image) return false;
    return computeChanges().length > 0;
  });

  // ── Save flow ────────────────────────────────────────────────
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
      // Reload to show updated values and reset dirty state
      await loadImage();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  // ── Write to File ────────────────────────────────────────────
  async function handleWriteToFile() {
    writeStatus = 'writing';
    writeError = null;
    try {
      await writeMetadataToFile(imageId);
      writeStatus = 'success';
      // Reload so metadata_synced reflects the new state
      await loadImage();
      setTimeout(() => { writeStatus = 'idle'; }, 3000);
    } catch (e) {
      writeStatus = 'error';
      writeError = String(e);
    }
  }

  // ── Helpers ──────────────────────────────────────────────────
  function fieldLabel(field: string): string {
    const labels: Record<string, string> = {
      title: 'Title', description: 'Description', city: 'City', state: 'State',
      country: 'Country', keywords: 'Keywords', date_display: 'Date (display)',
      date_start: 'Date start', date_end: 'Date end', photographer: 'Photographer',
      donor: 'Donor', acquisition_date: 'Acquisition date', usage_rights: 'Usage rights',
      internal_notes: 'Internal notes',
    };
    return labels[field] ?? field;
  }

  function formatValue(v: string | null): string {
    return v ?? '(empty)';
  }

  const USAGE_RIGHTS_OPTIONS = [
    'Public Domain',
    'Editorial Only',
    'No Commercial Use',
    'Contact for Permission',
    'Unknown',
  ];
</script>

{#if loading}
  <div class="flex h-full items-center justify-center text-gray-400">Loading…</div>

{:else if error}
  <div class="flex h-full flex-col items-center justify-center gap-4 p-8 text-center">
    <p class="text-red-600">{error}</p>
    <Button variant="outline" onclick={onBack}>← Back to Library</Button>
  </div>

{:else if image}
  <div class="flex h-full flex-col overflow-hidden">
    <!-- ── Top bar ──────────────────────────────────────────── -->
    <div class="flex shrink-0 items-center justify-between border-b border-gray-200 bg-white px-4 py-3">
      <button
        onclick={onBack}
        class="flex items-center gap-1.5 text-sm text-gray-500 hover:text-gray-900"
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
          <path stroke-linecap="round" stroke-linejoin="round" d="M15 19l-7-7 7-7" />
        </svg>
        Back to Library
      </button>
      <span class="text-sm font-medium text-gray-700">{image.catalog_number}</span>
      <div class="flex items-center gap-2">
        {#if !image.metadata_synced}
          <span class="text-xs text-amber-600">Unsaved to file</span>
        {/if}
        <Button
          variant="outline"
          size="sm"
          disabled={writeStatus === 'writing'}
          onclick={handleWriteToFile}
        >
          {#if writeStatus === 'writing'}Writing…
          {:else if writeStatus === 'success'}Written ✓
          {:else}Write to File
          {/if}
        </Button>
        <Button size="sm" disabled={!isDirty || saving} onclick={handleSaveClick}>
          {saving ? 'Saving…' : 'Save'}
        </Button>
      </div>
    </div>

    <!-- Write to File error (inline, non-blocking) -->
    {#if writeStatus === 'error' && writeError}
      <div class="shrink-0 bg-red-50 px-4 py-2 text-xs text-red-700">
        Write to file failed: {writeError}
      </div>
    {/if}

    <!-- ── Body: image preview + metadata form ──────────────── -->
    <div class="flex min-h-0 flex-1 overflow-hidden">

      <!-- Left: Image preview -->
      <div class="flex w-[45%] shrink-0 flex-col items-center justify-center border-r border-gray-200 bg-gray-50 p-4">
        <button
          onclick={() => (zoomed = !zoomed)}
          class="cursor-zoom-in border-0 bg-transparent p-0"
          aria-label={zoomed ? 'Zoom out' : 'Zoom in'}
        >
          <img
            src={convertFileSrc(image.file_path)}
            alt={image.catalog_number}
            class="max-h-full max-w-full rounded object-contain shadow"
            onerror={(e) => { (e.currentTarget as HTMLImageElement).style.display = 'none'; }}
          />
        </button>
        {#if zoomed}
          <button
            class="fixed inset-0 z-40 cursor-zoom-out border-0 bg-black/60 p-0"
            onclick={() => (zoomed = false)}
            aria-label="Close zoom"
          ></button>
          <button
            class="fixed inset-0 z-50 m-auto flex cursor-zoom-out items-center justify-center border-0 bg-transparent p-0"
            onclick={() => (zoomed = false)}
            aria-label="Close zoom"
          >
            <img
              src={convertFileSrc(image.file_path)}
              alt={image.catalog_number}
              class="max-h-[90vh] max-w-[90vw] rounded object-contain shadow-2xl"
            />
          </button>
        {/if}
      </div>

      <!-- Right: Metadata form -->
      <div class="min-h-0 flex-1 overflow-y-auto p-6">
        <div class="mx-auto max-w-xl space-y-5">

          <!-- Primary fields -->
          <div class="space-y-4">
            <div class="space-y-1.5">
              <Label for="title">Title</Label>
              <Input id="title" bind:value={title} placeholder="Image title" />
            </div>
            <div class="space-y-1.5">
              <Label for="description">Description</Label>
              <Textarea id="description" bind:value={description} placeholder="Caption or description" rows={3} />
            </div>
            <div class="grid grid-cols-3 gap-3">
              <div class="space-y-1.5">
                <Label for="city">City</Label>
                <Input id="city" bind:value={city} placeholder="City" />
              </div>
              <div class="space-y-1.5">
                <Label for="state">State</Label>
                <Input id="state" bind:value={stateField} placeholder="State" />
              </div>
              <div class="space-y-1.5">
                <Label for="country">Country</Label>
                <Input id="country" bind:value={country} placeholder="Country" />
              </div>
            </div>
            <div class="space-y-1.5">
              <Label for="keywords">Keywords</Label>
              <Input id="keywords" bind:value={keywords} placeholder="Comma-separated keywords" />
              {#if keywords.trim()}
                <div class="flex flex-wrap gap-1 pt-1">
                  {#each keywords.split(',').map(k => k.trim()).filter(Boolean) as kw}
                    <Badge variant="secondary">{kw}</Badge>
                  {/each}
                </div>
              {/if}
            </div>
            <div class="grid grid-cols-3 gap-3">
              <div class="space-y-1.5">
                <Label for="dateDisplay">Date display</Label>
                <Input id="dateDisplay" bind:value={dateDisplay} placeholder="ca. 1920" />
              </div>
              <div class="space-y-1.5">
                <Label for="dateStart">Date start</Label>
                <Input id="dateStart" bind:value={dateStart} placeholder="YYYY-MM-DD" />
              </div>
              <div class="space-y-1.5">
                <Label for="dateEnd">Date end</Label>
                <Input id="dateEnd" bind:value={dateEnd} placeholder="YYYY-MM-DD" />
              </div>
            </div>
            <div class="space-y-1.5">
              <Label for="photographer">Photographer</Label>
              <Input id="photographer" bind:value={photographer} placeholder="Photographer name" />
            </div>
          </div>

          <Separator />

          <!-- Archival Details (collapsible) -->
          <div>
            <button
              class="flex w-full items-center justify-between text-sm font-medium text-gray-700"
              onclick={() => (archivalOpen = !archivalOpen)}
            >
              Archival Details
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 transition-transform {archivalOpen ? 'rotate-180' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" />
              </svg>
            </button>
            {#if archivalOpen}
              <div class="mt-3 space-y-4">
                <div class="grid grid-cols-2 gap-3">
                  <div class="space-y-1.5">
                    <Label for="donor">Donor</Label>
                    <Input id="donor" bind:value={donor} placeholder="Donor name" />
                  </div>
                  <div class="space-y-1.5">
                    <Label for="acquisitionDate">Acquisition date</Label>
                    <Input id="acquisitionDate" bind:value={acquisitionDate} placeholder="YYYY-MM-DD" />
                  </div>
                </div>
                <div class="space-y-1.5">
                  <Label for="usageRights">Usage rights</Label>
                  <select
                    id="usageRights"
                    bind:value={usageRights}
                    class="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-xs transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                  >
                    <option value="">— Select —</option>
                    {#each USAGE_RIGHTS_OPTIONS as opt}
                      <option value={opt}>{opt}</option>
                    {/each}
                  </select>
                </div>
                {#if image.archival_collection}
                  <div class="space-y-1.5">
                    <Label>Archive collection</Label>
                    <p class="text-sm text-gray-600">{image.archival_collection}</p>
                  </div>
                {/if}
              </div>
            {/if}
          </div>

          <Separator />

          <!-- Internal Notes (collapsible) -->
          <div>
            <button
              class="flex w-full items-center justify-between text-sm font-medium text-gray-700"
              onclick={() => (notesOpen = !notesOpen)}
            >
              Internal Notes
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 transition-transform {notesOpen ? 'rotate-180' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" />
              </svg>
            </button>
            {#if notesOpen}
              <div class="mt-3">
                <Textarea
                  bind:value={internalNotes}
                  placeholder="Working notes — not shared externally"
                  rows={4}
                />
              </div>
            {/if}
          </div>

          <Separator />

          <!-- File Information (collapsible) -->
          <div>
            <button
              class="flex w-full items-center justify-between text-sm font-medium text-gray-700"
              onclick={() => (fileInfoOpen = !fileInfoOpen)}
            >
              File Information
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 transition-transform {fileInfoOpen ? 'rotate-180' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" />
              </svg>
            </button>
            {#if fileInfoOpen}
              <div class="mt-3 space-y-2 text-sm">
                {#each [
                  ['Catalog number', image.catalog_number],
                  ['File path', image.file_path],
                  ['File size', image.file_size != null ? formatFileSize(image.file_size) : '—'],
                  ['File modified', image.file_modified ?? '—'],
                  ['Metadata synced', image.metadata_synced ? 'Yes' : 'No — local changes'],
                ] as [label, value]}
                  <div class="flex gap-2">
                    <span class="w-36 shrink-0 text-gray-500">{label}</span>
                    <span class="break-all text-gray-800">{value}</span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>

        </div>
      </div>
    </div>
  </div>
{/if}

<!-- ── Save confirmation dialog ─────────────────────────────── -->
<Dialog.Root bind:open={showSaveDialog}>
  <Dialog.Content class="max-w-md">
    <Dialog.Header>
      <Dialog.Title>Save changes?</Dialog.Title>
      <Dialog.Description>The following fields will be updated:</Dialog.Description>
    </Dialog.Header>
    <div class="my-4 max-h-64 space-y-2 overflow-y-auto">
      {#each pendingChanges as change}
        <div class="rounded-md bg-gray-50 px-3 py-2 text-sm">
          <span class="font-medium">{fieldLabel(change.field)}</span>
          <div class="mt-1 flex gap-2 text-xs">
            <span class="text-red-500 line-through">{formatValue(change.old_value)}</span>
            <span class="text-gray-400">→</span>
            <span class="text-green-600">{formatValue(change.new_value)}</span>
          </div>
        </div>
      {/each}
    </div>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (showSaveDialog = false)}>Cancel</Button>
      <Button disabled={saving} onclick={confirmSave}>
        {saving ? 'Saving…' : 'Save'}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
