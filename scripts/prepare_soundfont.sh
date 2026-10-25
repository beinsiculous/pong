#!/usr/bin/env bash
# Build Tong's bank, assets/soundfonts/tong.sf2, and the LICENSE beside it, through the
# engine's scripts/prepare_game_soundfont.sh: the FluidR3 GM presets the Toreador arrangement
# plays. The subset is committed; this script is how anyone rebuilds it byte for byte.
#
# Usage: scripts/prepare_soundfont.sh [--bank path/to/FluidR3_GM.sf2]
#   The engine's script documents the fetch, the sha256 check and the 4 MiB cap.
#
# The presets, as bank:program: trumpet (the refrain and the jam), trombone (the
# accompaniment's chords), timpani (the march, and the goal's unit). No strings: 0:48 alone
# is 5.5 MiB, and a brass band's Toreador needs none.
set -euo pipefail

GAME_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ENGINE_DIR="$(cd "$GAME_DIR/../../insiculous_2d" && pwd)"
OUTPUT_DIR="$GAME_DIR/assets/soundfonts"

exec "$ENGINE_DIR/scripts/prepare_game_soundfont.sh" "$@" --max-mib 4 "$OUTPUT_DIR" tong 0:56 0:57 0:47
