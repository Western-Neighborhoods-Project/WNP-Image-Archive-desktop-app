<script lang="ts">
  import { onMount, type Snippet } from "svelte";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import ChevronDown from "@lucide/svelte/icons/chevron-down";

  interface Props {
    title: string;
    action?: Snippet;
    children: Snippet;
    /** When true the collapsed state is not persisted across sessions.
     *  Useful for transient groups; default is to remember each user's
     *  collapse choice in localStorage keyed on the title. */
    transient?: boolean;
  }

  let { title, action, children, transient = false }: Props = $props();

  // localStorage-backed collapse state. Read in onMount so the prop
  // access happens after mount (avoids Svelte 5's
  // state_referenced_locally warning). Keyed on title.
  let collapsed = $state(false);

  onMount(() => {
    if (typeof localStorage === "undefined" || transient) return;
    collapsed = localStorage.getItem(`wnp.sidebar.collapsed.${title}`) === "1";
  });

  function toggle() {
    collapsed = !collapsed;
    if (typeof localStorage === "undefined" || transient) return;
    localStorage.setItem(
      `wnp.sidebar.collapsed.${title}`,
      collapsed ? "1" : "0",
    );
  }
</script>

<div class="mb-[14px]">
  <div
    class="flex items-center justify-between pr-[14px] text-[11px] font-medium tracking-[0.3px] text-muted-foreground uppercase"
  >
    <button
      type="button"
      onclick={toggle}
      class="flex flex-1 items-center gap-1 pl-[8px] pr-1 pb-[6px] text-left hover:text-foreground transition-colors"
      aria-expanded={!collapsed}
    >
      <span class="flex w-3 justify-center text-muted-foreground/70">
        {#if collapsed}
          <ChevronRight size={10} />
        {:else}
          <ChevronDown size={10} />
        {/if}
      </span>
      <span>{title}</span>
    </button>
    {#if action}
      <span
        class="cursor-pointer normal-case tracking-normal text-xs text-muted-foreground flex items-center pb-[6px]"
      >
        {@render action()}
      </span>
    {/if}
  </div>
  {#if !collapsed}
    <div>
      {@render children()}
    </div>
  {/if}
</div>
