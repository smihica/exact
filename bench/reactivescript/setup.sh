#!/usr/bin/env bash
# setup.sh [dir]: clones ReactiveScript at the commit these results used, applies harness.patch (the contract arm and
# the desk9 reference in Contract), and installs the eval harness's packages and Playwright's Chromium.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
DIR=${1:-$HOME/bench-rx/harness}
UPSTREAM=https://github.com/solidjs/reactivescript
BASE=08255de688d8eb4265ed5221d226295f73163d74
[ -e "$DIR" ] && { echo "$DIR exists; name another directory" >&2; exit 1; }
git clone -q "$UPSTREAM" "$DIR"
git -C "$DIR" checkout -q -b exact "$BASE"
git -C "$DIR" -c user.name=bench -c user.email=bench@localhost am -q --committer-date-is-author-date "$HERE/harness.patch"
(cd "$DIR/eval" && npm install --no-audit --no-fund && npx playwright install chromium)
echo "harness ready at $DIR"
