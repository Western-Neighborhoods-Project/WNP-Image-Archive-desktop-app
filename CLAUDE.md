# Image Archive Manager

macOS desktop app for the Western Neighborhoods Project photo archive. Tauri v2: Rust backend in `src-tauri/`, Svelte 5 (runes) + SvelteKit static frontend in `src/`.

## Package manager: bun

`bun.lock` is the only lockfile and the only known-good dependency tree. Use bun for every install and script. Never run pnpm or npm here — pnpm ≥10 silently reinstalls freshly-resolved (newer) versions before every script, and that drifted tree freezes the app at runtime (Grid.svelte's virtualizer `$effect` loops → `effect_update_depth_exceeded` → UI renders but nothing responds).

Recovery when the tree is poisoned (symptom above): `rm -rf node_modules pnpm-lock.yaml && bun install --frozen-lockfile`.

## Commands

- `bun run check` — svelte-check + `cargo test --lib`; the repo's definition of green.
- `bun run tauri dev` — the real desktop app. (`bun run dev` alone serves a frontend with no backend; login fails there.)
- `scripts/release.sh <X.Y.Z|patch|minor|major>` — cut a release from `main`; CI builds, signs, and publishes automatically.

## Sharp edges

- Dev and prod builds share one SQLite database: `~/Library/Application Support/org.wnp.imagearchive/archive_manager.db`. A dev run touches real data; run one instance at a time.
- Settings whose values are credentials must be listed in `SECRET_KEYS` (`src-tauri/src/settings.rs`) — that list is the only wall between editor accounts and secrets.
- Auth is fully local: users table with Argon2id hashes, session in RAM (`AppState.current_user`). A forgotten password is reset by writing a new hash into the DB.
- `src-tauri/tauri.conf.json` is the source of truth for the app version (Cargo.toml stays at 0.1.0; the release script bumps tauri.conf.json + package.json).
- If `cargo test` fails with a foreign-path permissions error, clear `src-tauri/target/debug/build` and rerun.
