<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import type { ClipItem } from "$lib/api/tauri";
  import { foldersStore } from "$lib/stores/folders.svelte";
  import { clipsStore } from "$lib/stores/clips.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import SearchBar from "$lib/components/SearchBar.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import ClipGrid from "$lib/components/ClipGrid.svelte";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";
  import HelpPanel from "$lib/components/HelpPanel.svelte";
  import Toast from "$lib/components/Toast.svelte";
  import { hideMainWindow, deleteClip } from "$lib/api/tauri";

  let showSettings = $state(false);
  let showHelp = $state(false);
  let searchQuery = $state("");
  let toast: ReturnType<typeof Toast> | null = $state(null);
  let searchDebounce: ReturnType<typeof setTimeout>;
  let appVisible = $state(false);

  async function onSearch(q: string) {
    searchQuery = q;
    clearTimeout(searchDebounce);
    searchDebounce = setTimeout(async () => {
      await clipsStore.load(
        q ? undefined : foldersStore.activeId,
        q || undefined
      );
    }, 150);
  }

  onMount(async () => {
    // Load initial data
    await Promise.all([
      foldersStore.load(),
      clipsStore.load(1),
      settingsStore.load(),
    ]);

    // Trigger spring-in animation
    setTimeout(() => { appVisible = true; }, 10);

    // Listen for new clips from clipboard watcher
    await listen<ClipItem>("clip:new", ({ payload }) => {
      if (
        !searchQuery &&
        (foldersStore.activeId === 1 || foldersStore.activeId === null)
      ) {
        clipsStore.prependItem(payload);
      }
    });

    // Listen for folder shortcuts
    await listen<{ folderName: string; clip: ClipItem; source: string }>("folder:saved", ({ payload }) => {
      const label = payload.source === "selection" ? "selection" : "clipboard";
      (toast as unknown as { show: (msg: string, type: string) => void })?.show(
        `${payload.folderName} ← ${label}`, "success"
      );
    });

    // Listen for cleanup events
    await listen<number>("cleanup:done", ({ payload }) => {
      if (payload > 0) {
        (toast as unknown as { show: (msg: string, type: string) => void })?.show(`Auto-cleaned ${payload} old clips`, "info");
      }
    });

    // Listen for update progress/errors
    await listen<string>("update:progress", ({ payload }) => {
      (toast as unknown as { show: (msg: string, type: string, duration: number) => void })?.show(`⬆ ${payload}`, "info", 8000);
    });
    await listen<string>("update:error", ({ payload }) => {
      (toast as unknown as { show: (msg: string, type: string, duration: number) => void })?.show(`Update failed: ${payload}`, "error", 6000);
    });
    // Linux deb/rpm installs are package-manager owned — we point at the release
    // page rather than replacing files behind the package manager's back.
    await listen<string>("update:manual", ({ payload }) => {
      (toast as unknown as { show: (msg: string, type: string, duration: number) => void })?.show(payload, "info", 10000);
    });

    // Hide window when it loses focus; reload clips when it gains focus
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    win.onFocusChanged(({ payload: focused }) => {
      if (focused) {
        // Reload to surface any clips captured while the window was hidden
        clipsStore.load(foldersStore.activeId ?? 1);
      } else if (!showSettings && !showHelp) {
        // Small delay to allow click actions to complete
        setTimeout(() => hideMainWindow(), 200);
      }
    });

    // Keyboard shortcuts
    document.addEventListener("keydown", handleGlobalKeydown);
    return () => document.removeEventListener("keydown", handleGlobalKeydown);
  });

  function handleGlobalKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (showHelp) {
        showHelp = false;
      } else if (showSettings) {
        showSettings = false;
      } else {
        hideMainWindow();
      }
    }
    if ((e.key === "f" && e.metaKey) || e.key === "/") {
      e.preventDefault();
      document.querySelector<HTMLInputElement>('[data-search]')?.focus();
    }
    if (e.key === "Backspace" || e.key === "Delete") {
      const id = clipsStore.hoveredId;
      if (id !== null && !showSettings && !showHelp) {
        e.preventDefault();
        deleteClip(id).then(() => clipsStore.removeItem(id)).catch(() => {});
      }
    }
  }
</script>

<!-- Window root with glass effect + spring-in animation -->
<div
  class="w-full h-full flex flex-col rounded-2xl overflow-hidden
         bg-[rgba(20,20,22,0.88)] backdrop-blur-xl border border-white/8
         shadow-[0_24px_64px_rgba(0,0,0,0.6)]
         transition-all
         {appVisible ? 'animate-spring-in' : 'opacity-0 scale-[0.96]'}"
>
  <!-- Search bar at top -->
  <SearchBar bind:value={searchQuery} onchange={onSearch} />

  <!-- Main content -->
  <div class="flex flex-1 min-h-0">
    <Sidebar onSettingsClick={() => (showSettings = true)} onHelpClick={() => (showHelp = true)} />
    <main class="flex-1 min-w-0 flex flex-col">
      <ClipGrid
        searchQuery={searchQuery}
        folderName={foldersStore.active?.name ?? ""}
      />
    </main>
  </div>
</div>

<!-- Overlays -->
<HelpPanel bind:open={showHelp} />
<SettingsPanel bind:open={showSettings} />
<Toast bind:this={toast} />
