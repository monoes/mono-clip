# Rich-Text Notes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add blank-card creation, a hand-rolled rich-text editor with autosave, styled/markdown copy modes, full JSON import/export, and matching `mclip mcp` tools to MonoClip.

**Architecture:** Backend-first: new SQLite-backed Tauri commands land before any UI consumes them. Markdown is the only storage format — `content_type` gains one new value (`"note"`) but no schema migration is needed since that column is already free-text. The editor is hand-rolled (`contenteditable` + `execCommand`) with `marked` doing Markdown⇄HTML conversion only; no editor framework is added.

**Tech Stack:** Existing stack unchanged (Tauri 2 / Rust / rusqlite / Svelte 5 / Tailwind). New deps: `marked` (frontend, MD→HTML), `@tauri-apps/plugin-dialog` + `tauri-plugin-dialog` (native file pickers), `@tauri-apps/plugin-fs` + `tauri-plugin-fs` (file read/write).

**Spec:** `docs/superpowers/specs/2026-09-11-rich-text-notes-design.md`

## Global Constraints

- No existing test framework in this project (neither Rust `#[test]` nor a JS test runner) — every task's "test" step is `cargo check` / `pnpm build` for compile correctness, plus a concrete manual verification in a real `tauri dev` session. Do not introduce a new test framework as part of this work.
- `content_type` is a free-text `TEXT` column with no CHECK constraint — no migration needed anywhere in this plan.
- Match existing code style exactly: double-quoted strings, no comments except where a WHY isn't obvious (see `ShortcutRecorder.svelte`'s WebKitGTK comment for the bar this project sets), emoji-only icons (no icon libraries), Tauri commands return `Result<T, String>` mapping errors via `.map_err(|e| e.to_string())`.
- Every new Cargo/npm dependency needs its exact required capability permission confirmed against the generated schema (`src-tauri/gen/schemas/desktop-schema.json`) before wiring it into `capabilities/default.json` — do not guess permission strings from memory (this project has been burned by exactly that mistake once already, with `core:window:allow-start-dragging`).

---

## Task 1: Backend — create blank clips and edit clip content

**Files:**
- Modify: `src-tauri/src/db/queries.rs`
- Modify: `src-tauri/src/commands/clips.rs`
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: `crate::db::models::ClipItem` (existing), `queries::get_clip` (existing, at `queries.rs:176`)
- Produces: `queries::create_blank_clip(conn: &Connection, folder_id: i64) -> Result<ClipItem>`, `queries::update_clip_content(conn: &Connection, id: i64, content: &str) -> Result<ClipItem>`, Tauri commands `create_blank_clip(state, folder_id: i64) -> Result<ClipItem, String>` and `update_clip_content(state, id: i64, content: String) -> Result<ClipItem, String>`

- [ ] **Step 1: Add the two query functions**

Add to `src-tauri/src/db/queries.rs`, near `insert_clip` (which this mirrors — same INSERT shape, empty content, `content_type = "note"`, no `source_app`):

```rust
pub fn create_blank_clip(conn: &Connection, folder_id: i64) -> Result<ClipItem> {
    conn.execute(
        "INSERT INTO clip_items (content, content_type, preview, folder_id) VALUES ('', 'note', '', ?1)",
        params![folder_id],
    )?;
    let id = conn.last_insert_rowid();
    get_clip(conn, id)
}

pub fn update_clip_content(conn: &Connection, id: i64, content: &str) -> Result<ClipItem> {
    let preview = crate::clipboard::detector::make_preview(content, 200);
    conn.execute(
        "UPDATE clip_items SET content = ?1, preview = ?2, updated_at = datetime('now') WHERE id = ?3",
        params![content, preview, id],
    )?;
    get_clip(conn, id)
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: no errors. (`crate::clipboard::detector::make_preview` already exists — confirm the exact path by checking `src-tauri/src/clipboard/detector.rs` if this doesn't resolve; it's used the same way in `commands/clips.rs::save_current_clipboard_to_folder`.)

- [ ] **Step 3: Add the Tauri commands**

Add to `src-tauri/src/commands/clips.rs`, near the top alongside `get_clip`/`pin_clip`:

```rust
#[tauri::command]
pub fn create_blank_clip(state: State<AppState>, folder_id: i64) -> Result<ClipItem, String> {
    let conn = state.db.lock();
    queries::create_blank_clip(&conn, folder_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_clip_content(state: State<AppState>, id: i64, content: String) -> Result<ClipItem, String> {
    let conn = state.db.lock();
    queries::update_clip_content(&conn, id, &content).map_err(|e| e.to_string())
}
```

- [ ] **Step 4: Register both commands in `main.rs`**

In `src-tauri/src/main.rs`, add to the `tauri::generate_handler![...]` list, in the `// Clips` section next to `clips::get_clip`:

```rust
            clips::create_blank_clip,
            clips::update_clip_content,
```

- [ ] **Step 5: Verify and commit**

Run: `cd src-tauri && cargo check` — expect no errors.

```bash
git add src-tauri/src/db/queries.rs src-tauri/src/commands/clips.rs src-tauri/src/main.rs
git commit -m "feat: add create_blank_clip and update_clip_content commands"
```

---

## Task 2: Backend — styled copy via arboard::set_html

**Files:**
- Modify: `src-tauri/src/commands/clips.rs`
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: `tauri_plugin_clipboard_manager` is NOT used here — `arboard::Clipboard` is used directly, since `set_html` isn't exposed through the Tauri clipboard-manager plugin, only through the underlying `arboard` crate this project already depends on (`Cargo.toml`: `arboard = "3.6"`).
- Produces: Tauri command `copy_clip_styled(html: String, plain_text: String) -> Result<(), String>`

- [ ] **Step 1: Add the command**

Add to `src-tauri/src/commands/clips.rs`:

```rust
#[tauri::command]
pub fn copy_clip_styled(html: String, plain_text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_html(html, Some(plain_text)).map_err(|e| e.to_string())
}
```

Note this command takes the already-rendered HTML and plain text from the frontend (Task 3 produces the renderer) — it does no Markdown parsing itself, matching the spec's decision to keep one renderer (the frontend's `marked` call) rather than duplicating Markdown parsing in Rust.

- [ ] **Step 2: Register in `main.rs`**

Add to the `// Clips` section of `generate_handler!`:

```rust
            clips::copy_clip_styled,
```

- [ ] **Step 3: Verify and commit**

Run: `cd src-tauri && cargo check` — expect no errors (if `arboard::Clipboard` doesn't resolve, add `use arboard;` is not needed since it's a fully-qualified path already, but confirm `arboard` is reachable as an external crate name matching `Cargo.toml`'s dependency name — it is, no rename in `[dependencies]`).

```bash
git add src-tauri/src/commands/clips.rs src-tauri/src/main.rs
git commit -m "feat: add copy_clip_styled command using arboard::set_html"
```

---

## Task 3: Frontend — Markdown helpers

**Files:**
- Create: `src/lib/utils/markdown.ts`
- Modify: `package.json`

**Interfaces:**
- Produces: `mdToHtml(md: string): string`, `htmlToMd(html: string): string`, `mdToPlainText(md: string): string`

- [ ] **Step 1: Add the `marked` dependency**

```bash
pnpm add marked
```

- [ ] **Step 2: Write the helpers**

Create `src/lib/utils/markdown.ts`:

```typescript
import { marked } from "marked";

marked.setOptions({ breaks: true, gfm: true });

export function mdToHtml(md: string): string {
  return marked.parse(md, { async: false }) as string;
}

export function mdToPlainText(md: string): string {
  const html = mdToHtml(md);
  const doc = new DOMParser().parseFromString(html, "text/html");
  return doc.body.textContent ?? "";
}

// Serializes exactly the tags NoteEditor's toolbar can produce. Not a
// general HTML-to-Markdown converter — anything outside this tag set
// (e.g. pasted-in <table>, <img>) falls through to its text content.
export function htmlToMd(html: string): string {
  const container = document.createElement("div");
  container.innerHTML = html;
  return walk(container).trim() + "\n";
}

function walk(node: Node): string {
  if (node.nodeType === Node.TEXT_NODE) {
    return node.textContent ?? "";
  }
  if (node.nodeType !== Node.ELEMENT_NODE) return "";

  const el = node as HTMLElement;
  const inner = Array.from(el.childNodes).map(walk).join("");

  switch (el.tagName) {
    case "B":
    case "STRONG":
      return `**${inner}**`;
    case "I":
    case "EM":
      return `*${inner}*`;
    case "H1":
      return `# ${inner}\n\n`;
    case "H2":
      return `## ${inner}\n\n`;
    case "UL":
      return Array.from(el.children).map((li) => `- ${walk(li).trim()}\n`).join("") + "\n";
    case "OL":
      return Array.from(el.children).map((li, i) => `${i + 1}. ${walk(li).trim()}\n`).join("") + "\n";
    case "LI":
      return inner;
    case "A": {
      const href = el.getAttribute("href") ?? "";
      return `[${inner}](${href})`;
    }
    case "P":
      return `${inner}\n\n`;
    case "BR":
      return "\n";
    case "DIV":
      // contenteditable wraps each line in a <div> in most webviews on Enter
      return `${inner}\n`;
    default:
      return inner;
  }
}
```

- [ ] **Step 3: Verify it builds**

Run: `pnpm build`
Expected: no errors, no new warnings from this file.

- [ ] **Step 4: Manual verification**

Run `pnpm dev`, open the browser console at `http://localhost:1420`, and check:
```js
const m = await import("/src/lib/utils/markdown.ts");
m.mdToHtml("**bold** and a list:\n- one\n- two");
// → "<p><strong>bold</strong> and a list:</p>\n<ul>\n<li>one</li>\n<li>two</li>\n</ul>"
m.htmlToMd("<div><b>bold</b> text</div><ul><li>one</li><li>two</li></ul>");
// → "**bold** text\n- one\n- two\n\n"
```
(Exact whitespace in `htmlToMd`'s output doesn't need to be pixel-perfect — it's read back through `mdToHtml` on next open, not diffed byte-for-byte. What matters: bold/italic/lists/headings survive a round trip.)

- [ ] **Step 5: Commit**

```bash
git add package.json pnpm-lock.yaml src/lib/utils/markdown.ts
git commit -m "feat: add Markdown<->HTML helpers for the note editor"
```

---

## Task 4: Frontend — API wrapper functions

**Files:**
- Modify: `src/lib/api/tauri.ts`

**Interfaces:**
- Consumes: Task 1's `create_blank_clip`/`update_clip_content`, Task 2's `copy_clip_styled`
- Produces: `createBlankClip(folderId: number): Promise<ClipItem>`, `updateClipContent(id: number, content: string): Promise<ClipItem>`, `copyClipStyled(html: string, plainText: string): Promise<void>`

- [ ] **Step 1: Add the wrapper functions**

Add to `src/lib/api/tauri.ts`, near the existing clip-related functions (find where `copyToClipboard`/`pinClip` are defined and follow the exact same `invoke("command_name", { args })` pattern already used there):

```typescript
export async function createBlankClip(folderId: number): Promise<ClipItem> {
  return invoke("create_blank_clip", { folderId });
}

export async function updateClipContent(id: number, content: string): Promise<ClipItem> {
  return invoke("update_clip_content", { id, content });
}

export async function copyClipStyled(html: string, plainText: string): Promise<void> {
  return invoke("copy_clip_styled", { html, plainText });
}
```

- [ ] **Step 2: Verify it builds**

Run: `pnpm build` — expect no errors. (`invoke` and `ClipItem` are already imported at the top of this file — confirm by checking the existing import block rather than adding a duplicate import.)

- [ ] **Step 3: Commit**

```bash
git add src/lib/api/tauri.ts
git commit -m "feat: add frontend API wrappers for note commands"
```

---

## Task 5: Frontend — NoteEditor component

**Files:**
- Create: `src/lib/components/NoteEditor.svelte`

**Interfaces:**
- Consumes: `mdToHtml`, `htmlToMd` (Task 3), `updateClipContent` (Task 4), `ClipItem` type (existing, from `$lib/api/tauri`)
- Produces: `<NoteEditor clip={...} bind:open={...} />` — a Svelte component other tasks mount

- [ ] **Step 1: Write the component**

Create `src/lib/components/NoteEditor.svelte`, matching the existing modal pattern from `HelpPanel.svelte`/`SettingsPanel.svelte` (same backdrop, slide-up sheet, header with a close button):

```svelte
<script lang="ts">
  import { updateClipContent } from "$lib/api/tauri";
  import { mdToHtml, htmlToMd } from "$lib/utils/markdown";
  import type { ClipItem } from "$lib/api/tauri";

  interface Props {
    clip: ClipItem | null;
    open?: boolean;
    onclose?: () => void;
  }
  let { clip, open = $bindable(false), onclose }: Props = $props();

  let editorEl: HTMLDivElement;
  let saveTimeout: ReturnType<typeof setTimeout>;

  $effect(() => {
    if (open && clip && editorEl) {
      editorEl.innerHTML = mdToHtml(clip.content);
    }
  });

  function exec(command: string) {
    document.execCommand(command, false);
    editorEl.focus();
    scheduleSave();
  }

  function scheduleSave() {
    clearTimeout(saveTimeout);
    saveTimeout = setTimeout(async () => {
      if (!clip) return;
      const md = htmlToMd(editorEl.innerHTML);
      await updateClipContent(clip.id, md);
    }, 600);
  }

  function close() {
    // Flush synchronously: closing within 600ms of the last keystroke would
    // otherwise race the debounced save and silently drop the edit — found
    // during Task 5's review, not present in this plan's original draft.
    clearTimeout(saveTimeout);
    if (clip && editorEl) {
      updateClipContent(clip.id, htmlToMd(editorEl.innerHTML));
    }
    open = false;
    onclose?.();
  }
</script>

{#if open && clip}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 bg-black/40 z-40 flex items-end justify-stretch"
    onclick={close}
  >
    <div
      class="flex-1 bg-[#1c1c1e]/95 backdrop-blur-2xl border-t border-white/10
             rounded-t-2xl p-5 animate-slide-in z-50 max-h-[80%] overflow-y-auto"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
    >
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-sm font-semibold text-white/90">Note</h2>
        <button
          class="text-white/40 hover:text-white/70 transition-colors"
          onclick={close}
        >✕</button>
      </div>

      <div class="flex items-center gap-1 mb-3 pb-3 border-b border-white/8">
        {#each [["bold", "B"], ["italic", "I"], ["insertUnorderedList", "•"], ["insertOrderedList", "1."], ["formatBlock", "H1", "<h1>"], ["formatBlock", "H2", "<h2>"]] as [cmd, label, value]}
          <button
            class="w-7 h-7 rounded-md text-xs font-semibold text-white/60 hover:bg-white/10 hover:text-white/90 transition-colors"
            onmousedown={(e) => e.preventDefault()}
            onclick={() => value ? (document.execCommand(cmd, false, value), scheduleSave()) : exec(cmd)}
          >{label}</button>
        {/each}
      </div>

      <div
        bind:this={editorEl}
        contenteditable="true"
        class="min-h-[200px] text-sm text-white/90 leading-relaxed outline-none
               [&_strong]:font-semibold [&_em]:italic [&_ul]:list-disc [&_ul]:pl-5
               [&_ol]:list-decimal [&_ol]:pl-5 [&_h1]:text-lg [&_h1]:font-semibold
               [&_h2]:text-base [&_h2]:font-semibold"
        oninput={scheduleSave}
      ></div>
    </div>
  </div>
{/if}
```

- [ ] **Step 2: Verify it builds**

Run: `pnpm build` — expect no errors. This step can't be manually verified in isolation yet (nothing mounts `NoteEditor` until Task 6) — that verification happens in Task 6's manual check instead.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/NoteEditor.svelte
git commit -m "feat: add NoteEditor component with autosave"
```

---

## Task 6: Frontend — New Card button and pen icon

**Files:**
- Modify: `src/App.svelte`
- Modify: `src/lib/components/Sidebar.svelte`
- Modify: `src/lib/components/ClipCard.svelte`

**Interfaces:**
- Consumes: `createBlankClip` (Task 4), `NoteEditor` (Task 5)
- Produces: wires the whole New Card → edit flow end-to-end; nothing further depends on this task

- [ ] **Step 1: Add state and the New Card handler to `App.svelte`**

In `src/App.svelte`, add alongside the existing `showSettings`/`showHelp` state:

```typescript
  import NoteEditor from "$lib/components/NoteEditor.svelte";
  import { createBlankClip } from "$lib/api/tauri";
```

```typescript
  let editingClip: ClipItem | null = $state(null);
  let showEditor = $state(false);

  async function handleNewCard() {
    const folderId = foldersStore.activeId ?? 1;
    const clip = await createBlankClip(folderId);
    clipsStore.prependItem(clip);
    editingClip = clip;
    showEditor = true;
  }

  function openEditor(clip: ClipItem) {
    editingClip = clip;
    showEditor = true;
  }
```

Pass `onNewCard={handleNewCard}` to `<Sidebar>` and add `<NoteEditor clip={editingClip} bind:open={showEditor} />` next to the existing `<HelpPanel>`/`<SettingsPanel>` mounts at the bottom of the file. `ClipGrid` needs an `onEditClip` prop threaded through to `ClipCard` — add `onEditClip={openEditor}` to the existing `<ClipGrid ... />` usage.

- [ ] **Step 2: Add the button to `Sidebar.svelte`**

Add an `onNewCard?: () => void` prop next to the existing `onSettingsClick`/`onHelpClick` props, and a button near wherever "+ New Folder" already lives, following that button's existing styling exactly (read the surrounding markup before adding — match its classes verbatim rather than reinventing them):

```svelte
<button onclick={onNewCard} title="New blank card">
  {/* match the existing "+ New Folder" button's icon+label markup and classes here */}
</button>
```

- [ ] **Step 3: Thread `onEditClip` through `ClipGrid.svelte` to `ClipCard.svelte`**

`ClipGrid.svelte` needs an `onEditClip?: (clip: ClipItem) => void` prop added to its `Props` interface, passed straight through to each `<ClipCard ... onEditClip={onEditClip} />` it renders.

- [ ] **Step 4: Add the pen icon and style/md buttons to `ClipCard.svelte`**

Add to the `Props` interface:

```typescript
    onEditClip?: (clip: ClipItem) => void;
```

Add near the top of the `<script>` block, alongside the other imports:

```typescript
  import { copyClipStyled } from "$lib/api/tauri";
  import { mdToHtml, mdToPlainText } from "$lib/utils/markdown";

  const penEditableTypes = ["text", "url", "code", "note"];
```

Add handlers near `handleDelete`:

```typescript
  function handleEdit(e: MouseEvent) {
    e.stopPropagation();
    onEditClip?.(clip);
  }

  async function handleCopyStyled(e: MouseEvent) {
    e.stopPropagation();
    await copyClipStyled(mdToHtml(clip.content), mdToPlainText(clip.content));
    clipsStore.setFlashing(clip.id);
  }

  async function handleCopyMd(e: MouseEvent) {
    e.stopPropagation();
    await handleCopy(e);
  }
```

In the hover-actions `<div>` (the one currently containing the pin and delete buttons), add the pen icon before them, gated on the content type:

```svelte
      {#if penEditableTypes.includes(clip.contentType)}
        <button
          class="p-1 rounded-md hover:bg-white/15 text-white/50 hover:text-white/90 text-xs transition-colors"
          onclick={handleEdit}
          title="Edit"
        >✏️</button>
      {/if}
```

Add the style/md buttons as a new footer row, only for pen-editable types, replacing nothing (the existing footer with timestamp + pin/delete stays exactly as-is above this new row):

```svelte
  {#if penEditableTypes.includes(clip.contentType)}
    <div class="flex gap-2 mt-2 pt-2 border-t border-white/6">
      <button
        class="flex-1 py-1 rounded-md text-[10px] text-white/50 hover:bg-white/10 hover:text-white/80 transition-colors"
        onclick={handleCopyStyled}
      >style</button>
      <button
        class="flex-1 py-1 rounded-md text-[10px] text-white/50 hover:bg-white/10 hover:text-white/80 transition-colors"
        onclick={handleCopyMd}
      >md</button>
    </div>
  {/if}
```

- [ ] **Step 5: Verify it builds**

Run: `pnpm build` — expect no errors.

- [ ] **Step 6: Manual verification (real app, per this project's existing testing convention)**

```bash
cd src-tauri && cargo check   # confirm backend still compiles after Task 1/2 additions
```
Then run a real `tauri dev` (see this project's established sandboxed-HOME technique from earlier sessions if testing against production data is a concern) and:
1. Click "New Card" → confirm a blank card appears and the editor opens immediately.
2. Type some text, use the Bold/bullet-list toolbar buttons, close the editor (click outside), reopen via the pen icon → confirm the formatting persisted.
3. Click "md" on that card → paste into a plain-text field → confirm raw Markdown appears.
4. Click "style" on that card → paste into TextEdit.app (or similar) → confirm bold/lists render as actual formatting, not literal `**`/`-` characters.
5. Click the pen icon on an existing plain clipboard-captured text clip (not a "New Card" one) → confirm it opens the same editor and edits save.

- [ ] **Step 7: Commit**

```bash
git add src/App.svelte src/lib/components/Sidebar.svelte src/lib/components/ClipGrid.svelte src/lib/components/ClipCard.svelte
git commit -m "feat: wire up New Card, pen-icon editing, and style/md copy"
```

---

## Task 7: Backend — add dialog and fs plugins

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `package.json`

**Interfaces:**
- Produces: `@tauri-apps/plugin-dialog`'s `save()`/`open()` and `@tauri-apps/plugin-fs`'s `writeTextFile()`/`readTextFile()` become callable from the frontend in Task 9.

- [ ] **Step 1: Add the JS packages**

```bash
pnpm add @tauri-apps/plugin-dialog @tauri-apps/plugin-fs
```

- [ ] **Step 2: Add the Rust crates**

In `src-tauri/Cargo.toml`, add next to `tauri-plugin-opener`:

```toml
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
```

- [ ] **Step 3: Register both plugins**

In `src-tauri/src/main.rs`, add next to `.plugin(tauri_plugin_opener::init())`:

```rust
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
```

- [ ] **Step 4: Confirm the exact permission identifiers, then add them**

Run: `cd src-tauri && cargo check` first (this regenerates `gen/schemas/desktop-schema.json` with the new plugins' real permission identifiers — do not guess these).

```bash
grep -o '"const": "dialog:[a-z-]*"' src-tauri/gen/schemas/desktop-schema.json | sort -u
grep -o '"const": "fs:[a-z-]*"' src-tauri/gen/schemas/desktop-schema.json | sort -u
```

Add the resulting default/allow identifiers for opening and saving files to `src-tauri/capabilities/default.json`'s `permissions` array (expect something like `dialog:default` and a scoped `fs:allow-write-text-file` / `fs:allow-read-text-file`, but confirm against the actual grep output above rather than typing these from memory — this is exactly the mistake that broke window dragging earlier in this project).

- [ ] **Step 5: Verify and commit**

Run: `cd src-tauri && cargo check` — expect no errors, and no "unknown permission" failures at startup when this is exercised in Task 9's manual check.

```bash
git add package.json pnpm-lock.yaml src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/main.rs src-tauri/capabilities/default.json src-tauri/gen/schemas/
git commit -m "feat: add dialog and fs plugins for import/export"
```

---

## Task 8: Backend — export_data and import_data

**Files:**
- Create: `src-tauri/src/commands/backup.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: `queries::get_folders` (existing), `crate::clipboard::image_store` (existing — check its exact read/write function names in `src-tauri/src/clipboard/image_store.rs` before writing this task's code, since this plan hasn't verified them; the referenced functions below are named per the spec's description and must be matched to what actually exists there before implementation)
- Produces: Tauri commands `export_data() -> Result<String, String>`, `import_data(json: String) -> Result<ImportSummary, String>`

- [ ] **Step 1: Write `backup.rs`**

Create `src-tauri/src/commands/backup.rs`:

```rust
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::db::queries;
use crate::state::AppState;

#[derive(Serialize)]
struct ExportFolder {
    name: String,
    icon: String,
    color: String,
    global_shortcut: Option<String>,
    position: i64,
}

#[derive(Serialize)]
struct ExportClip {
    folder_name: String,
    content: String,
    content_type: String,
    preview: String,
    is_pinned: bool,
    created_at: String,
    image_data: Option<String>,
}

#[derive(Serialize)]
struct ExportPayload {
    format_version: u32,
    exported_at: String,
    folders: Vec<ExportFolder>,
    clips: Vec<ExportClip>,
}

#[derive(Deserialize)]
struct ImportClip {
    folder_name: String,
    content: String,
    content_type: String,
    preview: String,
    is_pinned: bool,
    #[allow(dead_code)]
    created_at: String,
    image_data: Option<String>,
}

#[derive(Deserialize)]
struct ImportFolder {
    name: String,
    icon: String,
    color: String,
    global_shortcut: Option<String>,
    position: i64,
}

#[derive(Deserialize)]
struct ImportPayload {
    #[allow(dead_code)]
    format_version: u32,
    folders: Vec<ImportFolder>,
    clips: Vec<ImportClip>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    folders_created: i64,
    folders_merged: i64,
    clips_imported: i64,
}

#[tauri::command]
pub fn export_data(state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock();
    let folders = queries::get_folders(&conn).map_err(|e| e.to_string())?;
    let folder_names: std::collections::HashMap<i64, String> =
        folders.iter().map(|f| (f.id, f.name.clone())).collect();

    let export_folders: Vec<ExportFolder> = folders
        .iter()
        .map(|f| ExportFolder {
            name: f.name.clone(),
            icon: f.icon.clone(),
            color: f.color.clone(),
            global_shortcut: f.global_shortcut.clone(),
            position: f.position,
        })
        .collect();

    let all_clips = queries::get_clips(&conn, None, None, i64::MAX, 0).map_err(|e| e.to_string())?;
    let mut export_clips = Vec::new();
    for clip in all_clips.into_iter().filter(|c| !c.is_deleted) {
        let image_data = if clip.content_type == "image" {
            std::fs::read(&clip.content).ok().map(|bytes| {
                format!("data:image/png;base64,{}", base64_encode(&bytes))
            })
        } else {
            None
        };
        export_clips.push(ExportClip {
            folder_name: folder_names.get(&clip.folder_id).cloned().unwrap_or_else(|| "Inbox".into()),
            content: clip.content,
            content_type: clip.content_type,
            preview: clip.preview,
            is_pinned: clip.is_pinned,
            created_at: clip.created_at,
            image_data,
        });
    }

    let payload = ExportPayload {
        format_version: 1,
        exported_at: chrono::Utc::now().to_rfc3339(),
        folders: export_folders,
        clips: export_clips,
    };
    serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_data(state: State<AppState>, json: String) -> Result<ImportSummary, String> {
    let payload: ImportPayload = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let conn = state.db.lock();

    let mut folders_created = 0i64;
    let mut folders_merged = 0i64;
    let mut folder_ids: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    let existing = queries::get_folders(&conn).map_err(|e| e.to_string())?;
    for f in &existing {
        folder_ids.insert(f.name.clone(), f.id);
    }

    for f in &payload.folders {
        if folder_ids.contains_key(&f.name) {
            folders_merged += 1;
            continue;
        }
        let created = queries::create_folder(&conn, &f.name, &f.icon, &f.color, f.global_shortcut.as_deref())
            .map_err(|e| e.to_string())?;
        folder_ids.insert(f.name.clone(), created.id);
        folders_created += 1;
    }

    let mut clips_imported = 0i64;
    for c in &payload.clips {
        let folder_id = *folder_ids.get(&c.folder_name).unwrap_or(&1);
        let content = if let (Some(data_uri), "image") = (&c.image_data, c.content_type.as_str()) {
            let b64 = data_uri.split(',').nth(1).unwrap_or("");
            let png_bytes = base64_decode(b64);
            // image_store::save_as_png takes decoded RGBA pixels + dimensions,
            // not a raw PNG byte blob — decode first (the `image` crate is
            // already a project dependency).
            match image::load_from_memory(&png_bytes) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    crate::clipboard::image_store::save_as_png(rgba.as_raw(), w, h)
                        .unwrap_or_else(|_| c.content.clone())
                }
                Err(_) => c.content.clone(),
            }
        } else {
            c.content.clone()
        };
        queries::insert_clip(&conn, &content, &c.content_type, &c.preview, folder_id, None)
            .map_err(|e| e.to_string())?;
        if c.is_pinned {
            if let Ok(clip) = queries::get_clip(&conn, conn.last_insert_rowid()) {
                let _ = queries::set_clip_pinned(&conn, clip.id, true);
            }
        }
        clips_imported += 1;
    }

    Ok(ImportSummary { folders_created, folders_merged, clips_imported })
}

// Minimal base64 — avoids adding a dependency for two one-line operations.
// Replace with the `base64` crate if this project already depends on it
// elsewhere by the time this task is implemented (check Cargo.toml first).
fn base64_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
        let _ = write!(out, "{}", CHARS[((n >> 18) & 63) as usize] as char);
        let _ = write!(out, "{}", CHARS[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { CHARS[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { CHARS[(n & 63) as usize] as char } else { '=' });
    }
    out
}

fn base64_decode(s: &str) -> Vec<u8> {
    fn val(c: u8) -> u32 {
        match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            _ => 0,
        }
    }
    let clean: Vec<u8> = s.bytes().filter(|&b| b != b'=').collect();
    let mut out = Vec::new();
    for chunk in clean.chunks(4) {
        let n = chunk.iter().enumerate().fold(0u32, |acc, (i, &c)| acc | (val(c) << (18 - 6 * i)));
        out.push((n >> 16) as u8);
        if chunk.len() > 2 { out.push((n >> 8) as u8); }
        if chunk.len() > 3 { out.push(n as u8); }
    }
    out
}
```

Both `queries::create_folder(conn, name: &str, icon: &str, color: &str, shortcut: Option<&str>) -> Result<Folder>` and `image_store::save_as_png(rgba: &[u8], width: u32, height: u32) -> Result<String>` have been confirmed against the actual source (see the ledger's pre-flight scan) — the code above already calls them with the correct shapes. No further signature-hunting needed for this task.

- [ ] **Step 2: Wire up the module and commands**

Add to `src-tauri/src/commands/mod.rs`: `pub mod backup;`

Add to `main.rs`'s `generate_handler!`, in a new `// Backup` section:

```rust
            backup::export_data,
            backup::import_data,
```

- [ ] **Step 3: Verify and commit**

Run: `cd src-tauri && cargo check` — expect no errors.

```bash
git add src-tauri/src/commands/backup.rs src-tauri/src/commands/mod.rs src-tauri/src/main.rs
git commit -m "feat: add export_data and import_data commands"
```

---

## Task 9: Frontend — Import/Export in Settings

**Files:**
- Modify: `src/lib/components/SettingsPanel.svelte`
- Modify: `src/lib/api/tauri.ts`

**Interfaces:**
- Consumes: `export_data`/`import_data` (Task 8), `@tauri-apps/plugin-dialog` + `@tauri-apps/plugin-fs` (Task 7)

- [ ] **Step 1: Add API wrappers**

Add to `src/lib/api/tauri.ts`:

```typescript
export async function exportData(): Promise<string> {
  return invoke("export_data");
}

export async function importData(json: string): Promise<{ foldersCreated: number; foldersMerged: number; clipsImported: number }> {
  return invoke("import_data", { json });
}
```

- [ ] **Step 2: Add the Settings section**

Add to `src/lib/components/SettingsPanel.svelte`, as a new `<section>` following the existing pattern (e.g. right after the "CLI Tools" section), importing `save`/`open` from `@tauri-apps/plugin-dialog` and `writeTextFile`/`readTextFile` from `@tauri-apps/plugin-fs` at the top of the `<script>` block. **Note: this component already destructures a bindable prop named `open` (`let { open = $bindable(false), onclose }: Props = $props();`) — the dialog plugin's `open` import collides with it and must be aliased**, e.g. `import { save, open as openDialog } from "@tauri-apps/plugin-dialog";`, with the call site updated to `openDialog(...)` accordingly. `foldersStore` must also be imported here if it isn't already (`import { foldersStore } from "$lib/stores/folders.svelte";`) — check the file's current imports rather than assuming:

```typescript
  import { save, open as openDialog } from "@tauri-apps/plugin-dialog";
  import { writeTextFile, readTextFile } from "@tauri-apps/plugin-fs";
  import { exportData, importData } from "$lib/api/tauri";

  let backupResult = $state<string | null>(null);

  async function handleExport() {
    const path = await save({ defaultPath: "monoclip-backup.json", filters: [{ name: "JSON", extensions: ["json"] }] });
    if (!path) return;
    try {
      const json = await exportData();
      await writeTextFile(path, json);
      backupResult = "Exported";
    } catch (e) {
      backupResult = `Export failed: ${e}`;
    }
    setTimeout(() => { backupResult = null; }, 3000);
  }

  async function handleImport() {
    const path = await openDialog({ filters: [{ name: "JSON", extensions: ["json"] }] });
    if (!path || Array.isArray(path)) return;
    try {
      const json = await readTextFile(path);
      const summary = await importData(json);
      backupResult = `Imported ${summary.clipsImported} clips, ${summary.foldersCreated} new folders`;
      clipsStore.load(foldersStore.activeId ?? 1);
      foldersStore.load();
    } catch (e) {
      backupResult = `Import failed: ${e}`;
    }
    setTimeout(() => { backupResult = null; }, 5000);
  }
```

```svelte
        <section class="mb-5">
          <h3 class="text-xs font-medium text-white/40 uppercase tracking-wider mb-3">Backup</h3>
          <div class="flex gap-2">
            <button
              class="flex-1 py-2 rounded-lg text-sm border transition-colors text-white/70 border-white/10 hover:bg-white/5"
              onclick={handleExport}
            >Export All</button>
            <button
              class="flex-1 py-2 rounded-lg text-sm border transition-colors text-white/70 border-white/10 hover:bg-white/5"
              onclick={handleImport}
            >Import</button>
          </div>
          {#if backupResult}
            <p class="text-xs text-green-400 mt-2">{backupResult}</p>
          {/if}
        </section>
```

`clipsStore`/`foldersStore` are already imported in this file (used elsewhere for `handleCleanup`/`handleClearAll`) — confirm rather than re-import.

- [ ] **Step 3: Verify it builds**

Run: `pnpm build` — expect no errors.

- [ ] **Step 4: Manual verification**

In a real `tauri dev` run: create a couple of folders/notes, Export All to a file, then use the sandboxed-HOME technique to launch a second instance against an empty database and Import that same file — confirm folders and notes appear, including at least one image clip if one was in the export.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/SettingsPanel.svelte src/lib/api/tauri.ts
git commit -m "feat: add Import/Export to Settings"
```

---

## Task 10: MCP — create_note, update_clip_content, export/import

**Files:**
- Modify: `src-tauri/src/bin/mclip.rs`

**Interfaces:**
- Consumes: `open_db()`, `resolve_folder_id()`, `detect_type()`, `make_preview()` (all existing, confirmed at `mclip.rs:95/113/131/154`)

- [ ] **Step 1: Add tool schemas**

Add to the `tools` array in `cmd_mcp()` (`mclip.rs`, alongside the existing 8 entries), before the closing `]);`:

```rust
        {
            "name": "create_note",
            "description": "Create a blank note card in MonoClip",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "folder": { "type": "string", "description": "Destination folder (default: Inbox)" }
                }
            }
        },
        {
            "name": "update_clip_content",
            "description": "Replace the content of an existing clip or note",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Clip ID (from list_clips)" },
                    "content": { "type": "string", "description": "New content (Markdown for notes)" }
                },
                "required": ["id", "content"]
            }
        }
```

(`export_notes`/`import_notes` are deferred — see the note at the end of this task.)

- [ ] **Step 2: Add dispatch arms**

Add to the `match name` block in `mcp_call_tool()`, before the final `_ => err(...)`:

```rust
        "create_note" => {
            let folder = args.get("folder").and_then(|v| v.as_str()).map(String::from);
            match mcp_create_note(folder) {
                Ok(msg) => text(msg),
                Err(e) => err(e),
            }
        }
        "update_clip_content" => {
            let id = match args.get("id").and_then(|v| v.as_i64()) {
                Some(i) => i,
                None => return err("missing required field: id".to_string()),
            };
            let content = match args.get("content").and_then(|v| v.as_str()) {
                Some(c) => c.to_string(),
                None => return err("missing required field: content".to_string()),
            };
            match mcp_update_clip_content(id, content) {
                Ok(msg) => text(msg),
                Err(e) => err(e),
            }
        }
```

- [ ] **Step 3: Add the handler functions**

Add near `mcp_add_clip` (mirroring its exact shape — same `open_db()`/`resolve_folder_id` pattern):

```rust
fn mcp_create_note(folder: Option<String>) -> Result<String, String> {
    let conn = open_db();
    let folder_id = folder.as_deref().map(|f| resolve_folder_id(&conn, f)).unwrap_or(1);
    conn.execute(
        "INSERT INTO clip_items (content, content_type, preview, folder_id) VALUES ('', 'note', '', ?1)",
        params![folder_id],
    ).map_err(|e| e.to_string())?;
    Ok(format!("Created note #{}", conn.last_insert_rowid()))
}

fn mcp_update_clip_content(id: i64, content: String) -> Result<String, String> {
    let conn = open_db();
    let preview = make_preview(&content, 200);
    let affected = conn.execute(
        "UPDATE clip_items SET content = ?1, preview = ?2, updated_at = datetime('now') WHERE id = ?3",
        params![content, preview, id],
    ).unwrap_or(0);
    if affected == 0 { return Err(format!("clip #{} not found", id)); }
    Ok(format!("Updated clip #{}", id))
}
```

- [ ] **Step 4: Verify and commit**

Run: `cd src-tauri && cargo check --bin mclip` — expect no errors.

```bash
git add src-tauri/src/bin/mclip.rs
git commit -m "feat: add create_note and update_clip_content MCP tools"
```

**Note on `export_notes`/`import_notes`:** the spec calls for these too, but they'd need to either duplicate Task 8's Rust logic into `mclip.rs` (a separate binary that doesn't share `commands/backup.rs`) or extract that logic into `monoclip_lib` so both binaries can call it. That extraction is a real design decision, not a bite-sized step — flag it back to the human partner rather than guessing which way to cut it once the first 9 tasks are done and this is the only thing left.

---

## Self-Review Notes

- **Spec coverage:** every spec section has a task — data model (Task 1), New Card (Task 1+6), editor (Task 3+5+6), style/md copy (Task 2+6), import/export (Task 7+8+9), MCP (Task 10, with `export_notes`/`import_notes` explicitly called out as needing a follow-up decision rather than silently dropped).
- **Type consistency:** `ClipItem`, `create_blank_clip`, `update_clip_content`, `copy_clip_styled` names and signatures are identical everywhere they're referenced across Tasks 1/2/4/6.
- **Known gap, flagged rather than guessed:** Task 8's `create_folder`/`image_store` call signatures are explicitly marked to be confirmed against the real source before writing that file, rather than invented from the spec's prose description alone.
