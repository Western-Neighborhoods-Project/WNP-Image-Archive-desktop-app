<script lang="ts">
  import type { Component } from "svelte";
  import type { SettingsPageKey } from "$lib/stores/navigation";
  import Settings from "@lucide/svelte/icons/settings";
  import Share2 from "@lucide/svelte/icons/share-2";
  import Cable from "@lucide/svelte/icons/cable";
  import User from "@lucide/svelte/icons/user";
  import BookOpen from "@lucide/svelte/icons/book-open";
  import Bug from "@lucide/svelte/icons/bug";

  interface NavItem {
    key: SettingsPageKey;
    label: string;
    icon: Component;
  }

  interface Props {
    active: SettingsPageKey;
    onSelect: (key: SettingsPageKey) => void;
  }

  let { active, onSelect }: Props = $props();

  const items: NavItem[] = [
    { key: "general", label: "General", icon: Settings },
    { key: "sharing", label: "Sharing", icon: Share2 },
    { key: "external", label: "External services", icon: Cable },
    { key: "users", label: "Users", icon: User },
    { key: "keyboard", label: "Keyboard", icon: BookOpen },
    { key: "debugging", label: "Debugging", icon: Bug },
  ];
</script>

<nav
  class="w-[220px] flex-shrink-0 border-r border-border bg-sidebar-bg pt-4"
>
  <div
    class="px-[18px] pb-[10px] text-[11px] font-semibold uppercase tracking-[0.4px] text-muted-foreground"
  >
    Settings
  </div>
  {#each items as item (item.key)}
    {@const Icon = item.icon}
    <button
      type="button"
      onclick={() => onSelect(item.key)}
      class="w-[calc(100%-16px)] flex items-center gap-[10px] h-[30px] pl-3 pr-[10px] mx-2 rounded-md text-[13px] text-left transition-colors
        {active === item.key
        ? 'bg-secondary text-foreground font-medium'
        : 'text-muted-fg-2 hover:bg-hover'}"
    >
      <span
        class="flex {active === item.key
          ? 'text-foreground'
          : 'text-muted-foreground'}"
      >
        <Icon size={13} />
      </span>
      <span class="flex-1 overflow-hidden text-ellipsis whitespace-nowrap">
        {item.label}
      </span>
    </button>
  {/each}
</nav>
