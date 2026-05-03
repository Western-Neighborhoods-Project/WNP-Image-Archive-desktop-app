<script lang="ts">
  import X from "@lucide/svelte/icons/x";

  interface Props {
    field: string;
    value: string;
    onRemove?: () => void;
  }

  let { field, value, onRemove }: Props = $props();
</script>

<span
  class="inline-flex items-center h-[26px] rounded-[4px] overflow-hidden border border-border text-xs bg-background"
>
  <span
    class="px-2 text-muted-foreground bg-sidebar-bg border-r border-border h-full flex items-center"
  >
    {field}
  </span>
  <span class="px-2 text-foreground font-medium flex items-center h-full">
    {value}
  </span>
  <button
    type="button"
    class="pl-[2px] pr-[6px] text-muted-foreground flex items-center h-full hover:text-foreground"
    onclick={onRemove}
    aria-label="Remove {field} filter"
  >
    <X size={11} />
  </button>
</span>
