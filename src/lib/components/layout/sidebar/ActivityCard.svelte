<script lang="ts">
  import Activity from "@lucide/svelte/icons/activity";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import {
    getRecentActivity,
    type RecentActivityEntry,
  } from "$lib/commands/activity";
  import { activityVersion } from "$lib/stores/activity";
  import { currentView } from "$lib/stores/navigation";
  import { formatRelativeTime } from "$lib/utils/format";

  let entries: RecentActivityEntry[] = $state([]);

  // Re-fetch whenever activityVersion bumps (initial mount = 0, then any edit)
  $effect(() => {
    $activityVersion;
    getRecentActivity(3)
      .then((r) => {
        entries = r;
      })
      .catch((e) => console.error("Failed to load recent activity", e));
  });

  // Persisted collapse state — same shape as SideGroup.
  const STORAGE_KEY = "wnp.sidebar.collapsed.recent-activity";
  let collapsed = $state(false);
  if (typeof localStorage !== "undefined") {
    collapsed = localStorage.getItem(STORAGE_KEY) === "1";
  }

  function toggle() {
    collapsed = !collapsed;
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(STORAGE_KEY, collapsed ? "1" : "0");
    }
  }

  function handleSeeAll(e: MouseEvent) {
    // Stop the click from also toggling the collapsed state via the
    // header's outer button.
    e.stopPropagation();
    currentView.set("audit");
  }
</script>

<div
  class="mx-[10px] mt-2 mb-3 rounded-lg bg-background border border-border text-[11px] text-muted-fg-2"
>
  <!-- Header — clicking anywhere (except "See all") toggles collapsed.
       Using a div with role=button instead of <button> so we can nest
       the inner See-all button inside without invalid HTML. -->
  <div
    role="button"
    tabindex="0"
    onclick={toggle}
    onkeydown={(e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        toggle();
      }
    }}
    aria-expanded={!collapsed}
    class="flex items-center gap-[6px] font-semibold text-foreground text-[11.5px] p-[10px] {collapsed
      ? ''
      : 'mb-2 pb-2 border-b border-border-muted'} cursor-pointer select-none"
  >
    <span class="flex w-3 justify-center text-muted-foreground/70">
      {#if collapsed}
        <ChevronRight size={10} />
      {:else}
        <ChevronDown size={10} />
      {/if}
    </span>
    <Activity size={11} />
    <span>Recent activity</span>
    <div class="flex-1"></div>
    <button
      type="button"
      onclick={handleSeeAll}
      class="font-normal text-[10.5px] text-muted-foreground hover:text-foreground transition-colors"
    >
      See all
    </button>
  </div>

  {#if !collapsed}
    {#if entries.length === 0}
      <div class="text-muted-foreground text-[11px] py-1 p-[10px]">
        No recent edits.
      </div>
    {:else}
      {#each entries as entry, i (entry.id)}
        <div class="leading-[1.4] px-[10px] pb-4">
          <div class="text-foreground text-[11.5px]">
            {entry.changed_by} edited
            <span class="text-muted-foreground">{entry.catalog_number}</span>
            at {formatRelativeTime(entry.changed_at)}
          </div>
        </div>
      {/each}
    {/if}
  {/if}
</div>
