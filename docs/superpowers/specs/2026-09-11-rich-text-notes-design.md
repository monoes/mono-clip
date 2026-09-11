# Rich-text notes: new card, editor, styled copy, import/export, MCP

Status: approved in chat 2026-09-11, written up for implementation.

## Summary

Turn MonoClip's clip cards into lightly-editable notes: a "New Card" action
to create a blank one, a pen icon on any text-ish card that opens a small
hand-rolled rich-text editor with autosave, two copy modes (styled HTML vs.
raw Markdown), a full backup/restore (Import/Export) of all folders and
clips as one JSON file, and matching `mclip mcp` tools.

## Data model

No migration needed. `content_type` is a free-text column already
(`TEXT NOT NULL DEFAULT 'text'`, no CHECK constraint) — this adds one new
value, `"note"`, used for cards created via "New Card" so the UI can flag
them distinctly. Existing clip types (`text`, `url`, `code`) become
pen-editable in place without their `content_type` changing; editing never
reclassifies a captured clip as a note. `image`, `color`, `file` are not
pen-editable — the icon doesn't appear on those.

`content` continues to store the raw value (Markdown, once a card has been
through the editor; unmodified captured text otherwise — the two are not
visually distinguishable in storage, only in whether the UI treats a given
`content_type` as pen-editable).

## New Card

- New button in `Sidebar.svelte`, beside "+ New Folder".
- Calls a new command `create_blank_clip(folder_id)` → inserts
  `content_type = 'note'`, `content = ''`, `preview = ''` into the given
  folder (falls back to folder id `1` / Inbox if no folder is active, e.g.
  the "All Clips" view), returns the created `ClipItem`.
- Frontend immediately opens the editor (see below) for the returned clip
  so the flow is click → typing, no intermediate blank-card state to find
  and click into separately.

## Editor (pen icon)

New `NoteEditor.svelte`, modal-styled like `HelpPanel`/`SettingsPanel`
(same backdrop/slide-up treatment). Contents:

- A small hand-rolled toolbar: Bold, Italic, Bullet list, Numbered list,
  H1, H2. Implemented with `document.execCommand` on a `contenteditable`
  div — acceptable here because the target is exactly three consistent
  webview engines (WKWebView / WebView2 / WebKitGTK) via Tauri, not the
  open web; a public-site-grade compatibility concern doesn't apply.
- On open: stored Markdown → HTML via `marked` (new frontend dependency,
  ~35KB, zero deps) to populate the `contenteditable`. Using a library for
  *parsing* is a different, higher-correctness-risk problem than hand-rolling
  the *editor interaction* — the toolbar and modal stay hand-rolled, the
  Markdown grammar itself doesn't.
- On input, debounced 600ms: serialize the `contenteditable`'s current HTML
  back to Markdown with a small custom serializer. This does **not** need to
  handle arbitrary HTML — only the tags our own toolbar can ever produce
  (`b`/`strong`, `i`/`em`, `ul`/`ol`/`li`, `h1`/`h2`, `p`/`br`, plus bare
  text nodes). Call new command `update_clip_content(id, content)` to
  persist. No save button — this debounced call *is* the save.
- Closing (Esc / click outside / pen icon again) just closes the modal;
  nothing further to flush, the debounce has already been saving throughout.

## Copy modes: style / md

Two small buttons at the bottom of any pen-editable card (replacing the
current whole-card-click-to-copy for these types specifically; other
content types keep today's click-to-copy behavior unchanged):

- **md** — calls the existing `copy_to_clipboard(id)` unchanged. Writes the
  raw Markdown source as plain text, exactly like today's behavior for any
  other clip.
- **style** — new command `copy_clip_styled(id)`. Frontend renders the
  clip's Markdown to HTML via `marked` (same renderer as the editor) and
  also derives a plain-text version (HTML tags stripped — a rendered
  reading, not the raw Markdown syntax, since that's what a plain-text-only
  paste target should reasonably see). Passes both strings to
  `copy_clip_styled(id, html, plain_text)`, which calls
  `arboard::set_html(html, Some(plain_text))` — already available in the
  `arboard` version this project pins (3.6.1), confirmed via docs.rs; no new
  Rust dependency.

## Import / Export (Settings)

New "Backup" section in `SettingsPanel.svelte`, two buttons.

**Format** — one JSON file:

```json
{
  "formatVersion": 1,
  "exportedAt": "2026-09-11T12:00:00Z",
  "folders": [
    { "name": "Work", "icon": "💼", "color": "#6366f1", "globalShortcut": null, "position": 0 }
  ],
  "clips": [
    {
      "folderName": "Work",
      "content": "...",
      "contentType": "note",
      "preview": "...",
      "isPinned": false,
      "createdAt": "2026-09-01T10:00:00Z",
      "imageData": null
    }
  ]
}
```

Notes on the shape:
- Folders/clips are matched by **name**, not id, on import — local
  `id`/`folderId` are never round-tripped, since they're meaningless across
  two different databases and would only invite collisions.
- `imageData`: for `content_type = "image"` clips, the referenced PNG file
  (see `clipboard/image_store.rs`) is read and embedded as a base64 data
  URI here, `content` holds the original file path as before (informational
  only — not reused verbatim on import). This was the one point explicitly
  left open in design review; embedding was chosen so "entire folders in
  one file" is actually true, accepting the size/complexity cost over a
  smaller but partial export.
- Soft-deleted clips (`is_deleted = 1`, i.e. trash) are excluded from
  export — this is a backup/migrate feature, not a trash-included dump.

**Export** — new command `export_data() -> String` (returns the JSON;
frontend writes it via a native Save dialog). Requires two new
dependencies not currently in this project, mirroring how the opener
plugin was added earlier: `@tauri-apps/plugin-dialog` (native
open/save file picker) and `@tauri-apps/plugin-fs` (the actual file
read/write once a path is chosen) — JS packages, matching Rust crates
(`tauri-plugin-dialog`, `tauri-plugin-fs`), plugin registration in
`main.rs`, and capability permissions in `capabilities/default.json`
(likely `dialog:default` and a scoped `fs:allow-write`/`fs:allow-read`,
exact permission names to be confirmed against the generated ACL schema
during implementation, the same way `core:window:allow-start-dragging`
was confirmed earlier this project).

**Import** — new command `import_data(json: String) -> ImportSummary`
(`{ foldersCreated, foldersMerged, clipsImported }`). For each folder in
the file: find-by-name-or-create. For each clip: insert into the resolved
folder, decoding `imageData` back to a file via
`clipboard::image_store` if present. Always additive — never deletes or
overwrites anything already in the destination database.

Existing, unrelated command `export_folder_clips` (human-readable
per-folder text dump, already shipped, used by the folder context menu)
is untouched and keeps its current name — the new `export_data`/
`import_data` pair is a different feature, not a rename.

## MCP (`mclip mcp`)

New tools alongside the existing eight (`list_clips`, `add_clip`,
`get_clip`, `remove_clip`, `pin_clip`, `list_folders`, `create_folder`,
`delete_folder`):

- `create_note(folder)` — same as New Card, callable from an AI client.
- `update_clip_content(id, content)` — same as the editor's autosave.
- `export_notes()` — returns the same JSON `export_data` produces.
- `import_notes(json)` — same as the Settings import.

## Testing

No existing frontend test framework in this project (confirmed earlier —
no `vitest`/`jest`, no `*.test.*` files), so this follows existing
convention: manual verification via `pnpm build` + a real `tauri dev` run
(New Card → type in editor → confirm autosave persists after closing and
reopening the panel; style vs. md copy into a plain-text field and a
rich-text field like TextEdit or Notes.app; export then import into a
second, empty database via the sandboxed-HOME technique used earlier this
project, confirming folders/clips/images round-trip). Rust side: `cargo
check` for compile correctness on every new command.

## Out of scope (not part of this pass)

- Conflict *resolution* UI for import (e.g., choosing per-item what to do
  on a name collision) — the additive/merge-by-name default is the only
  behavior for now.
- Any table/link/image support in the editor toolbar beyond what's listed
  above.
- Editing folder metadata (color/icon) from within the note editor.
