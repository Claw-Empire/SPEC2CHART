#!/bin/bash
# .claude/post-update.sh
# Called by Claude Code Stop hook after each session turn.
# Rebuilds the local app if sources changed, commits any pending changes, then pushes.

set -uo pipefail
cd /Users/joe888777/Desktop/project/experiment/openAtlas

BINARY=/Applications/openDraftly.app/Contents/MacOS/open-draftly

# 1. Rebuild if any .rs source or Cargo.toml is newer than the installed binary
NEEDS_BUILD=false
if [ ! -f "$BINARY" ]; then
    NEEDS_BUILD=true
elif find src/ Cargo.toml -newer "$BINARY" 2>/dev/null | grep -q .; then
    NEEDS_BUILD=true
fi

if [ "$NEEDS_BUILD" = "true" ]; then
    echo "[post-update] Source changed — cargo build --release..."
    if cargo build --release 2>&1; then
        cp target/release/open-draftly "$BINARY"
        echo "[post-update] Installed to $BINARY"
    else
        echo "[post-update] Build FAILED — skipping install"
        exit 1
    fi
fi

# 2. Commit any pending changes (src, docs, skill updates, etc.)
if [ -n "$(git status --porcelain 2>/dev/null)" ]; then
    git add -A
    MSG="chore: auto-update $(TZ=Asia/Shanghai date '+%Y-%m-%d %H:%M')"
    git commit -m "$MSG"
    echo "[post-update] Committed: $MSG"
fi

# 3. Push if ahead of remote
if git rev-parse "@{u}" >/dev/null 2>&1; then
    AHEAD=$(git rev-list "@{u}..HEAD" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$AHEAD" -gt "0" ]; then
        git push && echo "[post-update] Pushed $AHEAD commit(s)." || echo "[post-update] Push failed (check remote)"
    fi
else
    echo "[post-update] No upstream branch — skipping push"
fi

echo "[post-update] Done."
