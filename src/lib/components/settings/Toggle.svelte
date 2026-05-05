<script lang="ts">
  interface Props {
    on: boolean;
    disabled?: boolean;
    onToggle?: () => void;
    ariaLabel?: string;
  }

  let { on, disabled = false, onToggle, ariaLabel }: Props = $props();
</script>

<button
  type="button"
  onclick={() => !disabled && onToggle?.()}
  class="relative w-[30px] h-[18px] rounded-full transition-colors flex-shrink-0
    {disabled
    ? 'bg-secondary opacity-50 cursor-not-allowed'
    : on
      ? 'bg-primary'
      : 'bg-border'}"
  aria-pressed={on}
  aria-label={ariaLabel}
  {disabled}
>
  <span
    class="absolute top-[2px] w-[14px] h-[14px] rounded-full bg-background transition-all"
    style="left: {on ? '14px' : '2px'}; box-shadow: 0 1px 2px rgba(0,0,0,.15);"
  ></span>
</button>
