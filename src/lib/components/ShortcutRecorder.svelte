<script lang="ts">
  interface Props {
    value?: string;
    onchange?: (value: string) => void;
  }
  let { value = $bindable(""), onchange }: Props = $props();

  let recording = $state(false);
  let saved = $state(false);
  let inputEl: HTMLDivElement;

  // Map browser key names → Tauri shortcut format
  const KEY_MAP: Record<string, string> = {
    " ": "Space",
    "ArrowUp": "Up",
    "ArrowDown": "Down",
    "ArrowLeft": "Left",
    "ArrowRight": "Right",
    "Enter": "Return",
    "Backspace": "Backspace",
    "Delete": "Delete",
    "Tab": "Tab",
    "Escape": "Escape",
    "Home": "Home",
    "End": "End",
    "PageUp": "PageUp",
    "PageDown": "PageDown",
  };

  function formatShortcut(e: KeyboardEvent): string | null {
    // Ignore bare modifier key presses
    if (["Meta", "Alt", "Shift", "Control"].includes(e.key)) return null;

    const parts: string[] = [];
    if (e.metaKey || e.ctrlKey) parts.push("CmdOrCtrl");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");

    // Require at least one non-shift modifier for safety
    if (!e.metaKey && !e.ctrlKey && !e.altKey) return null;

    const rawKey = e.key;
    const mappedKey = KEY_MAP[rawKey]
      ?? (rawKey.length === 1 ? rawKey.toUpperCase() : rawKey);

    parts.push(mappedKey);
    return parts.join("+");
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();

    const shortcut = formatShortcut(e);
    if (shortcut) {
      value = shortcut;
      onchange?.(shortcut);
      recording = false;
      inputEl.blur();

      // Recording ending can otherwise look identical to nothing happening
      // (e.g. re-recording the same combo redisplays the same text), so flash
      // an explicit confirmation regardless of whether the value changed.
      saved = true;
      setTimeout(() => { saved = false; }, 1200);
    }
  }

  function startRecording() {
    recording = true;
  }

  function stopRecording() {
    recording = false;
  }

  function clear(e: MouseEvent) {
    e.stopPropagation();
    value = "";
    onchange?.("");
  }
</script>

<div class="relative">
  <!--
    A real <input readonly> here crashes WebKitGTK on Linux: the GTK input-method
    context attaches on focus, and when the value flips to "" while recording,
    WebKit computes a substring on stale offsets and hits
    `g_utf8_substring: assertion 'end_pos >= start_pos'`, taking down the render
    process. We never let the OS edit this field's text anyway, so a plain
    focusable div gets the same UX without touching the IME code path.
  -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    bind:this={inputEl}
    role="textbox"
    aria-readonly="true"
    aria-label={recording ? "Press shortcut" : value || "Click to record shortcut"}
    tabindex="0"
    class="w-full rounded-lg px-3 py-2 text-sm font-mono outline-none truncate
           cursor-pointer select-none transition-all duration-150
           {recording
             ? 'bg-accent/10 border border-accent/60 text-accent/50 ring-2 ring-accent/20'
             : saved
               ? 'bg-green-500/10 border border-green-500/50 text-green-400'
               : value
                 ? 'bg-white/5 border border-white/10 text-white/90 hover:border-white/20'
                 : 'bg-white/5 border border-white/10 text-white/30 hover:border-white/20'}"
    onfocus={startRecording}
    onblur={stopRecording}
    onkeydown={handleKeydown}
  >{recording ? "Press shortcut…" : saved ? `✓ Saved: ${value}` : value ? value : "Click to record…"}</div>

  <!-- Pulse dot while recording -->
  {#if recording}
    <span class="absolute left-3 top-1/2 -translate-y-1/2 w-1.5 h-1.5 rounded-full bg-accent animate-pulse"></span>
    <span class="absolute left-3 top-1/2 -translate-y-1/2 w-1.5 h-1.5 rounded-full bg-accent/40 animate-ping"></span>
  {/if}

  <!-- Clear button -->
  {#if value && !recording}
    <!-- svelte-ignore a11y_consider_explicit_label -->
    <button
      class="absolute right-2 top-1/2 -translate-y-1/2 w-4 h-4 flex items-center justify-center
             rounded text-white/30 hover:text-white/70 hover:bg-white/10 transition-colors text-[10px]"
      onmousedown={clear}
    >✕</button>
  {/if}
</div>
