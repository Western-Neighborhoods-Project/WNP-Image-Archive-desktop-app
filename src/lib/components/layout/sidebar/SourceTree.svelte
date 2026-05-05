<script lang="ts">
  // Recursive tree renderer for the sidebar's source-directory section.
  // Each node represents either a source root or a subfolder within one.
  // Clicking a node sets the active filter (source_directory_id +
  // relative_dir); descendants are reachable by expanding the chevron.
  //
  // We keep the expansion state in a Map keyed by "<sourceId>:<relativeDir>"
  // so it survives sidebar re-renders. The map lives in the parent
  // (Sidebar.svelte) so flipping between source roots remembers which
  // subfolders were open.
  import { ChevronRight, ChevronDown, Folder, FolderTree } from "@lucide/svelte";
  import type { SourceTreeNode } from "$lib/commands/sources";
  import Self from "./SourceTree.svelte";

  type Props = {
    /** Either the source root (depth 0) or a sub-folder. */
    node: SourceTreeNode;
    /** Pixels of left padding to show nesting. Source roots use 12px;
     *  each child level adds 12px. */
    depth: number;
    /** "<sourceId>:<relativeDir>" of the active node, or null. */
    activeKey: string | null;
    /** Set of expanded keys (same key shape). */
    expanded: Set<string>;
    /** Toggle expansion for a key (parent owns the Set so it persists). */
    onToggle: (key: string) => void;
    /** Apply this node as the active filter. */
    onSelect: (sourceId: number, relativeDir: string) => void;
    /** Optional override for the node's label — used for the source-root
     *  level, where we want to show the user-supplied label rather than
     *  the basename of the path that the backend tree walker computes. */
    labelOverride?: string;
    /** Marks this row as a source root. Applies different styling. */
    isSourceRoot?: boolean;
  };

  let {
    node,
    depth,
    activeKey,
    expanded,
    onToggle,
    onSelect,
    labelOverride,
    isSourceRoot = false,
  }: Props = $props();

  let key = $derived(`${node.sourceDirectoryId}:${node.relativeDir}`);
  let isExpanded = $derived(expanded.has(key));
  let isActive = $derived(activeKey === key);
  let hasChildren = $derived(node.children.length > 0);

  function handleClick() {
    onSelect(node.sourceDirectoryId, node.relativeDir);
  }

  function handleToggleClick(e: MouseEvent) {
    e.stopPropagation();
    onToggle(key);
  }
</script>

<div
  role="presentation"
  class="w-[calc(100%-16px)] mx-2 flex items-center rounded-md transition-colors {isActive
    ? 'bg-secondary'
    : 'hover:bg-hover'}"
>
  <!-- Chevron toggle for children. Always reserve the slot so labels align. -->
  {#if hasChildren}
    <button
      type="button"
      onclick={handleToggleClick}
      class="flex h-[30px] w-[18px] flex-shrink-0 items-center justify-center text-muted-foreground hover:text-foreground"
      title={isExpanded ? "Collapse" : "Expand"}
      aria-label={isExpanded ? "Collapse" : "Expand"}
      style="margin-left: {depth * 12}px;"
    >
      {#if isExpanded}
        <ChevronDown size={12} />
      {:else}
        <ChevronRight size={12} />
      {/if}
    </button>
  {:else}
    <span
      class="flex h-[30px] w-[18px] flex-shrink-0"
      style="margin-left: {depth * 12}px;"
      aria-hidden="true"
    ></span>
  {/if}

  <button
    type="button"
    onclick={handleClick}
    class="flex flex-1 items-center gap-[10px] h-[30px] pl-1 pr-[10px] text-[13px] text-left {isActive
      ? 'text-foreground font-medium'
      : 'text-muted-fg-2'}"
  >
    <span class={isActive ? "text-foreground" : "text-muted-foreground"}>
      {#if isSourceRoot}
        <FolderTree size={14} />
      {:else}
        <Folder size={14} />
      {/if}
    </span>
    <span class="flex-1 overflow-hidden text-ellipsis whitespace-nowrap">
      {labelOverride ?? node.label}
    </span>
    <span class="text-[11px] text-muted-foreground tabular-nums">
      {node.imageCount.toLocaleString()}
    </span>
  </button>
</div>

{#if hasChildren && isExpanded}
  {#each node.children as child (`${child.sourceDirectoryId}:${child.relativeDir}`)}
    <Self
      node={child}
      depth={depth + 1}
      {activeKey}
      {expanded}
      {onToggle}
      {onSelect}
    />
  {/each}
{/if}
