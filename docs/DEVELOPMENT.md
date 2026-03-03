# Development Guide

## Prerequisites

### macOS

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Install Bun
curl -fsSL https://bun.sh/install | bash
source ~/.zshrc  # or restart terminal

# Install ExifTool
brew install exiftool

# Install Tauri prerequisites (Xcode command line tools)
xcode-select --install
```

## Running in Development

```bash
# Install frontend dependencies (first time)
bun install

# Start dev server with hot reload (Rust + frontend)
bun run tauri dev
```

This opens the app window. Frontend changes hot-reload instantly. Rust changes trigger a Rust recompile (takes 5-30 seconds depending on what changed).

## Building for Production

```bash
bun run tauri build
```

Output: `src-tauri/target/release/bundle/macos/Image Archive Manager.app`

## Testing with Real Images

1. Launch `bun run tauri dev`
2. The setup screen appears (first launch)
3. Click "Select Directory" → pick a folder with images
4. Click "Start Import"
5. Wait for the three-stage import to complete
6. Browse the grid

**Tip:** For quick testing, use a small folder with 50–200 images. For performance testing, use the real drive with 50K+ images.

## Resetting the Catalog

Use the Settings view (gear icon in sidebar) → "Change Source Directory" → confirm reset. Or directly via the SQLite file:

```bash
# Find the DB location
ls ~/Library/Application\ Support/org.wnp.imagearchive/

# Open with sqlite3
sqlite3 ~/Library/Application\ Support/org.wnp.imagearchive/archive_manager.db
.tables
SELECT COUNT(*) FROM images;
```

## Debugging

### Tauri DevTools

In development, right-click anywhere in the app window and select "Inspect Element" to open WebKit DevTools.

### Rust Logs

Rust `println!` / `eprintln!` output appears in the terminal where you ran `bun run tauri dev`.

Enable structured logging by adding `env_logger` (future enhancement).

### SQLite Inspection

```bash
sqlite3 ~/Library/Application\ Support/org.wnp.imagearchive/archive_manager.db

# Useful queries
SELECT COUNT(*) FROM images;
SELECT COUNT(*) FROM images WHERE thumbnail_path IS NOT NULL;
SELECT COUNT(*) FROM images WHERE thumbnail_generated = 1;
SELECT * FROM app_settings;
SELECT name, source, COUNT(ci.image_id) FROM collections c
  LEFT JOIN collection_images ci ON ci.collection_id = c.id
  GROUP BY c.id;
```

### ExifTool Testing

```bash
# Test ExifTool on a single file
exiftool -json -fast2 /path/to/image.jpg

# Extract EXIF thumbnail
exiftool -b -ThumbnailImage /path/to/image.jpg > /tmp/thumb.jpg
open /tmp/thumb.jpg
```

## Common Issues

### "Failed to run exiftool"
ExifTool is not on PATH. Install it: `brew install exiftool`. Or set the `exiftool_path` key in `app_settings`.

### Thumbnails not loading
Check the thumbnail cache directory exists: `ls ~/Library/Application\ Support/org.wnp.imagearchive/thumbnails/`

If empty, re-run the import. The asset protocol in `tauri.conf.json` must have `assetProtocol.enable = true` and `scope = ["**"]`.

### Slow initial import
Normal for large archives. Metadata extraction runs ExifTool on every file in one pass (fast). EXIF thumbnail extraction runs ExifTool once per file (slower). Full thumbnail generation happens lazily during browsing.

### Rust compile errors after schema changes
The schema SQL is embedded at compile time via `include_str!()`. After editing `schema.sql`, run `cargo check` to verify and `bun run tauri dev` to rebuild.

## Project Structure Reference

```
src-tauri/src/
  main.rs        — Entry point (calls lib::run())
  lib.rs         — Tauri builder, all commands registered here
  db.rs          — DB init, path resolution, migration runner
  models.rs      — All serde structs shared between frontend and backend
  scanner.rs     — walkdir scanner, archive collection creation
  metadata.rs    — ExifTool subprocess, JSON parsing, pluggable adapters
  thumbnails.rs  — EXIF extraction + Lanczos3 resizing
  queries.rs     — query_images (paginated, filtered) + get_image
  collections.rs — get_collections command
  settings.rs    — get_setting, set_setting, reset_catalog

src/lib/
  commands/      — TypeScript wrappers for every Tauri command
  components/    — Svelte components organized by feature
  stores/        — Svelte stores (navigation, filters)
  utils/         — format.ts, thumbnailQueue.ts
```
