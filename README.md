# Image Archive Manager

A macOS desktop application for browsing, searching, and managing a large image archive (50,000+ images). Built for history and archive organizations.

## Tech Stack

| Layer | Technology |
|---|---|
| Desktop framework | [Tauri 2](https://tauri.app) |
| Frontend | [SvelteKit 2](https://kit.svelte.dev) + [Svelte 5](https://svelte.dev) |
| Styling | [Tailwind CSS 4](https://tailwindcss.com) |
| Database | SQLite (via [rusqlite](https://github.com/rusqlite/rusqlite), bundled) |
| Image metadata | [ExifTool](https://exiftool.org) |
| Image processing | [image-rs](https://github.com/image-rs/image) |
| Virtual scrolling | [@tanstack/svelte-virtual](https://tanstack.com/virtual/v3) |

## Prerequisites

- **Rust** — [rustup.rs](https://rustup.rs)
- **Bun** — [bun.sh](https://bun.sh)
- **ExifTool** — `brew install exiftool`
- **Tauri system dependencies** — [tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/)

## Getting Started

```bash
# Install frontend dependencies
bun install

# Run in development mode (hot-reloads frontend + Rust)
bun run tauri dev

# Build a production .app bundle
bun run tauri build
```

## Project Structure

```
wnp-app/
├── src/                    # SvelteKit frontend
│   ├── routes/             # SvelteKit pages (+layout.svelte, +page.svelte)
│   ├── lib/
│   │   ├── commands/       # Typed Tauri invoke wrappers
│   │   ├── components/     # Svelte components (layout, browsing, setup)
│   │   ├── stores/         # Svelte stores (navigation, filters)
│   │   └── utils/          # Utilities (format, thumbnailQueue)
│   └── app.css             # Global styles (Tailwind entry point)
├── src-tauri/              # Rust/Tauri backend
│   ├── src/
│   │   ├── main.rs         # App entry point
│   │   ├── lib.rs          # Tauri builder, command registration
│   │   ├── db.rs           # SQLite initialization and migration
│   │   ├── models.rs       # Shared data types (serde)
│   │   ├── scanner.rs      # Directory scanning (walkdir)
│   │   ├── metadata.rs     # ExifTool integration
│   │   ├── thumbnails.rs   # Two-tier thumbnail system
│   │   ├── queries.rs      # Image query/filter logic
│   │   ├── collections.rs  # Collection queries
│   │   └── settings.rs     # App settings (key-value store)
│   ├── sql/schema.sql      # SQLite schema (embedded at compile time)
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/                   # Detailed documentation
│   ├── ARCHITECTURE.md
│   ├── RUST-COMMANDS.md
│   ├── DATABASE.md
│   ├── COMPONENTS.md
│   ├── DEVELOPMENT.md
│   └── IMPORT.md
└── _project-specs/         # Original project specifications
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md) — System design, data flows, thumbnail strategy
- [Rust Commands](docs/RUST-COMMANDS.md) — All backend commands with types and examples
- [Database Schema](docs/DATABASE.md) — Full schema with descriptions
- [Components](docs/COMPONENTS.md) — All Svelte components
- [Development Guide](docs/DEVELOPMENT.md) — Setup, debugging, common issues
- [Import System](docs/IMPORT.md) — ExifTool integration, adapter pattern

## Development Status

- **Phase 1** ✅ Project setup, directory scanning, metadata extraction, thumbnails, virtual-scrolling grid
- **Phase 2** 🔲 Metadata editing, search, filtering
- **Phase 3** 🔲 Collections management
- **Phase 4** 🔲 Export & sharing
- **Phase 5** 🔲 Keyboard navigation & polish
- **Phase 6** 🔲 Backup & operations
