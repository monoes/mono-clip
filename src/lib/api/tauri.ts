import { invoke } from "@tauri-apps/api/core";

export interface Folder {
  id: number;
  name: string;
  icon: string;
  color: string;
  globalShortcut: string | null;
  position: number;
  createdAt: string;
  updatedAt: string;
}

export interface ClipItem {
  id: number;
  content: string;
  contentType: "text" | "url" | "email" | "color" | "code" | "image" | "note";
  preview: string;
  folderId: number;
  isPinned: boolean;
  isDeleted: boolean;
  sourceApp: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface Settings {
  id: number;
  masterShortcut: string;
  autoCleanEnabled: boolean;
  autoCleanDays: number;
  maxHistoryItems: number;
  pasteOnClick: boolean;
  theme: "light" | "dark" | "system";
  launchAtLogin: boolean;
  ignoredApps: string;
  updatedAt: string;
}

export interface AppStats {
  totalClips: number;
  foldersCount: number;
  pinnedCount: number;
}

export interface SettingsPatch {
  masterShortcut?: string;
  autoCleanEnabled?: boolean;
  autoCleanDays?: number;
  maxHistoryItems?: number;
  pasteOnClick?: boolean;
  theme?: "light" | "dark" | "system";
  launchAtLogin?: boolean;
  ignoredApps?: string;
}

// ─── Folder Commands ──────────────────────────────────────────────────────────
export const getFolders = () => invoke<Folder[]>("get_folders");

export const createFolder = (name: string, icon: string, color: string, shortcut?: string) =>
  invoke<Folder>("create_folder", { name, icon, color, shortcut });

export const updateFolder = (
  id: number,
  name?: string,
  icon?: string,
  color?: string,
  shortcut?: string,
  clearShortcut?: boolean
) => invoke<Folder>("update_folder", { id, name, icon, color, shortcut, clearShortcut });

export const deleteFolder = (id: number) => invoke<void>("delete_folder", { id });

export const reorderFolders = (ids: number[]) => invoke<void>("reorder_folders", { ids });

// ─── Clip Commands ─────────────────────────────────────────────────────────────
export const getClips = (
  folderId?: number,
  search?: string,
  limit?: number,
  offset?: number
) => invoke<ClipItem[]>("get_clips", { folderId, search, limit, offset });

export const getClip = (id: number) => invoke<ClipItem>("get_clip", { id });

export const pinClip = (id: number) => invoke<void>("pin_clip", { id });
export const unpinClip = (id: number) => invoke<void>("unpin_clip", { id });
export const deleteClip = (id: number) => invoke<void>("delete_clip", { id });
export const restoreClip = (id: number) => invoke<void>("restore_clip", { id });
export const hardDeleteClip = (id: number) => invoke<void>("hard_delete_clip", { id });
export const moveClip = (id: number, folderId: number) => invoke<void>("move_clip", { id, folderId });

export const copyToClipboard = (id: number) => invoke<void>("copy_to_clipboard", { id });

export const saveCurrentClipboardToFolder = (folderId: number) =>
  invoke<ClipItem>("save_current_clipboard_to_folder", { folderId });

export const createBlankClip = (folderId: number) => invoke<ClipItem>("create_blank_clip", { folderId });

export const updateClipContent = (id: number, content: string) =>
  invoke<ClipItem>("update_clip_content", { id, content });

export const copyClipStyled = (html: string, plainText: string) =>
  invoke<void>("copy_clip_styled", { html, plainText });

// ─── Settings Commands ─────────────────────────────────────────────────────────
export const getSettings = () => invoke<Settings>("get_settings");
export const updateSettings = (patch: SettingsPatch) => invoke<Settings>("update_settings", { patch });

// ─── Utility Commands ─────────────────────────────────────────────────────────
export const getStats = () => invoke<AppStats>("get_stats");
export const runAutoCleanup = () => invoke<number>("run_auto_cleanup");
export const showMainWindow = () => invoke<void>("show_main_window");
export const hideMainWindow = () => invoke<void>("hide_main_window");
export const toggleMainWindow = () => invoke<void>("toggle_main_window");
export const installCli = () => invoke<string>("install_cli");
export const clearAllClips = () => invoke<number>("clear_all_clips");
export const clearFolderClips = (folderId: number) => invoke<number>("clear_folder_clips", { folderId });
export const exportFolderClips = (folderId: number) => invoke<string>("export_folder_clips", { folderId });
export const checkAccessibility = () => invoke<boolean>("check_accessibility");
export const openAccessibilitySettings = () => invoke<void>("open_accessibility_settings");

// ─── Backup Commands ───────────────────────────────────────────────────────────
export async function exportData(): Promise<string> {
  return invoke("export_data");
}

export async function importData(json: string): Promise<{ foldersCreated: number; foldersMerged: number; clipsImported: number }> {
  return invoke("import_data", { json });
}
