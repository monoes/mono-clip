<script lang="ts">
  import { foldersStore } from "$lib/stores/folders.svelte";
  import { clipsStore } from "$lib/stores/clips.svelte";
  import FolderModal from "./FolderModal.svelte";
  import type { Folder } from "$lib/api/tauri";
  import { deleteFolder, moveClip, clearFolderClips, exportFolderClips } from "$lib/api/tauri";

  interface Props {
    onSettingsClick?: () => void;
    onHelpClick?: () => void;
    onNewCard?: () => void;
  }
  let { onSettingsClick, onHelpClick, onNewCard }: Props = $props();

  let showNewFolder = $state(false);
  let editingFolder = $state<Folder | null>(null);
  let showEditFolder = $state(false);
  let contextMenu = $state<{ folder: Folder; x: number; y: number } | null>(null);
  let dragOverId = $state<number | null>(null);

  async function selectFolder(id: number) {
    foldersStore.setActive(id);
    await clipsStore.load(id, clipsStore.searchQuery || undefined);
  }

  function openContextMenu(e: MouseEvent, folder: Folder) {
    e.preventDefault();
    contextMenu = { folder, x: e.clientX, y: e.clientY };
  }

  async function handleDeleteFolder() {
    if (!contextMenu) return;
    const { folder } = contextMenu;
    contextMenu = null;
    if (folder.id === 1) return; // Can't delete Inbox
    try {
      await deleteFolder(folder.id);
      foldersStore.removeFolder(folder.id);
      if (foldersStore.activeId === folder.id) {
        foldersStore.setActive(1);
        await clipsStore.load(1);
      }
    } catch (e) {
      console.error(e);
    }
  }

  function startEditFolder() {
    if (!contextMenu) return;
    editingFolder = contextMenu.folder;
    showEditFolder = true;
    contextMenu = null;
  }

  function closeEditFolder() {
    showEditFolder = false;
    editingFolder = null;
  }

  // ─── Drag & Drop ───────────────────────────────────────────────────────────
  function handleDragOver(e: DragEvent, folderId: number) {
    e.preventDefault();
    e.dataTransfer!.dropEffect = "move";
    dragOverId = folderId;
  }

  function handleDragLeave() {
    dragOverId = null;
  }

  async function handleDrop(e: DragEvent, folderId: number) {
    e.preventDefault();
    dragOverId = null;
    const clipId = Number(e.dataTransfer?.getData("text/plain"));
    if (!clipId) return;
    try {
      await moveClip(clipId, folderId);
      // Remove from current view since it moved to another folder
      clipsStore.removeItem(clipId);
    } catch (err) {
      console.error("Move failed:", err);
    }
  }

  // ─── Export folder ─────────────────────────────────────────────────────────
  async function handleExportFolder() {
    if (!contextMenu) return;
    const { folder } = contextMenu;
    contextMenu = null;
    try {
      const text = await exportFolderClips(folder.id);
      const blob = new Blob([text], { type: "text/plain" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${folder.name.replace(/[^a-zA-Z0-9]/g, "_")}_clips.txt`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error("Export failed:", err);
    }
  }

  // ─── Clear folder ─────────────────────────────────────────────────────────
  let clearConfirm = $state(false);

  async function handleClearFolder() {
    if (!contextMenu) return;
    if (!clearConfirm) {
      clearConfirm = true;
      setTimeout(() => { clearConfirm = false; }, 3000);
      return;
    }
    const { folder } = contextMenu;
    contextMenu = null;
    clearConfirm = false;
    try {
      await clearFolderClips(folder.id);
      // Reload if we're viewing this folder
      if (foldersStore.activeId === folder.id) {
        await clipsStore.load(folder.id);
      }
    } catch (err) {
      console.error("Clear failed:", err);
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<aside class="w-48 flex-shrink-0 flex flex-col border-r border-white/5 py-2">
  <!-- Folder list -->
  <nav class="flex-1 overflow-y-auto px-2 space-y-0.5">
    {#each foldersStore.items as folder (folder.id)}
      <button
        class="w-full flex items-center gap-2.5 px-2.5 py-2 rounded-lg text-left text-sm
               transition-all duration-100 group
               {dragOverId === folder.id
                 ? 'bg-accent/20 border border-accent/40 text-white/95'
                 : foldersStore.activeId === folder.id
                   ? 'bg-white/10 text-white/95 border border-transparent'
                   : 'text-white/55 hover:text-white/80 hover:bg-white/5 border border-transparent'}"
        onclick={() => selectFolder(folder.id)}
        oncontextmenu={(e) => openContextMenu(e, folder)}
        ondragover={(e) => handleDragOver(e, folder.id)}
        ondragleave={handleDragLeave}
        ondrop={(e) => handleDrop(e, folder.id)}
      >
        <!-- Color dot -->
        <span
          class="w-2 h-2 rounded-full flex-shrink-0 transition-transform duration-150
                 {foldersStore.activeId === folder.id ? 'scale-125' : ''}"
          style="background-color: {folder.color};"
        ></span>
        <span class="truncate flex-1">{folder.icon} {folder.name}</span>
      </button>
    {/each}
  </nav>

  <!-- Bottom actions -->
  <div class="px-2 pt-2 border-t border-white/5 space-y-0.5">
    <button
      class="w-full flex items-center gap-2 px-2.5 py-2 rounded-lg text-sm
             text-white/40 hover:text-white/70 hover:bg-white/5 transition-colors"
      onclick={onNewCard}
      title="New blank card"
    >
      <span class="text-base">✏️</span>
      <span>New Card</span>
    </button>
    <button
      class="w-full flex items-center gap-2 px-2.5 py-2 rounded-lg text-sm
             text-white/40 hover:text-white/70 hover:bg-white/5 transition-colors"
      onclick={() => { showNewFolder = true; }}
    >
      <span class="text-base">+</span>
      <span>New Folder</span>
    </button>
    <button
      class="w-full flex items-center gap-2 px-2.5 py-2 rounded-lg text-sm
             text-white/40 hover:text-white/70 hover:bg-white/5 transition-colors"
      onclick={onHelpClick}
    >
      <span>?</span>
      <span>Help</span>
    </button>
    <button
      class="w-full flex items-center gap-2 px-2.5 py-2 rounded-lg text-sm
             text-white/40 hover:text-white/70 hover:bg-white/5 transition-colors"
      onclick={onSettingsClick}
    >
      <span>⚙</span>
      <span>Settings</span>
    </button>
  </div>
</aside>

<!-- Context menu -->
{#if contextMenu}
  <div
    class="fixed z-50 bg-[#2c2c2e]/95 backdrop-blur-xl rounded-xl shadow-2xl
           border border-white/10 overflow-hidden py-1 w-44"
    style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
  >
    {#if contextMenu.folder.id !== 1}
      <button
        class="w-full px-3 py-2 text-sm text-left text-white/80 hover:bg-white/10
               flex items-center gap-2"
        onclick={startEditFolder}
      >
        ✏️ Edit
      </button>
    {/if}
    <button
      class="w-full px-3 py-2 text-sm text-left text-white/80 hover:bg-white/10
             flex items-center gap-2"
      onclick={handleExportFolder}
    >
      📄 Export to File
    </button>
    <button
      class="w-full px-3 py-2 text-sm text-left hover:bg-red-500/10
             flex items-center gap-2
             {clearConfirm ? 'text-red-400 font-medium' : 'text-white/80'}"
      onclick={handleClearFolder}
    >
      🧹 {clearConfirm ? 'Tap again to confirm' : 'Clear All Clips'}
    </button>
    {#if contextMenu.folder.id !== 1}
      <div class="border-t border-white/5 my-1"></div>
      <button
        class="w-full px-3 py-2 text-sm text-left text-red-400 hover:bg-red-500/10
               flex items-center gap-2"
        onclick={handleDeleteFolder}
      >
        🗑️ Delete Folder
      </button>
    {/if}
  </div>
  <!-- Dismiss backdrop -->
  <div
    class="fixed inset-0 z-40"
    onclick={() => { contextMenu = null; clearConfirm = false; }}
  ></div>
{/if}

<!-- Modals -->
<FolderModal bind:open={showNewFolder} />

<FolderModal
  open={showEditFolder}
  editFolder={editingFolder}
  onclose={closeEditFolder}
/>
