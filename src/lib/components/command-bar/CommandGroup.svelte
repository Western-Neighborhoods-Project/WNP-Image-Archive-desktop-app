<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    children: Snippet;
  }

  let { title, children }: Props = $props();
</script>

<div class="py-1.5">
  <div
    class="px-[18px] pt-2 pb-1 text-[10.5px] font-semibold uppercase tracking-[0.5px] text-muted-foreground"
  >
    {title}
  </div>
  {@render children()}
</div>
