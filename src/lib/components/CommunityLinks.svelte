<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { communityCtas, socialLinks } from "$lib/data/community";

  function open(url: string, e: MouseEvent) {
    e.preventDefault();
    openUrl(url);
  }
</script>

<section class="mb-5">
  <h3 class="text-xs font-medium text-white/40 uppercase tracking-wider mb-3">Community</h3>

  <div class="flex flex-col gap-2 mb-3">
    {#each communityCtas as { label, url, icon }}
      <a
        href={url}
        onclick={(e) => open(url, e)}
        class="w-full py-2 rounded-lg text-sm font-medium text-center border transition-colors
               bg-[#6366f1]/10 border-[#6366f1]/25 text-[#a5b4fc] hover:bg-[#6366f1]/20"
      >{icon} {label}</a>
    {/each}
  </div>

  <div class="flex flex-wrap gap-2">
    {#each socialLinks as { label, url, icon }}
      <a
        href={url}
        onclick={(e) => open(url, e)}
        title={label}
        aria-label={label}
        class="w-8 h-8 flex items-center justify-center rounded-lg text-base
               bg-white/5 border border-white/8 hover:bg-white/10 transition-colors"
      >{icon}</a>
    {/each}
  </div>
</section>
