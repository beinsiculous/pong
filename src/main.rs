mod achievements;
mod constants;
mod effects;
mod gameplay;
mod menu;
mod power_ups;
mod spawning;
mod types;
mod ui;

use engine_core::prelude::*;
use constants::*;
use spawning::*;
use types::*;

impl Game for PongGame {
    fn init(&mut self, ctx: &mut GameContext) {
        let font_path = engine_core::game_root!().join("assets/fonts/font.ttf");
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

fn main() {
    // Anchor assets and saves to the game's directory so launching from any
    // working directory behaves the same.
    let root = engine_core::game_root!();
    let config = GameConfig::new("Insiculous Pong")
        .with_size(WIN_W as u32, WIN_H as u32)
        .with_clear_color(0.0, 0.0, 0.0, 1.0)
        .with_fps(60)
        .with_asset_base_path(root.join("assets").to_string_lossy())
        .with_achievement_save_path(root.join("saves/pong_achievements.json").to_string_lossy())
        .with_input_settings_path(root.join("saves/input_settings.json").to_string_lossy());

    // With `--features editor` the game runs inside the scene editor
    // (hierarchy, inspector, gizmos, play/pause/stop, collider overlay);
    // without it the game runs bare. Same game code either way.
    #[cfg(feature = "editor")]
    editor_integration::run_game_with_editor(PongGame::default(), config).unwrap();
    #[cfg(not(feature = "editor"))]
    run_game(PongGame::default(), config).unwrap();
}
