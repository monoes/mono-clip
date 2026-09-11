# Releasing MonoClip

A release isn't done when the tag is pushed — it's done when **every channel a
user might install from actually serves the new version.** The v0.2.16 release
sat invisible for two days, and the Homebrew cask was 7 versions stale before
anyone noticed. Both were silent failures: nothing errored, nothing showed up
in the workflow logs as red. This checklist exists so that doesn't happen again.

## 1. Bump the version — in *every* place it appears

- `src-tauri/tauri.conf.json` → `version`
- `src-tauri/Cargo.toml` → `version`
- `README.md` — **do not just bump the Linux code block and assume you got
  everything.** Search the whole file:
  ```bash
  grep -n "MonoClip[-_]0\.[0-9]*\.[0-9]*" README.md
  ```
  There are at least two independent version strings in here (the Linux
  install block, and a separate macOS direct-download line) that don't share
  a single source of truth. A `sed` that only matches the *previous* version
  string will silently skip any line that was already stale for a different
  reason — which is exactly how the macOS line drifted 14 versions behind
  before anyone caught it.
- Rebuild the `mclip` sidecar after bumping (`pnpm build:mclip`), which also
  syncs `Cargo.lock`'s version entry. Without this, the CLI bundled in the
  release reports the old version from `mclip --version`.

The in-app version display (Settings panel, `Help` footer) reads
`getVersion()` from Tauri at runtime — no manual step needed there, it's
correct automatically once `tauri.conf.json` is bumped and rebuilt.

## 2. Commit, tag, push

```bash
git add <the files above>
git commit -m "chore: bump version to X.Y.Z"
git tag vX.Y.Z
git push origin main
git push origin vX.Y.Z
```

Pushing the tag triggers `.github/workflows/release.yml`, which builds all
three platforms and, once every leg of the matrix succeeds, **automatically
publishes the release** (added after v0.2.16 shipped as an invisible draft
for two days — see the `publish` job, which runs after `build` and flips
`releaseDraft` off). Confirm anyway rather than trusting the green
checkmark blindly:

```bash
gh run list --workflow=release.yml --limit 1
gh release view vX.Y.Z --json isDraft,publishedAt   # isDraft must be false
```

If `isDraft` is still `true` (workflow change didn't fire, one leg failed,
etc.), publish it by hand: `gh release edit vX.Y.Z --draft=false`.

## 3. Update the Homebrew cask — separate repo, does not auto-update

The cask lives in [`monoes/homebrew-tap`](https://github.com/monoes/homebrew-tap)
(`Casks/mono-clip.rb`), a **different repository** with no automation tying it
to mono-clip's own releases. Nothing here happens unless you do it by hand:

```bash
# Get the real checksum from the published asset — don't trust any
# API-reported digest without cross-checking; a wrong sha256 breaks the
# install for every Homebrew user with zero warning until they try it.
gh release download vX.Y.Z --repo monoes/mono-clip --pattern "*aarch64.dmg" --dir /tmp
shasum -a 256 /tmp/MonoClip_X.Y.Z_aarch64.dmg
```

Update `version` and `sha256` in the cask, commit, push to
`monoes/homebrew-tap`. Then verify for real, not just by eyeballing the diff:

```bash
brew fetch --cask nokhodian/tap/mono-clip   # downloads + validates the checksum
```

A `✔︎ Cask mono-clip (X.Y.Z)` line means it will actually install correctly.
Anything else means the version/checksum/URL don't line up — fix it before
calling the release done, since Homebrew is the README's *recommended*
(Option A) install path.

## 4. Sanity-check the other channels

- **Linux (deb/rpm/AppImage)** pull directly from whatever's on the GitHub
  Releases page, so once step 2 is confirmed published, these are already
  current — no separate pinning to go stale.
- **Direct DMG download** — same: current the moment the release is published.

## Why this matters

Users don't all install the same way. A release that's "done" on GitHub but
still serves an old build via Homebrew, or whose README tells people to
download a filename from three versions ago, is not actually released to
everyone — it just looks that way from the maintainer's side. Treat "is
every channel actually serving the new version" as part of the release, not
a follow-up task.
