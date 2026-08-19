<script lang="ts">
  import { createFolder, updateFolder } from "$lib/api/tauri";
  import type { Folder } from "$lib/api/tauri";
  import { foldersStore } from "$lib/stores/folders.svelte";
  import { isMac, formatShortcut } from "$lib/utils/platform";
  import ShortcutRecorder from "./ShortcutRecorder.svelte";

  interface Props {
    open?: boolean;
    editFolder?: Folder | null;
    onclose?: () => void;
  }
  let { open = $bindable(false), editFolder = null, onclose }: Props = $props();

  const EMOJIS = ["📋", "📁", "💻", "🔗", "📧", "📝", "⭐", "🔑", "💡", "🎨",
                  "📊", "🛒", "📱", "🎯", "🔖", "💬", "🏷️", "📌", "🗂️", "📂"];
  const COLORS = ["#6366f1", "#ec4899", "#f59e0b", "#10b981", "#3b82f6", "#ef4444",
                  "#8b5cf6", "#06b6d4", "#84cc16", "#f97316"];

  let name = $state(editFolder?.name ?? "");
  let icon = $state(editFolder?.icon ?? "📁");
  let color = $state(editFolder?.color ?? "#6366f1");
  let shortcut = $state(editFolder?.globalShortcut ?? "");
  let isLoading = $state(false);
  let error = $state("");

  $effect(() => {
    if (open) {
      name = editFolder?.name ?? "";
      icon = editFolder?.icon ?? "📁";
      color = editFolder?.color ?? "#6366f1";
      shortcut = editFolder?.globalShortcut ?? "";
      error = "";
    }
  });

  async function save() {
    if (!name.trim()) {
      error = "Folder name is required";
      return;
    }
    isLoading = true;
    error = "";
    try {
      if (editFolder) {
        const updated = await updateFolder(
          editFolder.id,
          name,
          icon,
          color,
          shortcut || undefined,
          !shortcut
        );
        foldersStore.updateFolder(updated);
      } else {
        const created = await createFolder(name, icon, color, shortcut || undefined);
        foldersStore.addFolder(created);
      }
      open = false;
      onclose?.();
    } catch (e) {
      error = String(e);
    } finally {
      isLoading = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      open = false;
      onclose?.();
    }
    if (e.key === "Enter" && (isMac ? e.metaKey : e.ctrlKey)) save();
  }
</script>

{#if open}
  <!-- Backdrop -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 bg-black/40 z-40 flex items-center justify-center"
    onclick={() => { open = false; onclose?.(); }}
  >
    <!-- Modal -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="bg-[#1c1c1e]/95 backdrop-blur-2xl rounded-2xl p-5 w-80 shadow-2xl
             border border-white/10 animate-spring-in z-50"
      onclick={(e) => e.stopPropagation()}
      onkeydown={handleKeydown}
      role="dialog"
      tabindex="-1"
    >
      <h2 class="text-sm font-semibold text-white/90 mb-4">
        {editFolder ? "Edit Folder" : "New Folder"}
      </h2>

      <!-- Name -->
      <div class="mb-4">
        <label class="text-xs text-white/40 mb-1.5 block">Name</label>
        <input
          type="text"
          bind:value={name}
          placeholder="e.g. Code Snippets"
          class="w-full bg-white/5 border border-white/10 rounded-lg px-3 py-2
                 text-sm text-white/90 placeholder-white/30 outline-none
                 focus:border-accent/50 transition-colors"
          autofocus
        />
      </div>

      <!-- Icon -->
      <div class="mb-4">
        <label class="text-xs text-white/40 mb-1.5 block">Icon</label>
        <div class="grid grid-cols-10 gap-1">
          {#each EMOJIS as emoji}
            <button
              class="aspect-square rounded-md text-base flex items-center justify-center
                     transition-colors {icon === emoji ? 'bg-accent/30 ring-1 ring-accent' : 'hover:bg-white/10'}"
              onclick={() => (icon = emoji)}
            >{emoji}</button>
          {/each}
        </div>
      </div>

      <!-- Color -->
      <div class="mb-4">
        <label class="text-xs text-white/40 mb-1.5 block">Color</label>
        <div class="flex gap-2">
          {#each COLORS as c}
            <button
              class="w-6 h-6 rounded-full transition-transform
                     {color === c ? 'scale-125 ring-2 ring-white/30' : 'hover:scale-110'}"
              style="background-color: {c};"
              onclick={() => (color = c)}
            ></button>
          {/each}
        </div>
      </div>

      <!-- Shortcut -->
      <div class="mb-5">
        <label class="text-xs text-white/40 mb-1.5 block">
          Global Shortcut <span class="opacity-50">(optional)</span>
        </label>
        <ShortcutRecorder bind:value={shortcut} />
        <p class="text-[10px] text-white/25 mt-1.5 leading-relaxed">
          Click the field then press your combo — e.g. {formatShortcut("⌘⌥1")}.<br/>
          When triggered, selected text is saved (falls back to clipboard).
        </p>
      </div>

      {#if error}
        <p class="text-xs text-red-400 mb-3">{error}</p>
      {/if}

      <!-- Actions -->
      <div class="flex gap-2">
        <button
          class="flex-1 py-2 rounded-lg text-sm text-white/50 hover:text-white/80
                 hover:bg-white/5 transition-colors"
          onclick={() => { open = false; onclose?.(); }}
        >Cancel</button>
        <button
          class="flex-1 py-2 rounded-lg text-sm font-medium bg-accent hover:bg-accent-dark
                 text-white transition-colors disabled:opacity-50"
          onclick={save}
          disabled={isLoading}
        >
          {isLoading ? "Saving…" : editFolder ? "Save" : "Create"}
        </button>
      </div>
    </div>
  </div>
{/if}
