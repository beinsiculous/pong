# Insiculous Pong

**Tong** — the Deion re-skin of Pong, built on the [insiculous_2d](../../insiculous_2d)
engine: tongs for paddles, a meatball with eyes for the ball, grills for goals, a
countertop court, power-ups, achievements, and the engine's signature chaos modes.

## Running

The game depends on the engine by path (`../../insiculous_2d`), so keep both
checkouts side by side:

```bash
cargo run                     # play the game
cargo run --features editor   # run the game inside the engine's visual editor
```

Assets and saves resolve relative to the executable (falling back to the crate
directory), so `cargo run` works from any working directory. Achievements
persist to `saves/pong_achievements.json`.

## Controls

**Menus** — `W`/`S` or `↑`/`↓` to navigate, `Enter`/`Space` to confirm,
`Escape` to go back.

| Mode | Left tong | Right tong |
|------|-----------|-------------|
| Single player | `W`/`A`/`S`/`D` or the arrow keys | AI (Easy / Medium / Hard) |
| Two player | `W`/`A`/`S`/`D` | arrow keys |

**Up and down move a tong; left and right work its jaw.** Pushing a tong
toward the court shuts its jaw and pulling it away opens it, and the press
sticks until you push the other way — on a stick, a lean past the middle is
enough. Both players push at the ball to bite: the left tong shuts on `D`,
the right one on `←`.

**The face follows the motion.** Moving up turns a tong's gripping tips up and
moving down turns them down, so a tong meets the ball whichever way it is
travelling. A tong turns only with its jaw at rest, and it holds the face it
has while a stick sits near the middle.

`F1` during a match toggles the in-game collider debug overlay (magenta
outlines drawn over the sprites) — the outline is the jaw the tong is drawing,
so it opens and shuts with the art.

## Chaos Modes

Pick one before each match:

| Mode | Effect |
|------|--------|
| Normal | Classic Pong |
| Insane | Ball speeds up on every paddle hit |
| Ridiculous | Match starts with two balls |
| Insiculous | Both at once |

## Editor Mode

`cargo run --features editor` opens the exact same game inside the engine's
scene editor — useful for inspecting and tuning entities while the game runs:

- **Play / Pause / Stop**: `F5` or `Ctrl+P` to play, `Ctrl+P` to pause,
  `Ctrl+Shift+P` to stop and restore the pre-play world.
- **Inspect**: click entities in the hierarchy or viewport; the inspector
  shows and edits Transform2D, Sprite, RigidBody, and Collider fields with
  undo/redo (`Ctrl+Z`/`Ctrl+Y`).
- **Collider overlay**: press `C` to toggle collider outlines in the scene
  view (green = solid, cyan = sensors like the goal zones, yellow = selected).
  The outlines show exactly what the physics simulation uses — collider sizes
  are absolute pixels and ignore `Transform2D.scale`, which is how the sprites
  are sized, so any sprite-vs-collider mismatch is immediately visible.
- **Tune collider shapes**: box half-extents, circle radius, and capsule
  height/radius are editable in the inspector. The overlay updates live;
  the running simulation picks the new shape up when the body is next
  created (e.g. a fresh play session). For permanent fixes, copy the tuned
  values back into `src/constants.rs` (`PADDLE_W`, `PADDLE_H`, `BALL_SIZE`,
  ...), since all entities are spawned from those constants in
  `src/spawning.rs`.

The same build runs in the browser at [beinsiculous.com/playground/pong/](https://beinsiculous.com/playground/pong/): the game inside the editor, layout only — the rules are compiled in and nothing you change there persists.

## Pong as data

Pong's gameplay rules and entities also exist entirely as data and Rhai scripts under
`insiculous_2d/crates/playground/assets/projects/pong/`. The project runs in the Web Playground at
`/playground/?project=pong`, where scene layouts, entity components, and script logic can be edited
and verified live in the browser without recompiling Rust.

## Tong: the Deion re-skin

Pong is the first of the six **Phase G Deion re-skins**, and the neon Geometry-Wars build is
gone — this is **Tong**. The paddles are living kitchen tongs with an eye on each gripping tip,
the ball is a **meatball with eyes**, and the **goals are grills** behind the tongs, on a
countertop court between two rails. The art came from Jesse's Sep 9 2026 brief, drawn in
Aseprite, and lives in `deion_assets` (DEION_STYLE §9); the tong design is shared with
Breakout's Food Pyramid re-skin.

- **You work the jaws.** A tong shuts when you push it at the court and opens when you pull it
  away, and a contact changes nothing — physics bounces the meatball and the jaw goes on doing
  what you told it. A ball off an open arm leaves at an angle; off a shut one it leaves flat.
  Those are the engine's clip state machine's states: the art's clips, cut into the sheets,
  driven by a table the game declares.
- **The AI works its jaw too.** Medium and Hard shut theirs just before a ball that is arriving,
  and open again only when its return leaves time for a whole bite; Easy never opens at all,
  which is the flat, classic return to beat.
- **A goal burns on the goal line**, not where the meatball died: the fire catches in front of
  the grill the point was conceded at, while that grill flares and the tong behind it wears its
  angry pose.
- **The meatball burns.** A scored meatball catches fire on the goal line and burns on through
  the next serve; the flame pickup toasts it for as long as the speed boost runs, and the knife
  pickup splits it in two.
- **1× art, pixel-snapped.** One art pixel is one window pixel, nearest-filtered, no faked
  scale and no bloom over the top. The spring-mass grid still ripples over the countertop.
- **Art arrives only through the sync.** `assets/sprites/sync.list` pins the `deion_assets`
  commit it came from, and `python3 deion_assets/scripts/sync_sprites.py .` copies each sheet in;
  the working set's `scripts/check-sprite-sync.sh` proves a game's copies still match that pin.
  The copies keep their `ai_` prefix, so this build is free-tier until the hand-cleaning pass
  (DEION_STYLE §6).
- **The site's entry still says *Insiculous Pong*** (slug `pong`): what the shopfront calls the
  game is M and Jesse's call (`insiculous_web#64`).

**Open questions** (answered questions move up into the spec above and
get DELETED from this list — live-docs convention):

- The two tongs are drawn as a pair, left and right. Should they also develop
  distinct personalities?

## Project Layout

```
src/
├── main.rs          # Game trait impl, window/config setup, editor wiring
├── constants.rs     # All gameplay tuning values (sizes, speeds, layout)
├── types.rs         # PongGame state (Playfield, Balls, Scoreboard, ...) and enums
├── spawning.rs      # All entity creation, each entity Named for the editor
├── gameplay/
│   ├── mod.rs       # Match update loop orchestration, clip states, collider overlay
│   ├── paddles.rs   # Player paddle control and CPU AI
│   ├── balls.rs     # Ball speed maintenance, extra-ball spawn/teardown
│   ├── scoring.rs   # Goal detection, point awards, win condition
│   └── flow.rs      # Serve/game-over input, match start/reset, visibility
├── menu.rs          # Title / difficulty / chaos / achievements navigation
├── power_ups.rs     # Power-up timing and pickup effects
├── effects.rs       # Particle presets for paddle hits and goals
├── chaos_theme.rs   # Per-chaos-mode color themes
├── achievements.rs  # Achievement definitions
└── ui.rs            # Menu screens and in-match HUD text
```

Every spawned entity carries a `Name` component ("Left Paddle", "Ball",
"Top Wall", "Power-Up (Multi-Ball)", ...), so the editor hierarchy shows
readable names instead of `Entity 7`.
