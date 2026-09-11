<script lang="ts">
  import { updateClipContent } from "$lib/api/tauri";
  import { mdToHtml, htmlToMd } from "$lib/utils/markdown";
  import type { ClipItem } from "$lib/api/tauri";
  import { clipsStore } from "$lib/stores/clips.svelte";

  interface Props {
    clip: ClipItem | null;
    open?: boolean;
    onclose?: () => void;
  }
  let { clip, open = $bindable(false), onclose }: Props = $props();

  let editorEl: HTMLDivElement;
  let saveTimeout: ReturnType<typeof setTimeout>;

  $effect(() => {
    if (open && clip && editorEl) {
      editorEl.innerHTML = mdToHtml(clip.content);
    }
  });

  function exec(command: string) {
    document.execCommand(command, false);
    editorEl.focus();
    scheduleSave();
  }

  function scheduleSave() {
    clearTimeout(saveTimeout);
    saveTimeout = setTimeout(async () => {
      if (!clip) return;
      const md = htmlToMd(editorEl.innerHTML);
      const updated = await updateClipContent(clip.id, md);
      clipsStore.updateItem(updated);
    }, 600);
  }

  function close() {
    clearTimeout(saveTimeout);
    if (clip && editorEl) {
      updateClipContent(clip.id, htmlToMd(editorEl.innerHTML)).then((updated) =>
        clipsStore.updateItem(updated)
      );
    }
    open = false;
    onclose?.();
  }

  export function requestClose() {
    close();
  }
</script>

{#if open && clip}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 bg-black/40 z-40 flex items-end justify-stretch"
    onclick={close}
  >
    <div
      class="flex-1 bg-[#1c1c1e]/95 backdrop-blur-2xl border-t border-white/10
             rounded-t-2xl p-5 animate-slide-in z-50 max-h-[80%] overflow-y-auto"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
    >
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-sm font-semibold text-white/90">Note</h2>
        <button
          class="text-white/40 hover:text-white/70 transition-colors"
          onclick={close}
        >✕</button>
      </div>

      <div class="flex items-center gap-1 mb-3 pb-3 border-b border-white/8">
        {#each [["bold", "B"], ["italic", "I"], ["insertUnorderedList", "•"], ["insertOrderedList", "1."], ["formatBlock", "H1", "<h1>"], ["formatBlock", "H2", "<h2>"]] as [cmd, label, value]}
          <button
            class="w-7 h-7 rounded-md text-xs font-semibold text-white/60 hover:bg-white/10 hover:text-white/90 transition-colors"
            onmousedown={(e) => e.preventDefault()}
            onclick={() => value ? (document.execCommand(cmd, false, value), scheduleSave()) : exec(cmd)}
          >{label}</button>
        {/each}
      </div>

      <div
        bind:this={editorEl}
        contenteditable="true"
        class="min-h-[200px] text-sm text-white/90 leading-relaxed outline-none
               [&_strong]:font-semibold [&_em]:italic [&_ul]:list-disc [&_ul]:pl-5
               [&_ol]:list-decimal [&_ol]:pl-5 [&_h1]:text-lg [&_h1]:font-semibold
               [&_h2]:text-base [&_h2]:font-semibold"
        oninput={scheduleSave}
      ></div>
    </div>
  </div>
{/if}
