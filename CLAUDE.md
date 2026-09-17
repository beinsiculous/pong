# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo run                     # play the game
cargo run --features editor   # run the game inside the engine's scene editor
cargo build                   # compile check
cargo test                    # run tests (inline modules, incl. the locale-file parity check)
cargo test <test_name>        # run a single test
```

The game depends on the `insiculous_2d` engine by relative path (`../../insiculous_2d`); both checkouts must sit side by side or nothing builds. Engine crates used: `engine_core` (always) and `editor_integration` (only behind the `editor` feature).

## Architecture

This is a single-crate game (`insiculous_pong`) built on the in-house `insiculous_2d` ECS engine. `PongGame` (in `src/types.rs`) implements the engine's `Game` trait in `src/main.rs` — `init()` spawns all entities, `update()` runs once per frame. With `--features editor` the identical game runs inside the engine's scene editor via `editor_integration::run_game_with_editor`, and at `/playground/pong/` in the browser (the same feature, built by the engine's `build_wasm.sh --kind editor` and served from the site); no game code changes between the modes.

`PongGame` is composed of focused sub-structs (`Playfield` entity handles, `Balls`, `Scoreboard`, `PowerUpState`, `MatchSettings`, `Sheets`) rather than flat fields — keep new state in the sub-struct it belongs to.

**State machine drives everything.** `GameState` (types.rs) is matched at the top of `update()` in main.rs: the menu states (`TitleScreen`, `DifficultySelect`, `ChaosSelect`, `Achievements`) dispatch to handlers in `menu.rs`; everything else (`Serving`, `Playing`, `GameOver`) falls through to `update_gameplay()` in `gameplay/mod.rs`, which orchestrates the per-frame steps implemented across `gameplay/{paddles,balls,scoring,flow}.rs`. Match flow is Title → Difficulty (single-player only) → Chaos select → Serving ↔ Playing → GameOver; match-lifecycle transitions (serve, start, reset-to-title) live in `gameplay/flow.rs`.

**Editor naming:** every spawned entity gets a `Name` component (e.g. "Left Paddle", "Ball 2", "Power-Up (Speed Boost)") so the editor hierarchy is readable — keep this when adding new entities. `Name` is re-exported through `engine_core::prelude`.

**The game steps physics itself.** `PongGame` owns a `PhysicsSystem` and calls `self.physics.update(&mut ctx.world, ctx.delta_time)` inside `update_gameplay()`. Collision events are snapshotted into a `Vec` once per frame and all consumers (goals, power-ups) read that slice — never re-read `collision_events()` mid-frame. Paddles are kinematic bodies moved via `set_kinematic_target`; balls are dynamic with CCD, zero damping, and restitution 1.0; goals are static sensor colliders just off-screen.

**Coordinate and scale conventions (the main trap):**
- World origin is screen center; window is 800×600 (`WIN_W`/`WIN_H`).
- The renderer multiplies `Transform2D.scale` by `RENDER_UNIT = 80.0` to get pixel size — that's why sprite scales are `cell / 80.0`.
- `Sprite.offset` is where the cell's centre sits relative to the entity, in world units, Y up — the measured anchors ride on it (see the Tong section).
- Collider shapes use **absolute pixels** and ignore `Transform2D.scale` entirely. Sprites and colliders are sized through different paths, so they can silently diverge. `F1` in-game (or `C` in the editor) overlays collider outlines to check.

**All tuning lives in `src/constants.rs`** (sizes, speeds, colours, the sheets block, power-up timing) and all entity creation lives in `src/spawning.rs`, spawned from those constants. Values tuned live in the editor inspector must be copied back into constants.rs to persist.

**Chaos modes** (Normal / Insane / Ridiculous / Insiculous) are an engine-provided `ChaosMode` enum. Insane doubles a per-ball speed multiplier (`ball_speed_mult: HashMap<EntityId, f32>`) on each paddle hit; Ridiculous starts with a second ball in `extra_balls`; Insiculous is both. `chaos_theme.rs` (engine) maps each mode to a color theme: its grid colour reaches the backdrop grid at match start (`start_game` — the mode is picked after `init()`), its `particle_count_mult` and accent colour reach the particle bursts every frame, and **nothing tints the art** — the sprites are the sheets' own colours.

**Visuals:** the art is the Deion sheets, drawn 1× and snapped. Every art sprite is white with emissive 0, so the neon build's bloom is gone with it. What is left of the Geometry-Wars look is the **backdrop grid**: `init()` spawns one entity carrying the engine's `GridBackdrop` (over-sprites, the theme's grid colour at a low alpha — D5), the engine simulates and draws it, and gameplay queues impulses into it through `ripple_grid`. The only line-buffer work the game still pushes itself is `gameplay/mod.rs`'s collider overlay, for F1.

**Paths:** assets and saves resolve through `game_root()` in main.rs (exe directory if it contains `assets/`, else `CARGO_MANIFEST_DIR`), so `cargo run` works from any cwd. Achievements persist to `saves/pong_achievements.json`; achievement definitions and unlock logic live in `achievements.rs` and register with the engine's achievement system in `register_achievements()`, which the engine calls before the window opens; `cargo run -- --achievements-manifest <path>` exports the list.

**Localization (Jul 2026):** every player-facing string goes through `ctx.strings.tr("key")`; the tables live in `assets/locales/{en,pirate}.ron` (engine loads them via the default `locales` dir under the asset base). Both files MUST define the same key set — `locale_files_have_matching_keys` in achievements.rs enforces it. The title menu's "Language" item cycles locales and re-registers achievements (id-keyed insert refreshes names/descriptions without touching unlocks; keys are `ach.<id>.name`/`ach.<id>.desc`). Pirate's locale file names `fonts/BlackSamsGold-ej5e.ttf`, so switching also swaps the game font. The pause overlay localizes via `PauseMenu::draw_labeled` + `PauseMenuLabels`; difficulty/chaos menu labels come from `Difficulty::label_key()` / `chaos_label_key()` in types.rs.

## Tong (landed 2026-09-15)

Pong is the **first of the six Phase G Deion re-skins**, and the build is no longer neon: the
paddles are **living tongs**, the ball is a **meatball with eyes**, the goals are **grills**
behind them, the floor is a **countertop tile** and the walls are **rails**. In-game the game is
**Tong** (`title.window` in both locale files); the site still lists it as *Insiculous Pong*, and
that stays until `insiculous_web#64` rules.

- **Art enters only through the sync.** `assets/sprites/sync.list` pins the `deion_assets` commit
  and lists the nine sources; `python3 deion_assets/scripts/sync_sprites.py .` copies each PNG and
  its `.sheet.ron` sidecar in, and `--check` hashes the copies against the pinned blobs. Never
  hand-copy or hand-edit a file there — fix the master in `deion_assets` and re-sync. The working
  set's `scripts/check-sprite-sync.sh` runs that check over every game and prints `pong OK` (the
  other five have no sync list yet).
- **The sheets block** in `src/constants.rs` names each sheet once (`SheetSpec`): its path, its
  cell, and the opaque bounds of the reference frame the collider is measured from. The cell
  drives the draw scale; the bounds drive the collider and the `Sprite.offset` that lands the art
  on it. Nothing is sized by guess, and nothing is faked through `Transform2D.scale`.

  | subject | cell | collider | anchor |
  |---|---|---|---|
  | tong (left / right) | 64×96 | the jaw pose the drawn frame is (table below) | centred |
  | meatball | 48×64 | circle r 15.5 | (−0.5, +10.5) — the body sits low, fire above it |
  | grill | 32×96 | none — the goal sensor scores | centred |
  | pickup (flame / knife) | 32×32 | circle r 12 (the unchanged `POWERUP_SIZE`) | centred; the art's own 1 px is deliberately not applied |
  | court tile / wall rail | 64×64 / 64×16 | none | centred |

  **The jaw's five poses** are measured from the left tong's `_up` cells, per row from the synced
  PNG, and live in `src/jaw.rs` (the working of them, frame by frame, is `gameplay/jaws.rs`). Every open pose
  is four capsules: two arms from the one hinge — the cell's mirror axis, 33.5 below the centre —
  to that pose's tips, 27.5 above the centre, and on each tip a pad, 11 px wide from row 26 up to
  the domed top at row 9 (a vertical capsule of radius 5.5 from 26.5 to 33.5, reaching 39 like the
  closed tong's own top). The arms are drawn 7 px thick, so `TONG_ARM_RADIUS` is 3.5:

  | pose | mouth | tips | frames |
  |---|---|---|---|
  | `open` | 24 px | ±17.5 | 0, 1, 9 |
  | `wide` | 18 px | ±14.5 | 2, 8 |
  | `narrow` | 10 px | ±10.5 | 3, 7, 11, 13 |
  | `twitch` | 4 px | ±7.5 | 10, 12 |
  | `closed` | one body | the flat `capsule_y(78, 11)` | 4, 5, 6 |

  A tip's centre is half its mouth plus half a pad (5.5 px) out from the axis, so the mouth is
  the whole of what closes, and the pad capsules' inner faces *are* the mouth. Every mouth is
  narrower than the meatball, so a closing jaw can never take a ball in: the bite is a tip
  deflection. The `_down` collider is the `_up` one mirrored in y, which is how the art draws it
  too: both tongs are one body, carried both ways up.

  **The collider follows the drawn frame.** `clip_poses` gives a pose per clip frame, and each
  frame pong reads the tong's `SpriteAnimation.current_clip` (its facing stripped) and its
  **clip-relative** `current_frame` and dresses the collider to match — written only when the
  shape changes, so a rebuild is spent only where the drawn jaw moved. The engine advances
  animations in the frame tail *after* the game's update, so the outline is the frame drawn last
  game frame: a lag of one game frame, a sixth of the shortest pose.

- **Animation is the engine's `ClipStateMachine`** — the game declares a table and never polls a
  clip. The state names are constants in `src/types.rs`, and they are the sidecars' clip names: a
  rename in the art is a rename in the table.

  | subject | table |
  |---|---|
  | tong | ten states, the five clips in each facing: `open_up`/`open_down` (Stay) · `closing_*` → `closed_*` · `closed_*` (Stay) · `opening_*` → `open_*` · `scored_on_*` → the jaw it rests at |
  | meatball | `idle` (Stay) · `toasted` (Stay) |
  | grill | `idle` (Stay) · `score` → `idle` |
  | pickup | `idle` (Stay); the puff it leaves plays `collect` → despawn |

  `_up` is the left tong's pictured orientation — tips up, hinge down — and `_down` its mirror.
  `TONG_CLOSED` is a state like any other: a shut jaw is something the tong rests at, not a pose
  passed through.

  **The player works the jaws.** Up and down move a tong; its horizontal axis works the jaw —
  toward the court closes and away opens, and the press sticks until the stick asks the other
  way (past a 0.5 dead zone, inside which the held jaw stands). Both players push at the ball to
  bite. The face follows the vertical axis, turned only from a resting jaw so no clip restarts
  under the tong. **A contact changes nothing**: physics bounces the meatball and the jaw goes on
  doing what it was told. A goal against sets `scored_on` in the facing that tong wears, flares
  the grill behind it, and burns the fire on the conceded side's goal line.

  The CPU works its jaw by policy: it shuts when the ball's time to its face is inside Medium's
  0.45 s or Hard's 0.6 s, and opens only when the ball's return leaves a whole open and close to
  spare. Its face turns toward a ball beyond the tong's own end (`CPU_FACE_TURN_DISTANCE`, half
  the tong) and holds for one alongside, where the chase reverses by a pixel or two every frame
  and a face that followed it would flap. **Easy never opens** — its jaw rests shut, a parameter of its machine, so a goal against
  it plays `scored_on` and comes back shut. A serve resets both jaws' asked-for state to the
  match's rest, so a bite asked for in the last rally and since released is not carried into
  the next one — a stick still pushing is still asking. Only a
  `started` event is a hit: the engine reports a pair by diffing each step's contact set against
  the last, so a jaw rebuilt under a ball it already touches emits no new start and a rebuild
  never echoes a hit. The **flame** toasts the meatball it catches (the Insane speed step
  no longer does) and the boost's expiry is what un-toasts it; the **knife**'s extra ball spawns
  from the collecting ball, so the meatball visibly splits.
- **1× and pixel-snapped** (`GameConfig::with_pixel_snap(true)`, D1): one art pixel per window
  pixel at `RENDER_UNIT = 80`, nearest filtering, no faked scale. Every art sprite is drawn white
  with emissive 0 — only the particle bursts keep a paddle-side colour.
- **The measured playfield** (all in `src/constants.rs`, derived from the art in
  `review/art-revamp/report-4.md`): the goal line is the court's edge (`COURT_HALF_W = 400`) and
  the goal sensor's box begins on it at `GOAL_SENSOR_X = 410`; the walls' inner face is at ±280;
  the tongs stand at `PADDLE_X = 350`, far enough in that a meatball resting on a tong is not
  already inside the sensor and the tong's whole 64 px cell clears it; `PADDLE_MAX_Y = 241` puts
  the tong's art top on the wall's face; `GRILL_X = 384` puts a grill's cell flush with the goal
  line. Inline tests in `constants.rs` pin those relations.
- **Style SSOT:** `deion_assets/DEION_STYLE.md` via the `deion_assets -> ../../deion_assets`
  symlink (the working set's layout — the Cargo path dep `../../insiculous_2d` already requires
  it). AI art is quarantined (`ai_` prefix, `deion_assets/ai/` only) — tiered ship rule
  (DEION_STYLE.md §6, Aug 19 2026): may ship in FREE web builds, never in paid/marketplace builds.
  **Tong is free-tier only until Jesse's cleanup pass** (D3): the synced copies keep their `ai_`
  prefix, so `deion_assets/scripts/check_no_ai_assets.sh assets` fails on a paid build, as it
  must. The exit is a hand-cleaned master under `deion_assets/sidescroller/…`, re-exported without
  the prefix, with the `sync.list` line moved to it.

## Work tracking

Open work lives on the **Studio Board** (https://github.com/orgs/beinsiculous/projects/1)
as issues in this repo. **Always pass `-R beinsiculous/pong`** — a bare `gh` command
resolves against the session's working directory, which is often the working-set root, so
it lists and files against the wrong repository.

```sh
gh issue list -R beinsiculous/pong
gh api repos/beinsiculous/pong/milestones --jq '.[] | "\(.title): \(.description)"'
```

Issues are grouped into **sprint milestones**; each description records the batch's
internal order and its gates. Take the next unblocked issue in a sprint, not an arbitrary
one. Claim by assigning yourself; close with `fixes beinsiculous/pong#N` in the commit.

**Unfinished work becomes an issue.** Anything you don't finish — work you deferred, debt
you created, a follow-up you spotted — is filed before you report done. Never buried in a
doc, never left as a bare `TODO:`, never dropped. The `file-issue` skill carries the shape;
`sprint-planning` groups issues into shippable batches.

## Review workflow

The adversarial-review skill lives in `.claude/skills/`. Approved plans go to `review/plan.md` and are reviewed via `scripts/request-review.sh plan review/plan.md --reviewer=kimi` BEFORE implementation. Commits over 100 changed lines are gated by `scripts/commit-review-hook.sh` — the `ADV_REVIEWED=1` prefix is used only after a code-mode review adjudicated with the user, or when the user explicitly skipped review. `review/` is gitignored transients. NOTE: `scripts/request-review.sh` and `scripts/commit-review-hook.sh` are copies — the canonical ones live in the working-set root, not in `insiculous_2d`. Never edit a copy: fix the root's and re-copy, and `scripts/check-skill-parity.sh` there reports any repo that drifted.
