<script lang="ts">
  // Settings → Keyboard. Read-only reference for every shortcut the app
  // honors, rendered from the shared metadata in $lib/utils/shortcuts —
  // the same source the in-app cheat sheet (`?` key) reads. Unlike the
  // cheat sheet, this page also lists the pointer-interaction groups.

  import { SHORTCUT_GROUPS } from "$lib/utils/shortcuts";
  import { Kbd, KbdSeq } from "$lib/components/ui/kbd";
</script>

<div class="max-w-[640px] space-y-7">
  {#each SHORTCUT_GROUPS as group (group.title)}
    <section>
      <h3 class="text-[14px] font-semibold text-foreground mb-1">
        {group.title}
      </h3>
      {#if group.sub}
        <p class="text-[12px] text-muted-foreground mb-3">{group.sub}</p>
      {/if}
      <ul class="rounded-md border border-border bg-secondary/30 divide-y divide-border">
        {#each group.items as item (item.label)}
          <li class="flex items-center gap-4 px-3.5 py-2">
            <span class="flex-1 min-w-0">
              <span class="block text-[12.5px] text-foreground">{item.label}</span>
              {#if item.hint}
                <span class="block text-[11px] text-muted-foreground mt-0.5">
                  {item.hint}
                </span>
              {/if}
            </span>
            <span class="flex items-center gap-1">
              {#if item.prose}
                <span class="text-[11.5px] text-muted-foreground italic">
                  {item.keys.join(" ")}
                </span>
              {:else if item.keys.length === 1}
                <Kbd>{item.keys[0]}</Kbd>
              {:else}
                <KbdSeq keys={item.keys} />
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
