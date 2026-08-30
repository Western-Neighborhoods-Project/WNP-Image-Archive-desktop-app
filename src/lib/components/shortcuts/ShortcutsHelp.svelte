<script lang="ts">
  import {
    shortcutsHelpOpen,
    closeShortcutsHelp,
  } from "$lib/stores/shortcutsHelp";
  import { SHORTCUT_GROUPS } from "$lib/utils/shortcuts";
  import { Kbd, KbdSeq } from "$lib/components/ui/kbd";

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      closeShortcutsHelp();
    }
  }

  function onBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) closeShortcutsHelp();
  }
</script>

<svelte:window onkeydown={$shortcutsHelpOpen ? onKeyDown : undefined} />

{#if $shortcutsHelpOpen}
  <div
    class="fixed inset-0 z-50 flex items-start justify-center pt-24"
    style="background: rgba(9, 9, 11, 0.35); backdrop-filter: blur(2px); -webkit-backdrop-filter: blur(2px);"
    onclick={onBackdropClick}
    role="presentation"
  >
    <div
      class="w-[520px] max-w-[calc(100vw-32px)] bg-popover text-popover-foreground rounded-[10px] overflow-hidden flex flex-col max-h-[70vh]"
      style="box-shadow: 0 0 0 1px rgba(0,0,0,0.08), 0 24px 48px rgba(0,0,0,0.2), 0 4px 12px rgba(0,0,0,0.1);"
    >
      <!-- Header -->
      <div
        class="px-5 py-4 border-b border-border-muted flex items-center"
      >
        <div
          class="text-[15px] font-semibold text-foreground tracking-[-0.2px]"
        >
          Keyboard shortcuts
        </div>
        <div class="flex-1"></div>
        <Kbd>Esc</Kbd>
      </div>

      <!-- Groups. Pointer-interaction groups (grid clicks, drags) are
           left to Settings → Keyboard; this overlay is keyboard-only. -->
      <div class="flex-1 overflow-y-auto px-5 py-4 space-y-5">
        {#each SHORTCUT_GROUPS.filter((g) => !g.pointer) as group (group.title)}
          <div>
            <div
              class="text-[10.5px] font-semibold uppercase tracking-[0.5px] text-muted-foreground mb-2"
            >
              {group.title}
            </div>
            {#if group.sub}
              <div class="text-[11px] text-muted-foreground mb-2">
                {group.sub}
              </div>
            {/if}
            <div class="space-y-1">
              {#each group.items as item (item.label)}
                <div
                  class="flex items-center justify-between gap-4 py-1.5"
                >
                  <div class="flex-1 min-w-0">
                    <div class="text-[13px] text-foreground">
                      {item.label}
                    </div>
                    {#if item.hint}
                      <div class="text-[11px] text-muted-foreground mt-0.5">
                        {item.hint}
                      </div>
                    {/if}
                  </div>
                  {#if item.keys.length === 1}
                    <Kbd>{item.keys[0]}</Kbd>
                  {:else}
                    <KbdSeq keys={item.keys} />
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/each}
      </div>

      <!-- Footer -->
      <div
        class="px-[14px] py-2 border-t border-border-muted bg-sidebar-bg text-[11px] text-muted-foreground"
      >
        Single source of truth for these lives in <span
          class="font-mono">src/lib/utils/shortcuts.ts</span
        >.
      </div>
    </div>
  </div>
{/if}
