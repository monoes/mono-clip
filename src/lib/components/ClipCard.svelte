<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { ClipItem } from "$lib/api/tauri";
  import { copyToClipboard, deleteClip, pinClip, unpinClip, copyClipStyled } from "$lib/api/tauri";
  import { relativeTime } from "$lib/utils/time";
  import { clipsStore } from "$lib/stores/clips.svelte";
  import { mdToHtml, mdToPlainText } from "$lib/utils/markdown";

  const penEditableTypes = ["text", "url", "code", "note"];

  interface Props {
    clip: ClipItem;
    index?: number;
    onCopy?: (id: number) => void;
    onEditClip?: (clip: ClipItem) => void;
  }

  let { clip, index = 0, onCopy, onEditClip }: Props = $props();

  let isHovered = $state(false);
  let isFlashing = $derived(clipsStore.flashingId === clip.id);
  let isMenuOpen = $derived(clipsStore.contextMenuId === clip.id);
  let menuX = $state(0);
  let menuY = $state(0);

  // For image clips, convert the stored file path to a WebView-accessible URL
  let imageSrc = $derived(
    clip.contentType === "image" ? convertFileSrc(clip.content) : ""
  );

async function handleCopy(e: MouseEvent) {
    e.stopPropagation();
    try {
      await copyToClipboard(clip.id);
      clipsStore.setFlashing(clip.id);
      onCopy?.(clip.id);
    } catch (err) {
      console.error("Copy failed:", err);
    }
  }

  async function handlePin(e: MouseEvent) {
    e.stopPropagation();
    try {
      if (clip.isPinned) {
        await unpinClip(clip.id);
        clipsStore.updateItem({ ...clip, isPinned: false });
      } else {
        await pinClip(clip.id);
        clipsStore.updateItem({ ...clip, isPinned: true });
      }
    } catch (err) {
      console.error("Pin failed:", err);
    }
  }

  async function handleDelete(e: MouseEvent) {
    e.stopPropagation();
    try {
      await deleteClip(clip.id);
      clipsStore.removeItem(clip.id);
    } catch (err) {
      console.error("Delete failed:", err);
    }
  }

  function handleEdit(e: MouseEvent) {
    e.stopPropagation();
    onEditClip?.(clip);
  }

  async function handleCopyStyled(e: MouseEvent) {
    e.stopPropagation();
    await copyClipStyled(mdToHtml(clip.content), mdToPlainText(clip.content));
    clipsStore.setFlashing(clip.id);
  }

  function openContextMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    menuX = e.clientX;
    menuY = e.clientY;
    clipsStore.contextMenuId = clip.id;
  }

  function closeContextMenu() {
    clipsStore.contextMenuId = null;
  }

  function menuAction(handler: (e: MouseEvent) => void) {
    return (e: MouseEvent) => {
      handler(e);
      closeContextMenu();
    };
  }

  const typeIcon: Record<string, string> = {
    url: "🔗",
    email: "📧",
    color: "🎨",
    code: "</>",
    image: "🖼",
    text: "",
  };

  const animDelay = `${Math.min(index * 30, 300)}ms`;

  function handleDragStart(e: DragEvent) {
    e.dataTransfer?.setData("text/plain", String(clip.id));
    e.dataTransfer!.effectAllowed = "move";
  }

</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="group relative rounded-xl p-3 cursor-pointer transition-all duration-150
         border border-transparent
         {isFlashing
           ? 'animate-copy-flash bg-accent/20 border-accent/30'
           : 'bg-white/5 hover:bg-white/10 hover:border-white/10'}
         opacity-0"
  style="animation: fade-up 200ms ease-out {animDelay} forwards;"
  draggable="true"
  ondragstart={handleDragStart}
  onmouseenter={() => { isHovered = true; clipsStore.hoveredId = clip.id; }}
  onmouseleave={() => { isHovered = false; if (clipsStore.hoveredId === clip.id) clipsStore.hoveredId = null; }}
  onclick={handleCopy}
  oncontextmenu={openContextMenu}
  role="button"
  tabindex="0"
  onkeydown={(e) => e.key === "Enter" && handleCopy(e as unknown as MouseEvent)}
>
  <!-- Pin indicator -->
  {#if clip.isPinned}
    <div class="absolute top-2 right-2 text-xs opacity-60">📌</div>
  {/if}

  <!-- Content type icon (not shown for image since the thumbnail replaces it) -->
  {#if typeIcon[clip.contentType] && clip.contentType !== "image"}
    <span class="text-xs opacity-40 mb-1 block font-mono">
      {typeIcon[clip.contentType]}
    </span>
  {/if}

  <!-- Image thumbnail -->
  {#if clip.contentType === "image"}
    <div class="w-full rounded-lg overflow-hidden mb-2 bg-white/5">
      <img
        src={imageSrc}
        alt=""
        class="w-full object-contain max-h-40"
        loading="lazy"
      />
    </div>
    <p class="text-xs text-white/40">{clip.preview}</p>

  <!-- Color swatch -->
  {:else if clip.contentType === "color"}
    <p class="text-sm leading-relaxed line-clamp-4 selectable text-white/85">
      {clip.preview || clip.content}
    </p>
    <div
      class="w-full h-6 rounded mt-2 border border-white/10"
      style="background-color: {clip.content};"
    ></div>

  <!-- Text / URL / code / email -->
  {:else}
    <p
      class="text-sm leading-relaxed line-clamp-4 selectable
             {clip.contentType === 'code' ? 'font-mono text-xs text-green-300/80' : 'text-white/85'}
             {clip.contentType === 'url' ? 'text-blue-300/80 underline-offset-2' : ''}"
    >
      {clip.preview || clip.content}
    </p>
  {/if}

  <!-- Footer -->
  <div class="flex items-center justify-between mt-2">
    <span class="text-xs text-white/30">{relativeTime(clip.updatedAt)}</span>

    <!-- Hover actions -->
    <div
      class="flex items-center gap-1 transition-opacity duration-100
             {isHovered ? 'opacity-100' : 'opacity-0'}"
    >
      {#if penEditableTypes.includes(clip.contentType)}
        <button
          class="p-1 rounded-md hover:bg-white/15 text-white/50 hover:text-white/90 text-xs transition-colors"
          onclick={handleEdit}
          title="Edit"
        >✏️</button>
      {/if}
      <button
        class="p-1 rounded-md hover:bg-white/15 text-white/50 hover:text-white/90 text-xs transition-colors"
        onclick={handlePin}
        title={clip.isPinned ? "Unpin" : "Pin"}
      >
        {clip.isPinned ? "📌" : "📍"}
      </button>
      <button
        class="p-1 rounded-md hover:bg-red-500/20 text-white/50 hover:text-red-400 text-xs transition-colors"
        onclick={handleDelete}
        title="Delete"
      >
        ✕
      </button>
    </div>
  </div>
</div>

<!-- Right-click context menu -->
{#if isMenuOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed z-50 bg-[#2c2c2e]/95 backdrop-blur-xl rounded-xl shadow-2xl
           border border-white/10 overflow-hidden py-1 w-44"
    style="left: {menuX}px; top: {menuY}px;"
  >
    <button
      class="w-full px-3 py-2 text-sm text-left text-white/80 hover:bg-white/10
             flex items-center gap-2"
      onclick={menuAction(handleCopy)}
    >
      📋 Copy
    </button>
    {#if penEditableTypes.includes(clip.contentType)}
      <button
        class="w-full px-3 py-2 text-sm text-left text-white/80 hover:bg-white/10
               flex items-center gap-2"
        onclick={menuAction(handleCopyStyled)}
      >
        🎨 Copy as Style
      </button>
      <button
        class="w-full px-3 py-2 text-sm text-left text-white/80 hover:bg-white/10
               flex items-center gap-2"
        onclick={menuAction(handleEdit)}
      >
        ✏️ Edit
      </button>
    {/if}
    <button
      class="w-full px-3 py-2 text-sm text-left text-white/80 hover:bg-white/10
             flex items-center gap-2"
      onclick={menuAction(handlePin)}
    >
      {clip.isPinned ? "📍 Unpin" : "📌 Pin"}
    </button>
    <div class="border-t border-white/5 my-1"></div>
    <button
      class="w-full px-3 py-2 text-sm text-left text-red-400 hover:bg-red-500/10
             flex items-center gap-2"
      onclick={menuAction(handleDelete)}
    >
      🗑️ Delete
    </button>
  </div>
  <!-- Dismiss backdrop -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-40"
    oncontextmenu={(e) => e.preventDefault()}
    onclick={closeContextMenu}
  ></div>
{/if}
