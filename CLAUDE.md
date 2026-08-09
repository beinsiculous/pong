# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo run                     # play the game
cargo run --features editor   # run the game inside the engine's scene editor
cargo build                   # compile check
cargo test                    # run tests (in src/achievements.rs — incl. locale-file parity)
cargo test <test_name>        # run a single test
```

The game depends on the `insiculous_2d` engine by relative path (`../../insiculous_2d`); both checkouts must sit side by side or nothing builds. Engine crates used: `engine_core` (always) and `editor_integration` (only behind the `editor` feature).

## Architecture

This is a single-crate game (`insiculous_pong`) built on the in-house `insiculous_2d` ECS engine. `PongGame` (in `src/types.rs`) implements the engine's `Game` trait in `src/main.rs` — `init()` spawns all entities, `update()` runs once per frame. With `--features editor` the identical game runs inside the engine's scene editor via `editor_integration::run_game_with_editor`; no game code changes between the two modes.

`PongGame` is composed of focused sub-structs (`Playfield` entity handles, `Balls`, `Scoreboard`, `PowerUpState`, `MatchSettings`, `Textures`) rather than flat fields — keep new state in the sub-struct it belongs to.

**State machine drives everything.** `GameState` (types.rs) is matched at the top of `update()` in main.rs: the menu states (`TitleScreen`, `DifficultySelect`, `ChaosSelect`, `Achievements`) dispatch to handlers in `menu.rs`; everything else (`Serving`, `Playing`, `GameOver`) falls through to `update_gameplay()` in `gameplay/mod.rs`, which orchestrates the per-frame steps implemented across `gameplay/{paddles,balls,scoring,flow}.rs`. Match flow is Title → Difficulty (single-player only) → Chaos select → Serving ↔ Playing → GameOver; match-lifecycle transitions (serve, start, reset-to-title) live in `gameplay/flow.rs`.

**Editor naming:** every spawned entity gets a `Name` component (e.g. "Left Paddle", "Ball 2", "Power-Up (Speed Boost)") so the editor hierarchy is readable — keep this when adding new entities. `Name` is re-exported through `engine_core::prelude`.

**The game steps physics itself.** `PongGame` owns a `PhysicsSystem` and calls `self.physics.update(&mut ctx.world, ctx.delta_time)` inside `update_gameplay()`. Collision events are snapshotted into a `Vec` once per frame and all consumers (goals, power-ups) read that slice — never re-read `collision_events()` mid-frame. Paddles are kinematic bodies moved via `set_kinematic_target`; balls are dynamic with CCD, zero damping, and restitution 1.0; goals are static sensor colliders just off-screen.

**Coordinate and scale conventions (the main trap):**
- World origin is screen center; window is 800×600 (`WIN_W`/`WIN_H`).
- The renderer multiplies `Transform2D.scale` by `RENDER_UNIT = 80.0` to get pixel size — that's why sprite scales are `size / 80.0`.
- Collider shapes use **absolute pixels** and ignore `Transform2D.scale` entirely. Sprites and colliders are sized through different paths, so they can silently diverge. `F1` in-game (or `C` in the editor) overlays collider outlines to check.

**All tuning lives in `src/constants.rs`** (sizes, speeds, colors, power-up timing) and all entity creation lives in `src/spawning.rs`, spawned from those constants. Values tuned live in the editor inspector must be copied back into constants.rs to persist.

**Chaos modes** (Normal / Insane / Ridiculous / Insiculous) are an engine-provided `ChaosMode` enum. Insane doubles a per-ball speed multiplier (`ball_speed_mult: HashMap<EntityId, f32>`) on each paddle hit; Ridiculous starts with a second ball in `extra_balls`; Insiculous is both. `chaos_theme.rs` maps each mode to a color theme applied at spawn time, so the theme is only fully applied on a fresh `init()`/match.

**Visuals:** the Geometry-Wars look comes from `Sprite::with_emissive` values feeding the engine's bloom (ball 2.5, paddles 1.5, walls 0.6) plus a spring-mass deforming grid (`effects.rs`) whose line vertices are pushed into `ctx.lines` every frame after gameplay, so it reacts to that frame's collisions.

**Paths:** assets and saves resolve through `game_root()` in main.rs (exe directory if it contains `assets/`, else `CARGO_MANIFEST_DIR`), so `cargo run` works from any cwd. Achievements persist to `saves/pong_achievements.json`; achievement definitions and unlock logic live in `achievements.rs` and register with the engine's achievement system in `init()`.

**Localization (Jul 2026):** every player-facing string goes through `ctx.strings.tr("key")`; the tables live in `assets/locales/{en,pirate}.ron` (engine loads them via the default `locales` dir under the asset base). Both files MUST define the same key set — `locale_files_have_matching_keys` in achievements.rs enforces it. The title menu's "Language" item cycles locales and re-registers achievements (id-keyed insert refreshes names/descriptions without touching unlocks; keys are `ach.<id>.name`/`ach.<id>.desc`). Pirate's locale file names `fonts/BlackSamsGold-ej5e.ttf`, so switching also swaps the game font. The pause overlay localizes via `PauseMenu::draw_labeled` + `PauseMenuLabels`; difficulty/chaos menu labels come from `Difficulty::label_key()` / `chaos_label_key()` in types.rs.

## The Deion Re-skin (Phase G): Tong

Planned identity — the game still ships the neon look today. Pong is FIRST in the Phase G re-skin order: it validates the sprite pipeline before the other five games follow.

- **New title: Tong.** The paddles become **living tong characters** — kitchen tongs with faces. Their rounded gripping ends give the paddles a naturally ROUNDED collision surface, deliberately making gameplay less flat than rectangle paddles. The tong paddle art/design is SHARED with Breakout's Food Pyramid re-skin.
- **Deion stays the ball** (canon: he squash-stretches on paddle hits, icicle mohawk trails). Countertop court + crumb/splash particles remain live proposals. The tong characters replace the earlier baguette-paddle casting in DEION_STYLE §5. Who "wields" what: the tongs ARE the characters (the AI opponent is a tong personality; 2P = a second tong) — expression/animation details TBD by Jesse.
- **Style SSOT:** `deion_assets/DEION_STYLE.md` via the root symlink (the symlink assumes the standard side-by-side checkout — the same requirement the Cargo path dep already imposes). Settled metrics: 16px base cell, nearest filtering, 5× integer scale to RENDER_UNIT=80, one art cell = one world unit; never fake a footprint via `Transform2D.scale`. IMPORTANT physics note for the re-skin: colliders are absolute pixels and ignore scale — a rounded tong paddle likely means a capsule/rounded collider decision at re-skin time (a collider audit is part of Phase G's definition of done).
- **Runtime assets arrive ONLY via the deion_assets sync copy into `assets/sprites/`** (F2, not yet built) — never symlink or hand-copy art in. AI art is quarantined (`ai_` prefix, `deion_assets/ai/` only) and NEVER ships; `deion_assets/scripts/check_no_ai_assets.sh` must pass on shipping asset trees. Sheet clip names are the stable API.

## Review workflow

The adversarial-review skill lives in `.claude/skills/`. Approved plans go to `review/plan.md` and are reviewed via `scripts/request-review.sh plan review/plan.md --reviewer=kimi` BEFORE implementation. Commits over 100 changed lines are gated by `scripts/commit-review-hook.sh` — the `ADV_REVIEWED=1` prefix is used only after a code-mode review adjudicated with the user, or when the user explicitly skipped review. `review/` is gitignored transients. NOTE: `scripts/request-review.sh` and `scripts/commit-review-hook.sh` are copies of `../../insiculous_2d/scripts/*` — re-copy when the engine master changes.
