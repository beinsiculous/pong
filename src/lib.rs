//! Insiculous Pong — game crate.
//!
//! The library owns the whole game (`PongGame` + its `Game` impl) so both
//! entry points stay thin: `main.rs` (native window, filesystem saves,
//! optional editor) and `web_entry.rs` (wasm-bindgen start: fetch assets,
//! then the same `run_game`). This split also keeps `editor_integration`
//! out of the library — editor wiring lives in `main.rs` only.

mod achievements;
mod constants;
mod effects;
mod gameplay;
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
        .with_asset_base_path(asset_base)
}

impl Game for PongGame {
    fn init(&mut self, ctx: &mut GameContext) {
        // Resolve against the configured asset base so the same relative
        // path works natively (game dir) and on the web (VFS keys).
        let font_path = std::path::Path::new(ctx.assets.base_path()).join("fonts/font.ttf");
        if let Ok(font) = ctx.ui.load_font_file(&font_path.to_string_lossy()) {
            ctx.ui.set_default_font(font);
        }

        // Names/descriptions come from the locale tables; the title menu's
        // Language item re-registers on locale switches.
        achievements::register_all(ctx.achievements, ctx.strings);

        let tex = ctx.assets.create_solid_color(1, 1, [255, 255, 255, 255]).unwrap();
        self.textures.white = tex.id;
        // Relative paths resolve against the asset base path set in main().
        self.textures.paddle = ctx.assets.load_texture("paddle_16px.png")
            .expect("missing assets/paddle_16px.png").id;
        self.textures.ball = ctx.assets.load_texture("ball_8px.png")
            .expect("missing assets/ball_8px.png").id;

        let theme = self.current_theme();
        self.playfield.background = Some(spawn_background(
            ctx.world, tex.id, theme.bg_color, Vec2::new(WIN_W, WIN_H)));

        // Left paddle: rounded face naturally on the right (toward the ball).
        // Right paddle: mirror so its rounded face points left (toward the ball).
        self.playfield.left_paddle = Some(spawn_paddle(
            ctx.world, "Left Paddle", -PADDLE_X, self.textures.paddle, LEFT_COLOR, false));
        self.playfield.right_paddle = Some(spawn_paddle(
            ctx.world, "Right Paddle", PADDLE_X, self.textures.paddle, RIGHT_COLOR, true));
        self.balls.primary = Some(self.spawn_ball(ctx.world, "Ball"));

        let wall_y = WIN_H / 2.0 - 10.0;
        self.playfield.walls.push(spawn_wall(
            ctx.world, "Top Wall", Vec2::new(0.0, wall_y), WIN_W, 20.0, tex.id, theme.structure_color));
        self.playfield.walls.push(spawn_wall(
            ctx.world, "Bottom Wall", Vec2::new(0.0, -wall_y), WIN_W, 20.0, tex.id, theme.structure_color));

        let goal_x = WIN_W / 2.0 + 10.0;
        self.playfield.left_goal = Some(spawn_goal_sensor(ctx.world, "Left Goal", -goal_x));
        self.playfield.right_goal = Some(spawn_goal_sensor(ctx.world, "Right Goal", goal_x));

        // Build the deforming grid background.
        self.grid = Some(default_playfield_grid(&theme));
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
