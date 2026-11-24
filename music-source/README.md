# Countertop Circuit

Original AI-authored 128 BPM D-minor electro-funk for Tong, CC0-1.0. No reference-game
melodies or recordings. The 32 bars are A (1–8), B (9–16), reduced break (17–24),
and reprise (25–32), with four-bar hook sentences, drum fills and velocity accents.
The backing plays independently. Pitched material uses D/F/A/C; every cue/backing
pitch-class pairing excludes semitones and tritones. The last bass note ends exactly
at tick 61440, at 480 PPQ. `melody` and `jam` retain the existing immediate Loop cues.
Map identifiers `trumpet` and `timpani` now name metallic contact and low goal synths.

`countertop.evil` is editable in MIDI Evil. `compose.py` is its deterministic original
composition recipe; rerunning it intentionally replaces manual project/patch edits.
Nine original patches live in `patches/`: clipped resonant bass, square lead, glass
chord, harmonic contact, low goal, swept kick, filtered snare high-passed hats and a quiet 35 ms triangle movement tick.
`render.json` pins isolated notes, velocity 127, rate 32000 Hz, master gain 0.8 and
seed base 4100 (patch index added). `bank.json` pins one exact-key zone per used note,
with no resampling or loops. Finite sample gates are 480 ticks plus renderer tails;
longest authored note is 180 ticks (0.1758 seconds), below the declared 0.24-second
budget plus 40 ms release. No source is truncated to fit the bank.

The engine uses three triggered slots and one layer per note, with five peak background
voices. Dense auditions exercise 20 contacts/second; 70-tick contact gates plus 40 ms
release need fewer than three simultaneous voices on average, comfortably below the
24-voice slot ceiling. Short patch decays further separate contacts from backing.
The raw previews and separate matched listening copies preserve dynamics.

## Rebuild

Run from this game directory, with the normal working-set sibling layout:

```sh
python3 music-source/compose.py       # only to recreate the authored source
python3 music-source/build.py        # or scripts/prepare_soundfont.sh
python3 music-source/build.py /tmp/tong-clean-build
```

This needs MIDI Evil at `../../midi-evil`, engine at `../../insiculous_2d`, Python 3.12.3
and Rust 1.94.0 on x86_64 Linux. Tool revisions:
MIDI Evil `46ae9082a358314202632c60c3cc13dbf5da1abf`, engine
`7c8fed0` (shared builder, including packer/audition/remapper).
Revalidated unchanged exports with MIDI Evil `3d061af` after its merge. The old comparison uses Tong
`7165c8c8340cf25df19bed7288a17ab399b5a52a`.
Cargo.lock pins dependencies. Same-platform rebuilds must match byte for byte;
no cross-platform floating-point identity is promised. Runtime needs only the
committed MIDI, engine map and bank, not MIDI Evil or Python.

Build outputs stay under ignored `rendered/`. Validate a staged asset closure through
`audition` before installation. Copy the exported MIDI and ai_tong.sf2 to their existing
asset folders, retain the old .mid.ron, then save the map through the public API:

```sh
cargo run --manifest-path ../../insiculous_2d/Cargo.toml -p editor --example remap_soundfont -- \
  assets ai/music/ai_tong.mid soundfonts/ai_tong.sf2 \
  'Original AI composition and MIDI Evil synthesis for Be Insiculous, 2026' CC0-1.0
cargo run --example install_movement
```

Never hand-edit the map/fingerprint. Bank provenance is in assets/soundfonts/LICENSE.
The build script only stages outputs and never overwrites committed assets on failure.

## Audition

```sh
python3 music-source/preview.py
```

This retrieves the previous committed audio from the pinned Tong revision, generates
idle/sparse/dense timestamp recipes and uses the real conductor and production limiter.
The ignored `previews/` directory holds `old-idle.wav`, `new-idle.wav`, `new-sparse.wav`,
`new-dense.wav`, matching `.log` metrics, and `*-matched.wav` listening copies. Each is
121 seconds: two complete new loops plus one second. They are synthetic event mixes,
not gameplay recordings. Both idle comparisons have RMS 0.04; sparse/dense use the same
new-score gain so their extra energy is not normalized away. `level_match.py` refuses
matching above -1 dBFS. Old reference audio remains governed by its original license.

Listening and actual browser behavior are separate planner/user gates. Compilation and
signal measurements do not establish that the song is enjoyable. Tong has no standalone
volume/mute UI; bus behavior is tested in the engine, without adding game controls.

The old/new comparison requires the pinned historical commit in your clone. Fetch
that history first, or pass `preview.py --reference REVISION` for an available old
audio revision. Missing history fails explicitly rather than skipping the comparison.

## Movement and explicit feedback demonstrations

`movement` is a separate named track/part on program 57, a triangle with gain 0.16,
35 ms decay, 40 ms release and 40-tick notes at velocity 72. It quotes D/F/A/C;
its cursor never advances `refrain`. Contact remains program 56 (harmonic sine,
75 ms decay); goal remains program 47 (low triangle, 140 ms decay). The approved
backing and previous cue notes are unchanged; contact and goal patch gains are raised as below. The native `install_movement`
example depends on editor only for development; its idempotent SongDocument
apply/save operation adds the part, instrument and binding and owns fingerprinting.

The dense recipe adds two movement sources every 300 ms to the contact bursts.
At the gameplay ceiling (two sources / 180 ms), 39.1 ms gates plus 40 ms releases
need at most two simultaneous movement voices, below the 24-voice slot limit.
Each preset has one layer per note, below six; three triggered slots stay below eight.

`preview.py` also creates `new-hit.wav`, `new-goal.wav`, `new-move.wav` and
`isolated-{hit,goal,move}.wav` in `previews/`. Each action recipe triggers once per
second for 120 events across two wraps. Isolation subtracts the identical engine
idle PCM from the engine action mix, and refuses if either render activated the
limiter (a nonlinear mix cannot be subtracted faithfully). Quantization error is
at most one 16-bit sample step. These isolated files retain original gain, expose
the distinct envelopes/registers, and carry separate peak/RMS logs. Historical
assets are extracted into a fresh temporary directory on every invocation;
NUL-delimited git paths are decoded with os.fsdecode and Git errors remain visible.

Contact and scoring gains are deliberately above the first audition (0.68/1.0
patch gain), without raising the backing. Movement remains gain 0.16 and short.

The first install command migrates the old bank paths and provenance; the second adds
movement. Run both when starting from a pre-Countertop sidecar. For a separately saved
classical editor audition, preserve or restore its original tong.sf2 and corresponding
LICENSE from the pinned old Git revision; do not retarget arbitrary old scores to the
new key-limited bank. New website bundles use game v8/editor v7; the historical score restoration above
is separate from that release.
