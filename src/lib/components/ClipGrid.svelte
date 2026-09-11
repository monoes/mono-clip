<script lang="ts">
  import ClipCard from "./ClipCard.svelte";
  import EmptyState from "./EmptyState.svelte";
  import { clipsStore } from "$lib/stores/clips.svelte";
  import type { ClipItem } from "$lib/api/tauri";

  interface Props {
    searchQuery?: string;
    folderName?: string;
    onEditClip?: (clip: ClipItem) => void;
  }
  let { searchQuery = "", folderName = "", onEditClip }: Props = $props();

  function handleCopy(_id: number) {
    // Could show a toast here via event
  }
</script>

<div class="flex-1 overflow-y-auto px-3 py-2">
  {#if clipsStore.isLoading}
    <!-- Skeleton loading state -->
    <div class="grid grid-cols-2 gap-2">
      {#each Array(6) as _, i (i)}
        <div class="rounded-xl p-3 bg-white/5 animate-pulse" style="height: {60 + (i % 3) * 20}px;"></div>
      {/each}
    </div>
  {:else if clipsStore.error}
    <div class="flex flex-col items-center justify-center h-full gap-2 text-red-400/70">
      <span class="text-2xl">⚠️</span>
      <p class="text-sm">Something went wrong</p>
      <p class="text-xs opacity-60">{clipsStore.error}</p>
    </div>
  {:else if clipsStore.items.length === 0}
    <EmptyState query={searchQuery} {folderName} />
  {:else}
    <!-- 2-column grid: row-first order (left→right, top→bottom), newest at top -->
    <div class="grid grid-cols-2 gap-2">
      {#each clipsStore.items as clip, i (clip.id)}
        <ClipCard {clip} index={i} onCopy={handleCopy} onEditClip={onEditClip} />
      {/each}
    </div>
  {/if}
</div>
