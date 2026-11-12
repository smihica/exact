#!/usr/bin/env bash
# run.sh: one round. Each desk is scaffolded from the brief, built by a fresh Claude Code agent (`claude -p`, run in
# the desk), then graded by the harness's grade.sh. Overrides:
#   ARMS (default "contract react"), FRONTIER (runs per arm on Opus, default 3), SMALL (runs per arm on Haiku,
#   default 6), PARALLEL (agents at once, default 3), EXACT2_REF (default origin/main), HARNESS, RUNS.
# Contract desks are made from a `git archive` of EXACT2_REF without bench/, so no agent can read this reference or
# its history; the archive is kept under $RUNS/../exact2-<sha> and reused.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
EXACT2_DIR=$(cd "$HERE/../.." && pwd)
HARNESS=${HARNESS:-$HOME/bench-rx/harness} RUNS=${RUNS:-$HOME/bench-rx/runs}
ARMS=${ARMS:-contract react} FRONTIER=${FRONTIER:-3} SMALL=${SMALL:-6} PARALLEL=${PARALLEL:-3}
export EXACT_FEEDBACK=0

git -C "$EXACT2_DIR" fetch -q
SHA=$(git -C "$EXACT2_DIR" rev-parse --short=10 "${EXACT2_REF:-origin/main}")
export EXACT2=$(dirname "$RUNS")/exact2-$SHA
if [ ! -e "$EXACT2/package.json" ]; then
  mkdir -p "$EXACT2"
  git -C "$EXACT2_DIR" archive "$SHA" | tar -x -C "$EXACT2"
  rm -rf "$EXACT2/bench"
  (cd "$EXACT2" && bun install --frozen-lockfile)
fi

ROUND=$RUNS/$(date +%Y-%m-%dT%H%M)
mkdir -p "$ROUND"
echo "exact2 $SHA, harness $(git -C "$HARNESS" rev-parse --short HEAD)" > "$ROUND/round.txt"

# Desks are numbered per arm; a desk's tier is in its `tier` file.
desks=()
for arm in $ARMS; do
  n=0
  for tier in opus haiku; do
    count=$([ "$tier" = opus ] && echo "$FRONTIER" || echo "$SMALL")
    for _ in $(seq 1 "$count"); do
      n=$((n + 1)); d=$ROUND/desk9-$arm-$n
      flags=(--brief plain); [ "$arm" = contract ] && flags+=(--inline-docs)
      (cd "$HARNESS/eval" && node harness/scaffold.mjs desk9 "$arm" "$d" "${flags[@]}" >/dev/null)
      echo "$tier" > "$d/tier"
      desks+=("$d")
    done
  done
done

# The first contract build compiles the archive's Contract tools; do it once, before agents race to. Not through
# ./check, which would count against the desk's agent in its .runlog.
for d in "${desks[@]}"; do case $d in *-contract-*) (cd "$d" && bun exact.mjs contract build app.contract >/dev/null 2>&1 || true); break;; esac; done

agent() {
  local d=$1
  (cd "$d" && claude -p "$(cat .prompt)" --model "$(cat tier)" --output-format json --dangerously-skip-permissions \
    --strict-mcp-config --disallowedTools WebSearch WebFetch > agent.json 2> agent.err) || true
  (cd "$HARNESS/eval" && sh harness/grade.sh "$d" "$d/grade.txt")
  echo "done $(basename "$d")"
}
export -f agent
export HARNESS
printf '%s\n' "${desks[@]}" | xargs -P "$PARALLEL" -I{} bash -c 'agent "$@"' _ {}

node "$HERE/report.mjs" "$ROUND" | tee "$ROUND/report.md"
