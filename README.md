# Insiculous Pong

Neon Pong built on the [insiculous_2d](../../insiculous_2d) engine — bloom-heavy
Geometry-Wars look, a spring-mass deforming grid background, power-ups,
achievements, and the engine's signature chaos modes.

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

| Mode | Left paddle | Right paddle |
|------|-------------|--------------|
| Single player | `W`/`S` or `↑`/`↓` | AI (Easy / Medium / Hard) |
| Two player | `W`/`S` | `↑`/`↓` |

`F1` during a match toggles the in-game collider debug overlay (magenta
outlines drawn over the sprites).

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

## The Deion Pivot: Tong

The game you get today is the neon Geometry-Wars build above — but Pong is
first in line for the **Phase G Deion re-skin**, where it becomes **Tong** and
doubles as the validation run for the engine's new sprite pipeline.

The paddles become **living tong characters**: kitchen tongs with faces. The
tongs ARE the characters — the AI opponent is a tong personality, and two-player
mode is simply a second tong. Their rounded gripping ends give each paddle a
naturally rounded collision surface, deliberately making play less flat than
rectangle paddles. The tong design is shared with Breakout's Food Pyramid
re-skin, and it replaces the earlier baguette-paddle casting in DEION_STYLE §5.

**Deion stays the ball.** Canon: he squash-stretches on paddle hits and his
icicle mohawk trails behind him. A countertop court and crumb/splash particles
are live proposals.

Art follows the settled style metrics (SSOT: `deion_assets/DEION_STYLE.md`
via the repo-root symlink): 16px base cell, nearest filtering, 5× integer
scale to `RENDER_UNIT = 80` — one art cell = one world unit, never faked via
`Transform2D.scale`. Because colliders are absolute pixels and ignore scale,
the rounded tong paddle forces a collider-shape decision at re-skin time
(part of Phase G's definition of done). Runtime art arrives only through the
deion_assets sync copy into `assets/sprites/`; AI-generated stand-ins never
ship.

**Open questions** (answered questions move up into the theme spec above and
get DELETED from this list — live-docs convention):

- Tong character personalities and expressions — what does each tong look
  and act like?
- Rounded paddle collider shape — capsule vs polyline?
- Do the two tongs get distinct designs, or is P2 a palette swap?
- Does the ball's icicle trail interact with the deforming grid?

## Project Layout

```
src/
├── main.rs          # Game trait impl, window/config setup, editor wiring
├── constants.rs     # All gameplay tuning values (sizes, speeds, layout)
├── types.rs         # PongGame state (Playfield, Balls, Scoreboard, ...) and enums
├── spawning.rs      # All entity creation, each entity Named for the editor
├── gameplay/
│   ├── mod.rs       # Match update loop orchestration, grid step/ripple
│   ├── paddles.rs   # Player paddle control and CPU AI
│   ├── balls.rs     # Ball speed maintenance, extra-ball spawn/teardown
│   ├── scoring.rs   # Goal detection, point awards, win condition
│   └── flow.rs      # Serve/game-over input, match start/reset, visibility
├── menu.rs          # Title / difficulty / chaos / achievements navigation
├── power_ups.rs     # Power-up timing and pickup effects
├── effects.rs       # Deforming grid background, hit effects
├── chaos_theme.rs   # Per-chaos-mode color themes
├── achievements.rs  # Achievement definitions
└── ui.rs            # Menu screens and in-match HUD text
```

Every spawned entity carries a `Name` component ("Left Paddle", "Ball",
"Top Wall", "Power-Up (Multi-Ball)", ...), so the editor hierarchy shows
readable names instead of `Entity 7`.
