# Image Archive Manager

A macOS desktop application for cataloging, searching, and managing
large historical image archives. Built to handle 50,000+ images
across multiple source directories with metadata, full-text search,
collections, and a workflow for fulfilling external image requests.

> **Status:** Pre-1.0. Core features are in place but the app hasn't
> yet seen long-term real-world use. Releases ship fast via the
> built-in auto-updater.

## What it does

- **Index a directory tree of image files.** Recursive scan picks up
  JPEG / PNG / TIFF / GIF / BMP / WebP under one or more source
  directories you point it at.
- **Edit metadata in place.** Title, description, location, dates,
  photographer, donor, keywords. Changes get written back to the
  files via ExifTool and tracked in an audit log.
- **Find images quickly.** Full-text search across title /
  description / keywords / catalog number, plus filters on city,
  photographer, year range, and missing metadata. Smart Collections
  save filter presets for one-click recall.
- **Organize.** Manual user collections, plus an automatic
  source-directory tree that mirrors your file-system layout
  (collapse / expand, click to scope).
- **Share images with external recipients.** Ad-hoc share links that
  resize, upload to S3-compatible storage, and email a download
  link via your existing mail provider.
- **Fulfill image-use requests.** Integrates with OpenSFHistory's
  image-request API: fetch incoming requests, resize per-tier, ship
  a zip to S3, post completion back.
- **Multi-user with roles.** Admin (full access) and editor
  (everything except settings + user management). Argon2id password
  hashing, login rate limiting, inactivity timeout.
- **File watcher.** Drop new images into a source directory and they
  appear in the catalog within seconds — thumbnails and metadata
  generate in the background.
- **Live progress indicator.** Footer pill shows how many images
  are still being processed; a popover surfaces per-file errors with
  a "retry all" button.
- **Auto-updates.** New releases install on next launch (or via
  the user menu) with a native confirm dialog. No manual reinstall.

## Installation

Download the latest `.dmg` from
[Releases](https://github.com/danielucas/WNP-Image-Archive-desktop-app/releases) and drag
the app to `/Applications`.

The app isn't currently signed with an Apple Developer ID, so the
first launch needs a Gatekeeper bypass. The simplest path is one
terminal command:

```sh
xattr -d com.apple.quarantine "/Applications/Image Archive Manager.app"
```

After that, double-click to open. Future launches and auto-updates
work normally.

You'll also need [ExifTool](https://exiftool.org) installed for
metadata extraction:

```sh
brew install exiftool
```

## First-run setup

1. Launch the app. The first run prompts you to create an admin
   account (username + password, 12+ characters).
2. Pick the directory containing your image archive. The scan runs
   immediately and routes you to the library.
3. The footer pill shows progress as thumbnails and metadata fill in
   over the next few minutes (depending on archive size).

Multiple source directories can be added later from
**Settings → General**.

## Updates

The app checks for updates on every launch (silent unless one is
available). When a new release is published on GitHub, you'll see a
native dialog: *"Version X is available. Install now?"*

You can also trigger a manual check from the user menu in the
sidebar bottom-left.

The current version is shown in **Settings → General**. Release
notes for each version live on the [Releases
page](https://github.com/danielucas/WNP-Image-Archive-desktop-app/releases).

## Architecture (high level)

- **Tauri 2** desktop shell with a Rust backend and a SvelteKit 2 +
  Svelte 5 frontend.
- **SQLite** (bundled, in-memory tests) for catalog data.
- **ExifTool** for metadata extraction; **image-rs** for thumbnail
  generation; **AWS SDK** for S3-compatible uploads.
- **notify** for file-system watching across registered source
  directories.
- A background worker thread handles thumbnail + metadata generation
  with bounded parallelism so the library stays responsive while
  the queue drains.

## For developers

If you want to run it locally or contribute changes:

```sh
# Prereqs:
brew install exiftool
curl -fsSL https://bun.sh/install | bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and run:
git clone https://github.com/danielucas/WNP-Image-Archive-desktop-app
cd WNP-Image-Archive-desktop-app
bun install
bun run tauri dev
```

Tech stack: Tauri 2 + SvelteKit 2 + Svelte 5 + Rust + SQLite (bundled),
plus ExifTool for metadata, image-rs for thumbnails, AWS SDK for S3
uploads, and `notify` for filesystem watching. Frontend styled with
Tailwind 4. The `src-tauri/src/` modules are organized by feature
(scanner, watcher, background_jobs, source_directories, etc.) and
each is fairly self-contained.

## License

MIT — see [LICENSE](LICENSE).
