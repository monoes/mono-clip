<script lang="ts">
  interface Props {
    value?: string;
    onchange?: (value: string) => void;
  }
  let { value = $bindable(""), onchange }: Props = $props();

  let inputEl: HTMLInputElement;

  export function focus() {
    inputEl?.focus();
  }

  function handleInput(e: Event) {
    value = (e.target as HTMLInputElement).value;
    onchange?.(value);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      value = "";
      onchange?.("");
      inputEl?.blur();
    }
  }
</script>

<div data-tauri-drag-region class="relative flex items-center px-3 py-2.5 border-b border-white/5">
  <span class="text-white/30 mr-2.5 text-sm flex-shrink-0">⌕</span>
  <input
    bind:this={inputEl}
    data-search
    type="text"
    placeholder="Search clips..."
    class="flex-1 bg-transparent text-sm text-white/85 placeholder-white/30
           outline-none border-none focus:outline-none selection:bg-accent/30"
    value={value}
    oninput={handleInput}
    onkeydown={handleKeydown}
  />
  {#if value}
    <button
      class="text-white/30 hover:text-white/60 transition-colors text-xs px-1.5 flex-shrink-0"
      onclick={() => { value = ""; onchange?.(""); }}
    >
      ✕
    </button>
  {/if}
</div>
