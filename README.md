<div align="center">

<img src="banner.png" width="100%" alt="MonoClip — Your clipboard, finally under control"/>

<br/><br/>

**A blazing-fast, macOS-native clipboard manager that lives in your menu bar and gets out of your way.**

<br/>

[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-black?style=flat-square)](https://github.com/nokhodian/mono-clip/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-FFC131?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app)
[![Svelte](https://img.shields.io/badge/frontend-Svelte%205-FF3E00?style=flat-square&logo=svelte&logoColor=white)](https://svelte.dev)
[![Rust](https://img.shields.io/badge/backend-Rust-CE422B?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-6366f1?style=flat-square)](LICENSE)
[![Stars](https://img.shields.io/github/stars/nokhodian/mono-clip?style=flat-square&color=6366f1)](https://github.com/nokhodian/mono-clip/stargazers)

<br/><br/>

<img src="screenshot.png" width="680" alt="MonoClip app screenshot"/>

<br/><br/>

</div>

---

## ✨ Why MonoClip?

You know that feeling when you copy something important, then copy something else, and the first thing is **gone forever**? Yeah. That ends today.

MonoClip sits quietly in your menu bar, **remembers everything you copy**, and lets you retrieve any past clip in under a second. No subscriptions. No cloud. No nonsense. Just your clipboard, supercharged.

<br/>

## 🚀 Features

<table>
<tr>
<td width="50%">

### 📁 Smart Folders
Create custom folders for anything — *Code Snippets*, *Email Templates*, *Links*, *Passwords*. Your clips, your structure.

### ⌨️ Global Shortcut Routing
Assign a hotkey to any folder. Press it and whatever you have selected (or in your clipboard) flies straight into that folder. No window, no friction.

### 🔍 Instant Search
Type to filter across all your clips instantly. Find that thing you copied six weeks ago in under a second.

### 🖼️ Images & Files
Copy an image — see the thumbnail. Copy a file or folder in Finder — get the full path. MonoClip captures it all.

</td>
<td width="50%">

### 🎨 macOS-Native Design
Glass-morphism floating panel. Frosted blur. Spring animations. It looks like it belongs on your Mac because it was built *for* your Mac.

### 📌 Pin Important Clips
Some things you need forever. Pin them. They stay safe even when auto-cleanup runs.

### 🧹 Auto-Cleanup
Set it and forget it. MonoClip automatically removes old unpinned clips to keep your history lean.

### 🤖 AI-Ready CLI
`mclip` ships with the app. Use it in your terminal, pipe clips into commands, or give your AI assistant instant access to your clipboard history.

</td>
</tr>
</table>

<br/>

## 🎬 See It In Action

```
1. Copy anything                    ⌘C
2. Copy something else              ⌘C
3. Open MonoClip                    ⌘⇧V
4. Pick what you actually wanted    ↑↓ + Enter
5. It's pasted                      ✨
```

> **With folder shortcuts:** Select text in any app → press your shortcut → it's saved. Done. You never even had to open MonoClip.

<br/>

## 📦 Installation

### Option A — Homebrew *(recommended)*

```bash
brew tap nokhodian/tap
brew install --cask mono-clip
```

That's it. Homebrew handles everything — download, verify checksum, install the `.app`.

### Option B — Direct Download

Grab the latest `.dmg` from the [Releases page](https://github.com/nokhodian/mono-clip/releases):

1. Download `MonoClip_0.2.2_aarch64.dmg`
2. Open the `.dmg` and drag **MonoClip.app** to `/Applications`
3. Launch from Spotlight or `/Applications`

> **Apple Silicon only** — the current release targets `aarch64`. Intel builds coming soon.

### Option C — Linux

Three artifacts ship for `x86_64` on the [Releases page](https://github.com/nokhodian/mono-clip/releases):

```bash
# AppImage — portable, self-updating, no install
chmod +x MonoClip_0.2.11_amd64.AppImage
./MonoClip_0.2.11_amd64.AppImage

# Debian / Ubuntu
sudo apt install ./MonoClip_0.2.11_amd64.deb

# Fedora / RHEL / openSUSE
sudo rpm -i MonoClip-0.2.11-1.x86_64.rpm
```

> **Pick the AppImage if you want in-app updates.** deb and rpm installs are owned by your package manager, so MonoClip won't overwrite them — it points you at the release page instead.

The tray icon needs an AppIndicator host. GNOME users: install the [AppIndicator extension](https://extensions.gnome.org/extension/615/appindicator-support/); most other desktops (KDE, XFCE, Cinnamon) support it out of the box.

**Known Linux limitations:**

| Feature | Status |
|---|---|
| Clipboard history, folders, search, pinning | ✅ Works |
| Master (open/close) shortcut, on X11 | ✅ Works |
| Master shortcut, on Wayland | ⚠️ Uses the [XDG `GlobalShortcuts` portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html) — works if your desktop's portal backend implements it (GNOME 48+, KDE Plasma 6.4+); not yet on COSMIC ([xdg-desktop-portal-cosmic](https://github.com/pop-os/xdg-desktop-portal-cosmic) doesn't implement this interface as of writing). The tray icon always works as a fallback. |
| Folder shortcuts | ✅ On X11 — same Wayland limitation as the master shortcut applies (not yet ported to the portal) |
| Auto-paste after picking a clip | ❌ Clip is copied; paste it yourself with `Ctrl+V` |
| Capture *selected* text via folder shortcut | ❌ Falls back to current clipboard contents |

### Option D — Build from Source

**Prerequisites:**

| Tool | Version | Install |
|---|---|---|
| Rust | 1.77+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js | 18+ | [nodejs.org](https://nodejs.org) or `brew install node` |
| pnpm | latest | `npm i -g pnpm` |
| Xcode CLT | *(macOS)* | `xcode-select --install` |
| System libs | *(Linux)* | see below |

**Linux system dependencies** (Debian/Ubuntu — adapt for your distro):

```bash
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
  librsvg2-dev libxdo-dev libssl-dev build-essential patchelf file \
  desktop-file-utils fuse3
```

```bash
git clone https://github.com/nokhodian/mono-clip.git
cd mono-clip
pnpm install
cargo tauri dev           # dev mode
# — OR —
cargo tauri build         # → src-tauri/target/release/bundle/
```

> 💡 The first build takes a few minutes while Rust compiles all dependencies.

<br/>

## 🛠️ Troubleshooting

### "MonoClip is damaged and can't be opened"

<img src=".github/assets/gatekeeper-error.png" width="360" alt="macOS Gatekeeper damaged app error"/>

macOS shows this warning for apps that aren't signed with an Apple Developer certificate. It's a Gatekeeper quarantine flag — the app is not actually damaged. Fix it with two commands in Terminal:

```bash
find /Applications/MonoClip.app -print0 | xargs -0 xattr -c
codesign --force --deep --sign - /Applications/MonoClip.app
```

Then open MonoClip normally from `/Applications` or Spotlight.

---

### `⌘⇧V` shortcut doesn't open MonoClip

MonoClip needs **Accessibility** permission to detect global keypresses. Open MonoClip, go to **Settings → Permissions → Accessibility** and click **Open Settings →**, then add MonoClip and enable the toggle.

---

### "Launch at Login" shows under Background Items, not Open at Login

This is correct — menu bar apps that run without a Dock icon appear under **Background Items** in System Settings → General → Login Items. MonoClip will still launch automatically on login.

<br/>

## 🔑 Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `⌘⇧V` | Open / close MonoClip |
| `↑` `↓` | Navigate clips |
| `Enter` | Copy selected clip (+ auto-paste) |
| `P` | Pin / unpin selected clip |
| `⌫` | Delete selected clip |
| `⌘F` or `/` | Focus search |
| `Esc` | Close window |
| `⌘⌥1` *(custom)* | Save clipboard → folder |

<br/>

## 📁 Setting Up Folder Shortcuts

1. Open MonoClip (`⌘⇧V`)
2. Click **+ New Folder** in the sidebar
3. Name it, pick an emoji and color
4. Click the **shortcut field** and press your combo (e.g. `⌘⌥1`)
5. Save — that's it!

Now, whenever you have text **selected** in any app (or just something in your clipboard), press your shortcut and it's saved to that folder instantly.

> **Pro tip:** MonoClip is smart about selection. If you have text highlighted, it captures *that* — not whatever happens to be in your clipboard. Perfect for grabbing snippets while reading docs.

<br/>

## 🖥️ mclip — Terminal CLI

`mclip` installs automatically with the app. Set it up once:

```bash
# In Settings → Install mclip CLI
# Then add to your shell profile:
export PATH="$HOME/.local/bin:$PATH"
```

Then use it anywhere:

```bash
mclip list                          # recent inbox items
mclip list --folder Work            # specific folder
mclip list --search http            # search across clips
mclip add "text"                    # add a clip
mclip add "text" --folder Work      # add to a folder
mclip get <id>                      # print raw content
mclip get <id> | pbcopy             # pipe back to clipboard
mclip remove <id>                   # delete a clip
mclip pin <id>                      # pin a clip
mclip folder list                   # list all folders
mclip folder add "Name"             # create a folder
mclip folder remove "Name"          # delete a folder
```

<br/>

## 🤖 Use with AI

MonoClip gives your AI coding assistant full access to your clipboard history.

### Copy context (works with any AI)

Open **Help → Use with AI** in the app and click **Copy AI Context**. Paste it into any chat window — Claude, Cursor, ChatGPT — or save it to `CLAUDE.md` / `.cursorrules`. The AI will then understand all `mclip` commands and can manage your clipboard on your behalf.

```bash
# Or from the terminal:
mclip context
```

### MCP server (Claude Desktop, Cursor, Windsurf)

`mclip mcp` starts a JSON-RPC stdio server that exposes your clipboard as native AI tools — no copy-paste needed.

Add this to `~/.config/claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "mclip": {
      "command": "mclip",
      "args": ["mcp"]
    }
  }
}
```

Available tools: `list_clips`, `add_clip`, `get_clip`, `remove_clip`, `pin_clip`, `list_folders`, `create_folder`, `delete_folder`.

<br/>

## 🏗️ Tech Stack

```
┌─────────────────────────────────────────────────────┐
│                   MonoClip v0.2                     │
├───────────────────┬─────────────────────────────────┤
│  Frontend         │  Svelte 5 (runes) + Vite        │
│  Styling          │  Tailwind CSS 3                  │
│  App Framework    │  Tauri 2                         │
│  Backend          │  Rust                            │
│  Database         │  SQLite (rusqlite, WAL mode)     │
│  Clipboard        │  tauri-plugin-clipboard-manager  │
│  Shortcuts        │  tauri-plugin-global-shortcut    │
│  Autostart        │  tauri-plugin-autostart          │
│  CLI              │  clap + rusqlite (standalone)    │
└───────────────────┴─────────────────────────────────┘
```

**Why Tauri over Electron?**

| | Tauri | Electron |
|---|---|---|
| Binary size | ~8 MB | ~150 MB |
| RAM usage | ~30 MB | ~300 MB |
| Startup time | < 200ms | ~2 seconds |
| Backend language | Rust 🦀 | Node.js |
| Native feel | ✅ | Mostly |

<br/>

## 🗂️ Project Structure

```
mono-clip/
├── src/                          # Svelte frontend
│   ├── App.svelte                # Root shell + event listeners
│   ├── lib/
│   │   ├── api/tauri.ts          # Typed Tauri command wrappers
│   │   ├── components/           # UI components
│   │   │   ├── ClipCard.svelte
│   │   │   ├── Sidebar.svelte
│   │   │   ├── SearchBar.svelte
│   │   │   ├── HelpPanel.svelte
│   │   │   ├── SettingsPanel.svelte
│   │   │   └── ...
│   │   └── stores/               # Svelte 5 rune-based state
│   └── app.css                   # Tailwind + CSS vars
│
└── src-tauri/                    # Rust backend
    └── src/
        ├── main.rs               # App entry, plugin setup
        ├── db/                   # SQLite: models, queries, migrations
        ├── clipboard/            # Background watcher + type detection
        ├── commands/             # Tauri IPC commands
        ├── shortcuts/            # Global shortcut manager
        ├── tray/                 # Menu bar tray
        ├── window/               # Window positioning
        └── bin/mclip.rs          # Standalone CLI binary
```

<br/>

## 🔐 Privacy

MonoClip is **100% local**. Your clipboard data:

- ✅ Stored only in `~/.monoclip/monoclip.db` on your machine
- ✅ Never sent anywhere, ever
- ✅ No analytics, no telemetry, no accounts
- ✅ You can delete everything by deleting that one file

**macOS Permissions required:**
- **Accessibility** — for simulating paste (`⌘V`) after you copy a clip
- **Input Monitoring** — for global keyboard shortcuts

Both are requested via standard macOS system dialogs on first use.

<br/>

## 🛣️ Roadmap

- [ ] Rich text + HTML clip support
- [ ] iCloud sync (opt-in)
- [ ] Clip templates / snippets with variables
- [ ] Multiple window themes
- [ ] Plugin system
- [x] Image clipboard capture with thumbnails
- [x] File & folder path capture
- [x] mclip CLI with AI context + MCP server
- [x] Multi-folder organization
- [x] Global shortcut routing
- [x] Selected text capture
- [x] Auto-cleanup
- [x] Keyboard-first navigation
- [x] Glass-morphism UI

<br/>

## 🤝 Contributing

PRs welcome! Here's how to get started:

```bash
git clone https://github.com/nokhodian/mono-clip.git
cd mono-clip
pnpm install
cargo tauri dev
```

Please open an issue first for large changes so we can discuss the approach.

<br/>

## 📄 License

MIT © [nokhodian](https://github.com/nokhodian)

---

<div align="center">

**If MonoClip saves you even one frustrated `⌘Z` a day, consider giving it a ⭐**

Made with 🦀 Rust, ❤️ and too much coffee.

</div>
