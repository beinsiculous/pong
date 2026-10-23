#!/usr/bin/env bash
# Build Tong's bank, assets/soundfonts/tong.sf2, and the LICENSE beside it: the FluidR3 GM
# presets the Toreador arrangement plays, cut from the full bank by the engine's
# scripts/subset_soundfont.py. The subset is committed; this script is how anyone rebuilds
# it byte for byte, and the two sha256s in the LICENSE are its provenance.
#
# Usage: scripts/prepare_soundfont.sh [--bank path/to/FluidR3_GM.sf2]
#   Without --bank, Debian's fluid-soundfont-gm 3.1-5.3 package is fetched with
#   `apt download`; where apt cannot find that version, the same package is fetched from
#   snapshot.debian.org. Either way the bank must match the sha256 below, or nothing is
#   written.
set -euo pipefail

GAME_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ENGINE_DIR="$(cd "$GAME_DIR/../../insiculous_2d" && pwd)"
OUTPUT_DIR="$GAME_DIR/assets/soundfonts"
BANK_SHA256="74594e8f4250680adf590507a306655a299935343583256f3b722c48a1bc1cb0"
PACKAGE="fluid-soundfont-gm"
VERSION="3.1-5.3"
# The arrangement's presets, as bank:program: trumpet (the refrain and the jam), trombone
# (the accompaniment's chords), timpani (the march, and the goal's unit). No strings: 0:48
# alone is 5.5 MiB, and a brass band's Toreador needs none.
TONG_PRESETS=(0:56 0:57 0:47)

fetch_from_snapshot() {
    local directory="$1"
    local hash
    hash="$(curl -fsSL "https://snapshot.debian.org/mr/binary/$PACKAGE/$VERSION/binfiles" \
        | python3 -c 'import json, sys; print(json.load(sys.stdin)["result"][0]["hash"])')"
    curl -fsSL -o "$directory/${PACKAGE}_${VERSION}_all.deb" "https://snapshot.debian.org/file/$hash"
}

if [[ $# -ge 1 ]]; then
    if [[ "$1" != "--bank" || $# -ne 2 ]]; then
        echo "usage: $0 [--bank path/to/FluidR3_GM.sf2]" >&2
        exit 2
    fi
    BANK="$2"
else
    DOWNLOAD_DIR="$(mktemp -d)"
    trap 'rm -rf "$DOWNLOAD_DIR"' EXIT
    if ! (cd "$DOWNLOAD_DIR" && apt download "$PACKAGE=$VERSION" >/dev/null 2>&1); then
        echo "apt has no $PACKAGE $VERSION; fetching it from snapshot.debian.org" >&2
        fetch_from_snapshot "$DOWNLOAD_DIR"
    fi
    dpkg -x "$DOWNLOAD_DIR/${PACKAGE}_${VERSION}_all.deb" "$DOWNLOAD_DIR/unpacked"
    BANK="$DOWNLOAD_DIR/unpacked/usr/share/sounds/sf2/FluidR3_GM.sf2"
fi

if [[ "$(sha256sum "$BANK" | cut -d' ' -f1)" != "$BANK_SHA256" ]]; then
    echo "ERROR: $BANK is not FluidR3_GM.sf2 from $PACKAGE $VERSION (sha256 mismatch)." >&2
    exit 1
fi

mkdir -p "$OUTPUT_DIR"
python3 "$ENGINE_DIR/scripts/subset_soundfont.py" "$BANK" "$OUTPUT_DIR/tong.sf2" "${TONG_PRESETS[@]}"

cat > "$OUTPUT_DIR/LICENSE" <<EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: FluidR3 GM (subset)
Upstream-Contact: Frank Wen <getfrank@gmail.com>
Source: Debian $PACKAGE $VERSION, FluidR3_GM.sf2
 sha256 $BANK_SHA256

Files: tong.sf2
Copyright: 2000-2002, 2008 Frank Wen
Comment: A subset of FluidR3_GM.sf2 made by scripts/subset_soundfont.py with the
 presets ${TONG_PRESETS[*]} (bank:program), their instruments and their samples,
 unedited. Re-running scripts/prepare_soundfont.sh reproduces it byte for byte.
 sha256 $(sha256sum "$OUTPUT_DIR/tong.sf2" | cut -d' ' -f1)
License: MIT
 Permission is hereby granted, free of charge, to any person obtaining a copy of
 this software and associated documentation files (the "Software"), to deal in the
 Software without restriction, including without limitation the rights to use, copy,
 modify, merge, publish, distribute, sublicense, and/or sell copies of the Software,
 and to permit persons to whom the Software is furnished to do so, subject to the
 following conditions:
 .
 The above copyright notice and this permission notice shall be included in all
 copies or substantial portions of the Software.
 .
 THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
 INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
 PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
 HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF
 CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
EOF
echo "tong.sf2 ready in $OUTPUT_DIR"
