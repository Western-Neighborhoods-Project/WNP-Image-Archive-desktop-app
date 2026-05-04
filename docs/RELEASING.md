# Releasing

The app uses Tauri's auto-updater paired with a GitHub Actions workflow.
A push of a `v*` tag triggers a draft GitHub Release with signed
artifacts; installed apps fetch the latest manifest on boot and offer
the update via a native dialog.

## One-time setup

These steps need to be done before the first release. After this, every
release is just a tag push.

### 1. Generate updater signing keys

```sh
bun run tauri signer generate -- -w ~/.tauri/wnp-app.key
```

Set a passphrase when prompted. Two files are produced:

- `~/.tauri/wnp-app.key` — **private** key. Keep this secret. It signs
  every release artifact.
- `~/.tauri/wnp-app.key.pub` — **public** key. Embedded in the app so
  it can verify update signatures.

### 2. Embed the public key in `tauri.conf.json`

Open `src-tauri/tauri.conf.json` and replace the placeholder:

```json
"plugins": {
  "updater": {
    "endpoints": ["https://github.com/danielucas/wnp-app/releases/latest/download/latest.json"],
    "pubkey": "REPLACE_WITH_OUTPUT_OF_TAURI_SIGNER_GENERATE"
  }
}
```

Copy the contents of `~/.tauri/wnp-app.key.pub` (one long base64 string)
into the `pubkey` field. Commit the change.

### 3. Add the private key as a GitHub Actions secret

On github.com:

`https://github.com/danielucas/wnp-app/settings/secrets/actions` →
**New repository secret**

Add **two** secrets:

- `TAURI_SIGNING_PRIVATE_KEY` — paste the entire contents of
  `~/.tauri/wnp-app.key` (the private file, not the `.pub` one). It's
  a multi-line base64 blob; paste it verbatim.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — the passphrase you set when
  generating the key.

These are how the GitHub Actions runner signs each release.

### 4. (Optional) verify the release workflow runs locally

You can dry-run a build (without releasing) on your own machine:

```sh
export TAURI_SIGNING_PRIVATE_KEY=$(cat ~/.tauri/wnp-app.key)
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=your-passphrase
bun run tauri build -- --target universal-apple-darwin
```

Outputs land in `src-tauri/target/universal-apple-darwin/release/bundle/`.
The presence of `.app.tar.gz` and `.app.tar.gz.sig` next to the `.dmg`
confirms signing worked.

## Cutting a release

Each release is two commands plus a click on github.com.

### 1. Bump the version

Edit both files to the same version (semver, no leading `v`):

- `src-tauri/tauri.conf.json` → `"version": "0.1.1"`
- `package.json` → `"version": "0.1.1"`

### 2. Commit and tag

```sh
git commit -am "release: v0.1.1"
git tag v0.1.1
git push origin main v0.1.1
```

### 3. Wait for CI, then publish

The Actions workflow (`.github/workflows/release.yml`) runs on the
`v0.1.1` tag push. Takes ~10–15 minutes for a first build, less when
the cache is warm. When it finishes, a **draft** release appears at
`https://github.com/danielucas/wnp-app/releases`.

Open the draft, edit the release notes if you want, then click
**Publish release**.

### 4. Installed apps pick it up

On the next launch (or via "Check for updates…" in the user menu),
installed apps will:

1. Fetch `https://github.com/danielucas/wnp-app/releases/latest/download/latest.json`
2. Compare versions
3. If newer, prompt: *"Version 0.1.1 is available. Install now?"*
4. On confirm: download `.app.tar.gz`, verify signature, swap, relaunch

## What ships in each release

The workflow attaches these files to each release:

- `Image Archive Manager_x.y.z_universal.dmg` — installer for fresh installs
- `Image Archive Manager_x.y.z_universal.app.tar.gz` — update artifact
- `Image Archive Manager_x.y.z_universal.app.tar.gz.sig` — signature
- `latest.json` — manifest the in-app updater fetches

The `.dmg` is what you'd send to a fresh machine. The `.app.tar.gz` +
`.sig` + `latest.json` are what enable existing installs to update
themselves.

## Troubleshooting

**Build fails with "TAURI_SIGNING_PRIVATE_KEY env var is required":**
You haven't set the GitHub Actions secrets, or you're trying to build
locally without exporting them. See step 3 (CI) or "verify locally"
(local builds).

**App says "Failed to check for updates" on every launch:**
Either the `pubkey` in `tauri.conf.json` doesn't match the private key
used to sign the release, or the release hasn't been published yet
(only draft). Publish the draft on github.com.

**App says "Update verification failed":**
The `.sig` file's signature doesn't match the embedded `pubkey`. Most
likely you regenerated the keys and forgot to update the embedded
pubkey, OR the release was signed with a different key. Re-publish
with the correct key.

**Updater works on Apple Silicon but not Intel (or vice versa):**
The release was built for one arch, not universal. Check that the
workflow uses `--target universal-apple-darwin` (it does by default;
this would only happen if someone changed it).

## Bypassing Gatekeeper on first install

The auto-updater is independent of Apple code signing — updates work
even on unsigned builds. But the **first install** of an unsigned
build still triggers Gatekeeper. On the WNP machine:

1. Drag `.app` from the mounted `.dmg` to `/Applications`
2. **Right-click** → **Open** → confirm in the dialog
3. macOS remembers; future launches and auto-updates work transparently

If you ever want to skip the right-click step (e.g. shipping outside
your org), you'd need an Apple Developer ID Application certificate
($99/year) and notarization. The workflow doesn't do this today;
adding it would be ~20 lines in the YAML plus three more secrets.
