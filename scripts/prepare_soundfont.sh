#!/usr/bin/env bash
# Rebuild the original MIDI Evil score/bank into staging; see music-source/README.md.
set -euo pipefail
GAME_DIR="$(cd "$(dirname "$0")/.." && pwd)"
exec python3 "$GAME_DIR/music-source/build.py" "$@"
