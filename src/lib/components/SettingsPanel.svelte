<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { runAutoCleanup, clearAllClips, installCli, checkAccessibility, openAccessibilitySettings } from "$lib/api/tauri";
  import { clipsStore } from "$lib/stores/clips.svelte";
  import { isMac, formatShortcut } from "$lib/utils/platform";
  import ShortcutRecorder from "./ShortcutRecorder.svelte";

  interface Props {
    open?: boolean;
    onclose?: () => void;
  }
  let { open = $bindable(false), onclose }: Props = $props();

  let cleanupResult = $state<string | null>(null);
  let clearConfirm = $state(false);
  let cliResult = $state<{ ok: boolean; msg: string } | null>(null);
  let accessibilityGranted = $state<boolean | null>(null);
  let appVersion = $state("");

  async function handleCleanup() {
    const count = await runAutoCleanup();
    cleanupResult = count > 0 ? `Removed ${count} clips` : "Nothing to clean yet";
    clipsStore.load();
    setTimeout(() => { cleanupResult = null; }, 3000);
  }

  async function handleClearAll() {
    if (!clearConfirm) { clearConfirm = true; return; }
    clearConfirm = false;
    await clearAllClips();
    clipsStore.load();
    cleanupResult = "All clips cleared";
    setTimeout(() => { cleanupResult = null; }, 3000);
  }

  async function handleInstallCli() {
    try {
      const path = await installCli();
      cliResult = { ok: true, msg: `Installed → ${path}` };
    } catch (e) {
      cliResult = { ok: false, msg: String(e) };
    }
    setTimeout(() => { cliResult = null; }, 5000);
  }

  async function refreshAccessibility() {
    accessibilityGranted = await checkAccessibility();
  }

  $effect(() => {
    if (open) {
      if (isMac) refreshAccessibility();
      getVersion().then((v) => { appVersion = v; });
    }
  });
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
      <div class="flex items-center justify-between mb-5">
        <h2 class="text-sm font-semibold text-white/90">Settings</h2>
        <button
          class="text-white/40 hover:text-white/70 transition-colors"
          onclick={() => { open = false; onclose?.(); }}
        >✕</button>
      </div>

      {#if settingsStore.data}
        {@const s = settingsStore.data}

        <!-- Permissions -->
        <section class="mb-5">
          <h3 class="text-xs font-medium text-white/40 uppercase tracking-wider mb-3">Permissions</h3>
          <div class="rounded-xl border border-white/8 overflow-hidden">

            <!-- Accessibility row (macOS only — Windows paste needs no permission) -->
            {#if isMac}
              <div class="p-3 flex items-start gap-3">
                <div class="mt-0.5 text-base leading-none">
                  {accessibilityGranted === true ? '✅' : accessibilityGranted === false ? '⚠️' : '⏳'}
                </div>
                <div class="flex-1 min-w-0">
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-sm font-medium text-white/80">Accessibility</span>
                    {#if accessibilityGranted === false}
                      <button
                        class="text-xs px-2.5 py-1 rounded-lg bg-[#6366f1]/20 border border-[#6366f1]/30
                               text-[#a5b4fc] hover:bg-[#6366f1]/30 transition-colors shrink-0"
                        onclick={openAccessibilitySettings}
                      >Open Settings →</button>
                    {/if}
                  </div>
                  <p class="text-xs text-white/35 mt-1 leading-relaxed">
                    {#if accessibilityGranted === true}
                      Granted — global shortcut and paste-on-click are working.
                    {:else if accessibilityGranted === false}
                      Not granted. MonoClip needs this to detect your keyboard shortcut
                      (<span class="font-mono">{formatShortcut("⌘⇧V")}</span>) and to auto-paste when you click a clip.<br/>
                      Click <strong class="text-white/55">Open Settings →</strong>, then add
                      <strong class="text-white/55">MonoClip</strong> and enable the toggle.
                      Reopen this panel to confirm.
                    {:else}
                      Checking…
                    {/if}
                  </p>
                </div>
              </div>

              <div class="border-t border-white/6 mx-3"></div>
            {/if}

            <!-- Launch at Login row -->
            <div class="p-3 flex items-start gap-3">
              <div class="mt-0.5 text-base leading-none">🔑</div>
              <div class="flex-1 min-w-0">
                <div class="flex items-center justify-between gap-2">
                  <span class="text-sm font-medium text-white/80">Launch at Login</span>
                  <input
                    type="checkbox"
                    checked={s.launchAtLogin}
                    onchange={(e) => settingsStore.update({ launchAtLogin: (e.target as HTMLInputElement).checked })}
                    class="w-4 h-4 accent-[#6366f1] shrink-0"
                  />
                </div>
                <p class="text-xs text-white/35 mt-1 leading-relaxed">
                  MonoClip will start automatically when you log in.
                  It appears under <strong class="text-white/55">Background Items</strong> in
                  System Settings → General → Login Items — this is normal for menu bar apps.
                </p>
              </div>
            </div>

          </div>
        </section>

        <!-- CLI Tools -->
        <section class="mb-5">
          <h3 class="text-xs font-medium text-white/40 uppercase tracking-wider mb-3">CLI Tools</h3>
          <div class="rounded-xl border border-white/8 overflow-hidden">
            <div class="p-3 flex items-start gap-3">
              <div class="mt-0.5 text-base leading-none">🖥️</div>
              <div class="flex-1 min-w-0">
                <span class="text-sm font-medium text-white/80">mclip</span>
                <p class="text-xs text-white/35 mt-1 leading-relaxed">
                  Installs <span class="font-mono text-white/55">mclip</span> to
                  <span class="font-mono text-white/55">~/.local/bin/</span> so you can manage
                  your clipboard from any terminal. Also enables AI tools via
                  <span class="font-mono text-white/55">mclip mcp</span>.
                  Make sure <span class="font-mono text-white/55">~/.local/bin</span> is in your
                  <span class="font-mono text-white/55">$PATH</span>.
                </p>
                <button
                  class="mt-2.5 w-full py-2 rounded-lg text-sm border transition-colors
                         {cliResult?.ok === false
                           ? 'text-red-400 border-red-500/30 bg-red-500/5'
                           : cliResult?.ok === true
                             ? 'text-green-400 border-green-500/30 bg-green-500/5'
                             : 'text-white/70 border-white/10 hover:bg-white/5'}"
                  onclick={handleInstallCli}
                >
                  {cliResult ? cliResult.msg : "Install mclip CLI"}
                </button>
              </div>
            </div>
          </div>
        </section>

        <!-- General -->
        <section class="mb-5">
          <h3 class="text-xs font-medium text-white/40 uppercase tracking-wider mb-3">General</h3>
          <div class="space-y-3">
            <label class="flex items-center justify-between">
              <span class="text-sm text-white/75">Paste on Click</span>
              <input
                type="checkbox"
                checked={s.pasteOnClick}
                onchange={(e) => settingsStore.update({ pasteOnClick: (e.target as HTMLInputElement).checked })}
                class="w-4 h-4 accent-[#6366f1]"
              />
            </label>

            <div class="flex items-center justify-between">
              <span class="text-sm text-white/75">Theme</span>
              <select
                value={s.theme}
                onchange={(e) => settingsStore.update({ theme: (e.target as HTMLSelectElement).value as "light" | "dark" | "system" })}
                class="bg-white/10 border border-white/10 rounded-lg px-2 py-1 text-sm text-white/80 outline-none"
              >
                <option value="system">System</option>
                <option value="dark">Dark</option>
                <option value="light">Light</option>
              </select>
            </div>
          </div>
        </section>

        <!-- Clipboard -->
        <section class="mb-5">
          <h3 class="text-xs font-medium text-white/40 uppercase tracking-wider mb-3">Clipboard</h3>
          <div class="space-y-3">
            <div>
              <div class="flex items-center justify-between mb-1.5">
                <span class="text-sm text-white/75">Max History</span>
                <span class="text-sm text-white/50">{s.maxHistoryItems} items</span>
              </div>
              <input
                type="range"
                min="50"
                max="2000"
                step="50"
                value={s.maxHistoryItems}
                oninput={(e) => settingsStore.update({ maxHistoryItems: Number((e.target as HTMLInputElement).value) })}
                class="w-full accent-[#6366f1]"
              />
            </div>
          </div>
        </section>

        <!-- Auto Cleanup -->
        <section class="mb-5">
          <h3 class="text-xs font-medium text-white/40 uppercase tracking-wider mb-3">Auto Cleanup</h3>
          <div class="space-y-3">
            <label class="flex items-center justify-between">
              <span class="text-sm text-white/75">Auto Clean</span>
              <input
                type="checkbox"
                checked={s.autoCleanEnabled}
                onchange={(e) => settingsStore.update({ autoCleanEnabled: (e.target as HTMLInputElement).checked })}
                class="w-4 h-4 accent-[#6366f1]"
              />
            </label>

            {#if s.autoCleanEnabled}
              <div>
                <div class="flex items-center justify-between mb-1.5">
                  <span class="text-sm text-white/75">Keep for</span>
                  <span class="text-sm text-white/50">{s.autoCleanDays} days</span>
                </div>
                <input
                  type="range"
                  min="1"
                  max="365"
                  value={s.autoCleanDays}
                  oninput={(e) => settingsStore.update({ autoCleanDays: Number((e.target as HTMLInputElement).value) })}
                  class="w-full accent-[#6366f1]"
                />
              </div>
            {/if}

            <div class="flex gap-2">
              <button
                class="flex-1 py-2 rounded-lg text-sm border transition-colors
                       {cleanupResult ? 'text-green-400 border-green-500/30 bg-green-500/5' : 'text-white/70 border-white/10 hover:bg-white/5'}"
                onclick={handleCleanup}
              >
                {cleanupResult ?? "Clean Now"}
              </button>
              <button
                class="flex-1 py-2 rounded-lg text-sm border transition-colors
                       {clearConfirm ? 'text-red-400 border-red-500/40 bg-red-500/10' : 'text-white/50 border-white/8 hover:border-red-500/30 hover:text-red-400'}"
                onclick={handleClearAll}
              >
                {clearConfirm ? "Tap again to confirm" : "Clear All Clips"}
              </button>
            </div>
            <p class="text-xs text-white/25">Clean Now removes old clips and trash. Clear All deletes everything except pinned.</p>
          </div>
        </section>

        <!-- Shortcuts -->
        <section>
          <h3 class="text-xs font-medium text-white/40 uppercase tracking-wider mb-3">Master Shortcut</h3>
          <ShortcutRecorder
            value={s.masterShortcut}
            onchange={(shortcut) => settingsStore.update({ masterShortcut: shortcut })}
          />
        </section>

        <!-- Version -->
        <p class="text-center text-xs text-white/20 mt-6">MonoClip v{appVersion}</p>
      {/if}
    </div>
  </div>
{/if}
