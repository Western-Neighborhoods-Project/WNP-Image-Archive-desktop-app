#!/usr/bin/env bash
# Cut a release of Image Archive Manager entirely from the terminal.
#
# Usage: scripts/release.sh <X.Y.Z | patch | minor | major>
#
# Bumps the version in src-tauri/tauri.conf.json + package.json, commits,
# tags vX.Y.Z, and pushes. The Release workflow then builds, signs, and
# PUBLISHES the GitHub release automatically — installed apps pick it up
# on their next update check. No GitHub UI step required.

set -euo pipefail
cd "$(dirname "$0")/.."

die() { echo "release.sh: $*" >&2; exit 1; }

[[ $# -eq 1 ]] || die "usage: scripts/release.sh <X.Y.Z | patch | minor | major>"

current=$(node -p "require('./src-tauri/tauri.conf.json').version")

case "$1" in
  patch|minor|major)
    IFS=. read -r maj min pat <<<"$current"
    case "$1" in
      patch) version="$maj.$min.$((pat + 1))" ;;
      minor) version="$maj.$((min + 1)).0" ;;
      major) version="$((maj + 1)).0.0" ;;
    esac
    ;;
  *) version="$1" ;;
esac

[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "not a semver version: $version"
tag="v$version"

# ── Preflight ───────────────────────────────────────────────────────────────
branch=$(git rev-parse --abbrev-ref HEAD)
[[ "$branch" == "main" ]] || die "releases are cut from main (currently on $branch)"
[[ -z "$(git status --porcelain)" ]] || die "working tree not clean"
git fetch -q origin main
[[ "$(git rev-parse HEAD)" == "$(git rev-parse origin/main)" ]] \
  || die "main is not in sync with origin/main — pull or push first"
git rev-parse -q --verify "refs/tags/$tag" >/dev/null && die "tag $tag already exists locally"
git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1 && die "tag $tag already exists on origin"

echo "Releasing $current -> $version"

# ── Checks (svelte-check + cargo tests) ─────────────────────────────────────
bun run check

# ── Bump, commit, tag, push ─────────────────────────────────────────────────
node - "$version" <<'EOF'
const fs = require('fs');
const version = process.argv[2];
for (const f of ['src-tauri/tauri.conf.json', 'package.json']) {
  const j = JSON.parse(fs.readFileSync(f, 'utf8'));
  j.version = version;
  fs.writeFileSync(f, JSON.stringify(j, null, 2) + '\n');
}
EOF

git add src-tauri/tauri.conf.json package.json
git commit -m "Bump version to $version"
git tag "$tag"
git push origin main "$tag"

echo
echo "Pushed $tag. CI is building and will publish the release automatically"
echo "(universal .dmg + updater artifacts; takes ~15 minutes)."
echo
echo "Watch it:  gh run watch"
echo "  or:      https://github.com/Western-Neighborhoods-Project/WNP-Image-Archive-desktop-app/actions"
