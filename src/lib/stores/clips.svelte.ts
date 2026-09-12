import type { ClipItem } from "$lib/api/tauri";
import { getClips } from "$lib/api/tauri";

class ClipsStore {
  items = $state<ClipItem[]>([]);
  isLoading = $state(false);
  error = $state<string | null>(null);
  searchQuery = $state("");
  activeFolder = $state<number | null>(1);
  flashingId = $state<number | null>(null);
  hoveredId = $state<number | null>(null);
  contextMenuId = $state<number | null>(null);

  async load(folderId?: number, search?: string) {
    this.isLoading = true;
    this.error = null;
    try {
      this.items = await getClips(folderId, search);
    } catch (e) {
      this.error = String(e);
    } finally {
      this.isLoading = false;
    }
  }

  prependItem(clip: ClipItem) {
    // Remove existing entry with same id if present
    this.items = [clip, ...this.items.filter((c) => c.id !== clip.id)];
  }

  removeItem(id: number) {
    this.items = this.items.filter((c) => c.id !== id);
  }

  updateItem(clip: ClipItem) {
    this.items = this.items.map((c) => (c.id === clip.id ? clip : c));
  }

  setFlashing(id: number | null) {
    this.flashingId = id;
    if (id !== null) {
      setTimeout(() => {
        if (this.flashingId === id) this.flashingId = null;
      }, 200);
    }
  }
}

export const clipsStore = new ClipsStore();
