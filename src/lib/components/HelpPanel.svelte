<script lang="ts">
  import { formatShortcut } from "$lib/utils/platform";

  interface Props {
    open?: boolean;
    onclose?: () => void;
  }
  let { open = $bindable(false), onclose }: Props = $props();

  let copied = $state(false);

  const AI_CONTEXT = `## mclip — MonoClip CLI context

mclip is a command-line tool for managing clipboard history on macOS via MonoClip.
Use it to list, search, add, remove, and organise clips and folders from any terminal
or AI coding session.

### Clip commands

| Command | Description |
|---------|-------------|
| mclip list | List recent Inbox items (default: 20) |
| mclip list --folder Work | List items in a named folder |
| mclip list --search <query> | Search clips by content |
| mclip list --limit 50 | Show up to 50 items |
| mclip add "text" | Add a text clip to Inbox |
| mclip add "text" --folder Work | Add to a specific folder |
| mclip get <id> | Print raw content of a clip (pipe-friendly) |
| mclip remove <id> | Soft-delete a clip |
| mclip pin <id> | Pin a clip (protects from auto-cleanup) |
| mclip unpin <id> | Unpin a clip |

### Folder commands

| Command | Description |
|---------|-------------|
| mclip folder list | List all folders with item counts |
| mclip folder add "Name" | Create a new folder |
| mclip folder remove "Name" | Delete a folder (items move to Inbox) |

### Notes
- IDs come from the ID column in \`mclip list\` output.
- Folder names are case-insensitive.
- \`mclip get <id>\` outputs raw text with no trailing newline — ideal for piping:
  mclip get 42 | pbcopy
- Pinned clips are never removed by auto-cleanup.

### Examples
\`\`\`bash
# Show what's in the Work folder
mclip list --folder Work --limit 50

# Find recent URLs
mclip list --search http

# Add a snippet directly from terminal
mclip add "SELECT * FROM users LIMIT 10" --folder SQL

# Pipe a clip back to the system clipboard
mclip get 7 | pbcopy
\`\`\``;

  async function copyContext() {
    await navigator.clipboard.writeText(AI_CONTEXT);
    copied = true;
    setTimeout(() => { copied = false; }, 2500);
  }
</script>

{#if open}
  <!-- Backdrop -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 bg-black/40 z-40 flex items-end justify-stretch"
    onclick={() => { open = false; onclose?.(); }}
  >
    <div
      class="flex-1 bg-[#1c1c1e]/95 backdrop-blur-2xl border-t border-white/10
             rounded-t-2xl p-5 animate-slide-in z-50 max-h-[80%] overflow-y-auto"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
    >
      <!-- Header -->
      <div class="flex items-center justify-between mb-5">
        <h2 class="text-sm font-semibold text-white/90">Help</h2>
        <button
          class="text-white/40 hover:text-white/70 transition-colors"
          onclick={() => { open = false; onclose?.(); }}
        >✕</button>
      </div>

      <!-- Keyboard Shortcuts -->
      <section class="mb-5">
        <h3 class="text-xs font-medium text-white/40 uppercase tracking-wider mb-3">Keyboard Shortcuts</h3>
        <div class="space-y-2">
          {#each [
            ["Open / Close", "⌘ ⇧ V"],
            ["Search", "⌘ F  or  /"],
            ["Dismiss / Close", "Esc"],
          ] as [action, keys]}
            <div class="flex items-center justify-between">
              <span class="text-sm text-white/65">{action}</span>
              <kbd class="font-mono text-xs bg-white/8 border border-white/10 rounded-md px-2 py-0.5 text-white/70">{formatShortcut(keys)}</kbd>
            </div>
          {/each}
        </div>
      </section>

      <!-- Using the inbox -->
      <section class="mb-5">
        <h3 class="text-xs font-medium text-white/40 uppercase tracking-wider mb-3">Using MonoClip</h3>
        <ul class="space-y-2 text-sm text-white/60 list-none">
          <li class="flex gap-2"><span class="text-white/30 shrink-0">→</span>Everything you copy lands in <strong class="text-white/80">Inbox</strong> automatically.</li>
          <li class="flex gap-2"><span class="text-white/30 shrink-0">→</span>Click any clip to copy it back to your clipboard.</li>
          <li class="flex gap-2"><span class="text-white/30 shrink-0">→</span>Right-click a folder to rename or delete it.</li>
          <li class="flex gap-2"><span class="text-white/30 shrink-0">→</span>Images are captured and shown as thumbnails.</li>
          <li class="flex gap-2"><span class="text-white/30 shrink-0">→</span>Copying files or folders captures the full path.</li>
          <li class="flex gap-2"><span class="text-white/30 shrink-0">→</span>Pin a clip 📌 to keep it from being auto-cleaned.</li>
        </ul>
      </section>

      <!-- CLI -->
      <section class="mb-5">
        <h3 class="text-xs font-medium text-white/40 uppercase tracking-wider mb-3">CLI — mclip</h3>
        <p class="text-xs text-white/40 mb-3">
          Install via <span class="font-mono text-white/60">Settings → Install mclip CLI</span>, then add
          <span class="font-mono text-white/60">~/.local/bin</span> to your <span class="font-mono text-white/60">$PATH</span>.
        </p>
        <div class="bg-black/30 rounded-xl p-3 font-mono text-xs space-y-1.5 text-white/65">
          {#each [
            ["mclip list", "List recent inbox items"],
            ["mclip list --folder Work", "List a specific folder"],
            ["mclip list --search query", "Search clips"],
            ["mclip add \"text\"", "Add a clip to inbox"],
            ["mclip add \"text\" --folder Work", "Add to a folder"],
            ["mclip remove <id>", "Delete a clip"],
            ["mclip pin <id>", "Pin a clip"],
            ["mclip get <id>", "Print raw content (pipeable)"],
            ["mclip folder list", "List all folders"],
            ["mclip folder add \"Name\"", "Create a folder"],
            ["mclip folder remove \"Name\"", "Delete a folder"],
          ] as [cmd, desc]}
            <div class="flex gap-3">
              <span class="text-[#6366f1]/90 shrink-0 w-44">{cmd}</span>
              <span class="text-white/35"># {desc}</span>
            </div>
          {/each}
        </div>
      </section>

      <!-- AI Context -->
      <section class="mb-5">
        <h3 class="text-xs font-medium text-white/40 uppercase tracking-wider mb-3">Use with AI</h3>
        <p class="text-xs text-white/45 mb-3 leading-relaxed">
          Copy the context below and paste it into any AI coding session — Claude, Cursor, ChatGPT, etc.
          The AI will then understand all <code class="font-mono text-white/65">mclip</code> commands and
          can manage your clipboard history on your behalf.
          You can also save it to
          <code class="font-mono text-white/65">CLAUDE.md</code> or
          <code class="font-mono text-white/65">.cursorrules</code> so it's always available.
        </p>

        <!-- Context preview -->
        <div class="relative">
          <pre class="bg-black/40 border border-white/8 rounded-xl p-3 text-xs text-white/45
                      font-mono leading-relaxed overflow-hidden max-h-28
                      [mask-image:linear-gradient(to_bottom,white_40%,transparent)]">{AI_CONTEXT}</pre>
          <button
            class="mt-2 w-full py-2.5 rounded-xl text-sm font-medium border transition-all duration-200
                   {copied
                     ? 'bg-green-500/15 border-green-500/30 text-green-400'
                     : 'bg-[#6366f1]/10 border-[#6366f1]/25 text-[#a5b4fc] hover:bg-[#6366f1]/20'}"
            onclick={copyContext}
          >
            {copied ? '✓ Copied to clipboard' : 'Copy AI Context'}
          </button>
        </div>
      </section>

      <!-- Version -->
      <p class="text-xs text-white/20 text-center">MonoClip v0.2.2</p>
    </div>
  </div>
{/if}
