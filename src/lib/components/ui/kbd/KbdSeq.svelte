<script lang="ts">
  import Kbd from "./Kbd.svelte";

  interface Props {
    keys: string[];
    dim?: boolean;
  }

  let { keys, dim = false }: Props = $props();
</script>

<span class="inline-flex gap-[3px]">
  {#each keys as key}
    <Kbd {dim}>{key}</Kbd>
  {/each}
</span>
