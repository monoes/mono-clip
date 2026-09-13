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
  let dirty = false;
  let checklistBeforeInput: Set<Element> | null = null;

  $effect(() => {
    if (open && clip && editorEl) {
      editorEl.innerHTML = mdToHtml(clip.content);
      dirty = false;
    }
  });

  function exec(command: string) {
    document.execCommand(command, false);
    editorEl.focus();
    scheduleSave();
  }

  // Checklist items are a <ul><li> like any other list, just with a
  // checkbox prepended — there's no execCommand for this, so the list
  // structure itself is still built by insertUnorderedList/native Enter
  // handling and we only add the checkbox by hand afterward.
  //
  // This deliberately does NOT use window.getSelection() to find "the
  // current item" — after execCommand("insertUnorderedList") on an empty
  // editor, Chromium leaves the selection's container on the editor root,
  // not inside the newly created <li> (verified empirically). Diffing the
  // set of <li> elements before/after the mutation is what actually works
  // regardless of where the browser leaves the selection.
  function checkboxifyNewItems(before: Set<Element>) {
    editorEl.querySelectorAll("li").forEach((li) => {
      if (before.has(li) || li.querySelector(':scope > input[type="checkbox"]')) return;
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      const space = document.createTextNode(" ");
      li.insertBefore(checkbox, li.firstChild);
      checkbox.after(space);
      // Inserting nodes doesn't move the cursor, so without this, typing
      // right after clicking the button lands before the checkbox instead
      // of after it.
      const sel = window.getSelection();
      if (sel) {
        const range = document.createRange();
        range.setStart(space, 1);
        range.collapse(true);
        sel.removeAllRanges();
        sel.addRange(range);
      }
    });
  }

  function insertChecklist() {
    const before = new Set(editorEl.querySelectorAll("li"));
    const sel = window.getSelection();
    const node = sel?.anchorNode;
    const current = (node?.nodeType === Node.TEXT_NODE ? node.parentElement : node as Element | null)?.closest?.("li");
    // Native unordered-list formatting toggles an existing list off.
    if (current && editorEl.contains(current) && current.parentElement?.tagName === "UL") {
      before.delete(current);
    } else {
      document.execCommand("insertUnorderedList", false);
    }
    checkboxifyNewItems(before);
    editorEl.focus();
    scheduleSave();
  }

  // Pressing Enter at the end of a checklist item creates a new <li> via
  // the browser's native list handling, but that new item has no checkbox
  // yet — remember the old items and complete the mutation on input.
  function handleEditorKeydown(e: KeyboardEvent) {
    checklistBeforeInput = null;
    if (e.key !== "Enter" || e.shiftKey || e.isComposing) return;
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return;
    let node: Node | null = sel.getRangeAt(0).startContainer;
    if (node.nodeType === Node.TEXT_NODE) node = node.parentElement;
    const li = (node as Element | null)?.closest?.("li");
    if (!li?.querySelector(':scope > input[type="checkbox"]')) return;
    if (!li.textContent?.trim()) {
      // Let native Enter exit an empty list item after removing its control.
      li.replaceChildren(document.createElement("br"));
      const range = document.createRange();
      range.setStart(li, 0);
      range.collapse(true);
      sel.removeAllRanges();
      sel.addRange(range);
      return;
    }
    checklistBeforeInput = new Set(editorEl.querySelectorAll("li"));
  }

  function handleEditorInput() {
    // Input runs after the browser inserts the new list item, without timers.
    if (checklistBeforeInput) checkboxifyNewItems(checklistBeforeInput);
    checklistBeforeInput = null;
    scheduleSave();
  }

  function scheduleSave() {
    dirty = true;
    clearTimeout(saveTimeout);
    saveTimeout = setTimeout(async () => {
      if (!clip || !dirty) return;
      const md = htmlToMd(editorEl);
      try {
        const updated = await updateClipContent(clip.id, md);
        clipsStore.updateItem(updated);
        if (editorEl && htmlToMd(editorEl) === md) {
          dirty = false;
        }
      } catch (err) {
        console.error("Autosave failed:", err);
      }
    }, 600);
  }

  function close() {
    clearTimeout(saveTimeout);
    if (clip && editorEl && dirty) {
      const md = htmlToMd(editorEl);
      updateClipContent(clip.id, md)
        .then((updated) => {
          clipsStore.updateItem(updated);
          if (editorEl && htmlToMd(editorEl) === md) {
            dirty = false;
          }
        })
        .catch((err) => console.error("Save on close failed:", err));
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
        {#each [["bold", "B"], ["italic", "I"], ["insertUnorderedList", "•"], ["insertOrderedList", "1."]] as [cmd, label, value]}
          <button
            class="w-7 h-7 rounded-md text-xs font-semibold text-white/60 hover:bg-white/10 hover:text-white/90 transition-colors"
            onmousedown={(e) => e.preventDefault()}
            onclick={() => value ? (document.execCommand(cmd, false, value), scheduleSave()) : exec(cmd)}
          >{label}</button>
        {/each}
        <button
          class="w-7 h-7 rounded-md text-xs font-semibold text-white/60 hover:bg-white/10 hover:text-white/90 transition-colors"
          onmousedown={(e) => e.preventDefault()}
          onclick={insertChecklist}
          title="Checklist"
        >☑</button>
        {#each [["formatBlock", "H1", "<h1>"], ["formatBlock", "H2", "<h2>"]] as [cmd, label, value]}
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
               [&_h2]:text-base [&_h2]:font-semibold
               [&_li:has(>input[type=checkbox])]:list-none
               [&_li>input[type=checkbox]]:mr-1.5"
        oninput={handleEditorInput}
        onkeydown={handleEditorKeydown}
      ></div>
    </div>
  </div>
{/if}
