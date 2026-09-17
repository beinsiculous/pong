//! Insiculous Pong — game crate.
//!
//! The library owns the whole game (`PongGame` + its `Game` impl) so both
//! entry points stay thin: `main.rs` (native window, filesystem saves,
//! optional editor) and `web_entry.rs` (wasm-bindgen start: fetch assets,
//! then the same `run_game`). This split also keeps `editor_integration`
//! behind the `editor` feature in both entry points — `main.rs` for the native
//! window, `web_entry.rs` for the browser's editor bundle.

mod achievements;
mod constants;
mod effects;
mod gameplay;
mod jaw;
mod menu;
mod power_ups;
mod spawning;
mod types;
mod ui;

#[cfg(target_arch = "wasm32")]
mod web_entry;

use engine_core::prelude::*;
use constants::*;
use spawning::*;
use types::*;

pub use types::PongGame;

/// The shared `GameConfig` for every target. Entry points add their own
/// platform extras on top (native: save paths anchored to the game dir;
/// web: localStorage keys per the engine's `docs/WEB_SAVES.md` contract).
///
/// `asset_base` must be an ANCHORED base: native callers pass an absolute
/// path (`main.rs` derives it from `game_root!()` so the cwd never
/// matters); the web entry passes the deploy URL base. Passing a bare
/// relative path like `"assets"` would silently resolve against the
/// current working directory.
pub fn game_config(asset_base: &str) -> GameConfig {
    GameConfig::new("Insiculous Pong")
        .with_size(WIN_W as u32, WIN_H as u32)
        .with_clear_color(0.0, 0.0, 0.0, 1.0)
        .with_fps(60)
        // The art is 1x with nearest filtering, so snapping every sprite's origin to a
        // whole device pixel is what keeps it crisp at this window size (D1).
        .with_pixel_snap(true)
        .with_asset_base_path(asset_base)
}

/// Load one synced sheet by the path its spec names. Fail-loud: the sheets ship with
/// the game, so a missing or malformed one is a broken build rather than a game that
/// silently draws nothing.
fn load_sheet(assets: &mut AssetManager, spec: &SheetSpec) -> SpriteSheet {
    assets
        .load_sprite_sheet(spec.path)
        .expect("every Tong sheet ships with the game")
}

impl Game for PongGame {
    fn register_achievements(&self, achievements: &mut AchievementManager, strings: &Strings) {
        // Names and descriptions come from the locale tables; the title menu's Language item
        // re-registers on a locale switch.
        achievements::register_all(achievements, strings);
    }

    fn init(&mut self, ctx: &mut GameContext) {
        // Resolve against the configured asset base so the same relative
        // path works natively (game dir) and on the web (VFS keys).
        let font_path = std::path::Path::new(ctx.assets.base_path()).join("fonts/font.ttf");
        if let Ok(font) = ctx.ui.load_font_file(&font_path.to_string_lossy()) {
            ctx.ui.set_default_font(font);
        }

        let tex = ctx.assets.create_solid_color(1, 1, [255, 255, 255, 255]).unwrap();
        self.sheets.white = tex.id;

        // Every sheet's path, cell and measured anchor is in `constants.rs`'s sheets
        // block; the PNG and its sidecar are the synced copies under `assets/sprites/`.
        self.sheets.tong_left = load_sheet(ctx.assets, &TONG_LEFT);
        self.sheets.tong_right = load_sheet(ctx.assets, &TONG_RIGHT);
        self.sheets.meatball = load_sheet(ctx.assets, &MEATBALL);
        self.sheets.grill_left = load_sheet(ctx.assets, &GRILL_LEFT);
        self.sheets.grill_right = load_sheet(ctx.assets, &GRILL_RIGHT);
        self.sheets.pickup_flame = load_sheet(ctx.assets, &PICKUP_FLAME);
        self.sheets.pickup_knife = load_sheet(ctx.assets, &PICKUP_KNIFE);
        self.sheets.court = load_sheet(ctx.assets, &COURT_TILE);
        self.sheets.court_edge = load_sheet(ctx.assets, &COURT_EDGE);

        // Demo SFX for the web-audio slice (engine H7): beeps on paddle hits.
        // Missing asset is non-fatal — the game plays silent, with one warn.
        let beep_path = std::path::Path::new(ctx.assets.base_path()).join("sounds/beep_e.wav");
        self.paddle_beep = match ctx.audio.load_sound(&beep_path) {
            Ok(handle) => Some(handle),
            Err(e) => {
                log::warn!("paddle beep failed to load: {e}");
                None
            }
        };

        // The court first: the floor everything else stands on.
        self.playfield.court = Some(spawn_court(ctx.world, &self.sheets.court));

        // Walls: one continuous collider each, drawn by a row of 64 px strips.
        let wall_y = WIN_H / 2.0 - WALL_INSET;
        self.playfield.wall_strips.extend(spawn_wall_strips(
            ctx.world, &self.sheets.court_edge, &COURT_EDGE, "Top Wall Strip", wall_y));
        self.playfield.wall_strips.extend(spawn_wall_strips(
            ctx.world, &self.sheets.court_edge, &COURT_EDGE, "Bottom Wall Strip", -wall_y));
        spawn_wall(ctx.world, "Top Wall", wall_y);
        spawn_wall(ctx.world, "Bottom Wall", -wall_y);

        self.playfield.left_paddle = Some(spawn_paddle(
            ctx.world, "Left Paddle", -PADDLE_X, &TONG_LEFT, &self.sheets.tong_left,
            Facing::Up));
        self.playfield.right_paddle = Some(spawn_paddle(
            ctx.world, "Right Paddle", PADDLE_X, &TONG_RIGHT, &self.sheets.tong_right,
            Facing::Down));
        self.playfield.left_grill = Some(spawn_grill(
            ctx.world, "Left Grill", -GRILL_X, &GRILL_LEFT, &self.sheets.grill_left));
        self.playfield.right_grill = Some(spawn_grill(
            ctx.world, "Right Grill", GRILL_X, &GRILL_RIGHT, &self.sheets.grill_right));
        self.balls.primary = Some(self.spawn_ball(ctx.world, "Ball"));

        self.playfield.left_goal = Some(spawn_goal_sensor(ctx.world, "Left Goal", -GOAL_SENSOR_X));
        self.playfield.right_goal = Some(spawn_goal_sensor(ctx.world, "Right Goal", GOAL_SENSOR_X));

        // The grid is the engine's now (D5): it simulates and draws the backdrop
        // entity, and gameplay events queue impulses into it.
        self.playfield.backdrop = Some(spawn_backdrop(ctx.world, &self.current_theme()));
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.frame_count = self.frame_count.wrapping_add(1);

        match self.state.clone() {
            GameState::TitleScreen { selection } => self.update_title_input(ctx, selection),
            GameState::DifficultySelect { selection } => self.update_difficulty_input(ctx, selection),
            GameState::ChaosSelect { selection } => self.update_chaos_input(ctx, selection),
            GameState::Achievements => self.update_achievements_input(ctx),
            _ => self.update_gameplay(ctx),
        }

        self.update_entity_visibility(ctx);
        self.draw_ui(ctx);
    }
}
