#!/usr/bin/env bash
# Informative embedding benchmark (v2.0 Grupo A Fase 4) — NOT a CI gate.
#
# Builds benchmarks/embedding_cosine.bx with -O3, does one warm-up run, then
# times REPS runs (default 5) and prints a Markdown report with the platform,
# CPU, the BLAS library the binary actually links, the BLAS thread settings
# and the timings. Timings include process startup and generating the two
# random embeddings (negligible next to 1M BLAS-backed cosines).
#
# Usage (from the repo root): benchmarks/run.sh   [REPS=10 benchmarks/run.sh]
# In GitHub Actions the report is also appended to the job summary.
set -euo pipefail

cd "$(dirname "$0")/.."
REPS="${REPS:-5}"
BIN=./embedding_cosine
trap 'rm -f "$BIN"' EXIT

cargo run -q -- benchmarks/embedding_cosine.bx -O 3 >/dev/null

case "$(uname -s)" in
  Darwin)
    cpu="$(sysctl -n machdep.cpu.brand_string)"
    blas="$(otool -L "$BIN" | grep -i 'libblas' | awk '{print $1}' | paste -sd ' ' -)"
    ;;
  *)
    cpu="$(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ *//' || true)"
    [ -n "$cpu" ] || cpu="$(lscpu 2>/dev/null | grep -m1 'Model name' | cut -d: -f2- | sed 's/^ *//' || true)"
    blas="$(ldd "$BIN" | grep -i blas | awk '{print $3}' | while read -r lib; do readlink -f "$lib"; done | paste -sd ' ' -)"
    ;;
esac

"$BIN" >/dev/null # warm-up (page cache, BLAS initialization)

TIMEFORMAT=%R # bash builtin `time`: elapsed seconds, millisecond resolution
times=""
for _ in $(seq "$REPS"); do
  t="$( { time "$BIN" >/dev/null; } 2>&1 )"
  times="$times $t"
done
sorted="$(echo "$times" | tr ' ' '\n' | grep . | sort -n)"
median="$(printf '%s\n' "$sorted" | awk '{a[NR]=$1} END {print (NR % 2) ? a[(NR+1)/2] : (a[NR/2] + a[NR/2+1]) / 2}')"
best="$(printf '%s\n' "$sorted" | head -1)"

report="$(cat <<EOF
### Embedding benchmark (informative, not a CI gate)

1,000,000 \`cosine_similarity\` calls between two \`Embedding<1536>\` (each call: 1× \`ddot_\` + 2× \`dnrm2_\`), \`-O 3\`.

| | |
|---|---|
| Commit | \`$(git rev-parse --short HEAD 2>/dev/null || echo unknown)\` |
| Platform | $(uname -sm) |
| CPU | ${cpu:-unknown} |
| BLAS | \`${blas:-unknown}\` |
| Threads | OPENBLAS_NUM_THREADS=${OPENBLAS_NUM_THREADS:-unset}, VECLIB_MAXIMUM_THREADS=${VECLIB_MAXIMUM_THREADS:-unset}, OMP_NUM_THREADS=${OMP_NUM_THREADS:-unset} |
| Warm-up | 1 run (discarded) |
| Runs (s) |$times |
| **Median** | **${median} s** (best ${best} s) |
EOF
)"

echo "$report"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  echo "$report" >> "$GITHUB_STEP_SUMMARY"
fi
