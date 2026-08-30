<script lang="ts">
  // Settings → Keyboard. Read-only reference for every shortcut the
  // app honors. Mirrors the in-app cheat sheet (`?` key) but lives
  // here as the canonical list. Grouped by where they apply.

  import { Kbd, KbdSeq } from "$lib/components/ui/kbd";

  interface ShortcutRow {
    keys: string[]; // each entry rendered as a single Kbd; multiple keys = sequence
    label: string;
  }
  interface ShortcutGroup {
    title: string;
    sub?: string;
    rows: ShortcutRow[];
  }

  const groups: ShortcutGroup[] = [
    {
      title: "Global",
      rows: [
        { keys: ["⌘", "K"], label: "Open command bar" },
        { keys: ["?"], label: "Show this cheat sheet (also)" },
        { keys: ["⌘", ";"], label: "Open Settings" },
        { keys: ["⌘", "⇧", "B"], label: "Report a problem (when debugging is on)" },
        { keys: ["⌘", "⇧", "L"], label: "Log out" },
      ],
    },
    {
      title: "Navigate",
      sub: "Press G then the letter — both keystrokes within ~1s.",
      rows: [
        { keys: ["G", "A"], label: "All images" },
        { keys: ["G", "R"], label: "Recently viewed" },
        { keys: ["G", "Q"], label: "Image requests" },
        { keys: ["G", "I"], label: "Import inbox" },
        { keys: ["G", "L"], label: "Audit log" },
        { keys: ["G", "B"], label: "Backups" },
        { keys: ["G", "S"], label: "Settings (admin only)" },
      ],
    },
    {
      title: "Detail view",
      rows: [
        { keys: ["⌘", "⇧", "S"], label: "Share image" },
      ],
    },
    {
      title: "Library grid",
      rows: [
        { keys: ["click"], label: "Open image in detail view" },
        { keys: ["⌘", "click"], label: "Toggle image in selection" },
        { keys: ["⇧", "click"], label: "Range select from last clicked" },
        { keys: ["drag from empty"], label: "Marquee selection" },
        { keys: ["drag image"], label: "Drag-and-drop into a sidebar collection" },
        { keys: ["⌃", "click"], label: "Open context menu" },
      ],
    },
  ];
</script>

<div class="max-w-[640px] space-y-7">
  {#each groups as group (group.title)}
    <section>
      <h3 class="text-[14px] font-semibold text-foreground mb-1">
        {group.title}
      </h3>
      {#if group.sub}
        <p class="text-[12px] text-muted-foreground mb-3">{group.sub}</p>
      {/if}
      <ul class="rounded-md border border-border bg-secondary/30 divide-y divide-border">
        {#each group.rows as row (row.label)}
          <li class="flex items-center gap-4 px-3.5 py-2">
            <span class="flex-1 text-[12.5px] text-foreground">{row.label}</span>
            <span class="flex items-center gap-1">
              {#if row.keys.length === 1 && /^[a-z]/i.test(row.keys[0]) === false && row.keys[0].length <= 2}
                <Kbd>{row.keys[0]}</Kbd>
              {:else if row.keys.length === 1}
                <span class="text-[11.5px] text-muted-foreground italic">
                  {row.keys[0]}
                </span>
              {:else}
                <KbdSeq keys={row.keys} />
              {/if}
            </span>
          </li>
        {/each}
      </ul>
    </section>
  {/each}

  <p class="text-[11.5px] text-muted-foreground italic">
    Shortcuts are fixed for now — remapping would land in a future build.
  </p>
</div>
