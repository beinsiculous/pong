//! Match lifecycle: serving, game-over input, starting/leaving a match,
//! position resets, and hiding gameplay entities while menus are up.

use engine_core::prelude::*;
use crate::constants::*;
use crate::spawning::backdrop_color;
use crate::types::*;
use super::balls::serve_direction;
use super::set_clip_state;

impl PongGame {
    /// State transitions during a match: serve from `Serving`, restart or
    /// bail to the title screen from `GameOver`. Menu (Escape/pad Start)
    /// during Serving/Playing is handled by the pause gate upstream.
    pub(crate) fn handle_gameplay_input(&mut self, ctx: &mut GameContext) {
        let primary = ctx.players.just_activated_any(GameAction::Action1, ctx.input);
        let menu = ctx.players.just_activated_any(GameAction::Menu, ctx.input);
        match &self.state {
            GameState::Serving => {
                if primary {
                    self.serve(ctx.world);
                }
            }
            GameState::GameOver { .. } => {
                if primary {
                    self.start_game(ctx.world);
                } else if menu {
                    self.reset_to_title(ctx.world);
                }
            }
            _ => {}
        }
    }

    /// Launch the primary ball toward the last scorer's opponent. In
    /// Ridiculous mode a second ball heads the opposite way so each player
    /// gets one incoming.
    ///
    /// Whoever launches — the lone player, or either one in two-player — the launch is
    /// a clean slate: the detached effects a point left behind end here, which is how a
    /// goal's fire stops burning.
    fn serve(&mut self, world: &mut World) {
        self.clear_transient_visuals(world);

        let Some(ball) = self.balls.primary else { return };
        let dir_x = match self.score.last_scorer {
            Side::Left => -1.0,
            Side::Right => 1.0,
        };
        let dir = serve_direction(self.frame_count, 0, dir_x);
        self.physics.set_velocity(ball, dir * BALL_INITIAL_SPEED, 0.0);

        if self.settings.chaos.is_ridiculous() {
            let name = self.next_extra_ball_name();
            let extra = self.spawn_ball(world, &name);
            let dir2 = serve_direction(self.frame_count, 0x9E37, -dir_x);
            self.physics.set_velocity(extra, dir2 * BALL_INITIAL_SPEED, 0.0);
            self.balls.extras.push(extra);
        }

        self.state = GameState::Playing;
    }

    /// Begin a fresh match with the current settings: zero the score, drop all
    /// transient state, put every meatball back to idling, and wait for the serve.
    pub(crate) fn start_game(&mut self, world: &mut World) {
        self.score.reset();
        self.clear_transient_visuals(world);
        self.destroy_all_extra_balls(world);
        self.destroy_all_powerups(world);
        self.power_ups = PowerUpState::default();
        self.balls.speed_mult.clear();
        self.apply_backdrop_theme(world);
        // A restart reuses the meatball already in play: one that was still toasted when
        // the last match ended — its boost long stopped — would otherwise stay toasted
        // for good, since only the timer's expiry un-toasts.
        for ball in self.balls.all() {
            set_clip_state(world, ball, MEATBALL_IDLE);
        }
        self.reset_positions();
        self.state = GameState::Serving;
    }

    pub(crate) fn reset_to_title(&mut self, world: &mut World) {
        self.clear_transient_visuals(world);
        self.destroy_all_extra_balls(world);
        self.destroy_all_powerups(world);
        self.power_ups.speed_boost.stop();
        self.reset_positions();
        self.state = GameState::TitleScreen { selection: 0 };
    }

    /// Give the backdrop grid the chosen chaos mode's colour. The mode is picked after
    /// `init()` spawned the backdrop, so the colour it was born with is Normal's; this
    /// is the one themed surface left on the court, and it follows the pick from here.
    pub(crate) fn apply_backdrop_theme(&self, world: &mut World) {
        let color = backdrop_color(&self.current_theme());
        if let Some(backdrop) = self.playfield.backdrop {
            if let Some(grid) = world.get_mut::<GridBackdrop>(backdrop) {
                grid.color = color;
            }
        }
    }

    /// Drop every detached effect entity. An id the world has already dropped — a puff
    /// whose `collect` clip ran out, a fire whose lifetime expired — is skipped, so a
    /// stale list is a normal state rather than an error.
    pub(crate) fn clear_transient_visuals(&mut self, world: &mut World) {
        for entity in self.transient_visuals.drain(..) {
            world.remove_entity(&entity).ok();
        }
    }

    pub(crate) fn reset_positions(&mut self) {
        if let Some(ball) = self.balls.primary {
            self.physics.reset_body(ball, Vec2::ZERO);
        }
        if let Some(lp) = self.playfield.left_paddle {
            self.physics.set_kinematic_target(lp, Vec2::new(-PADDLE_X, 0.0), 0.0);
        }
        if let Some(rp) = self.playfield.right_paddle {
            self.physics.set_kinematic_target(rp, Vec2::new(PADDLE_X, 0.0), 0.0);
        }
    }

    /// Gameplay sprites only render during a match — menus get the bare court. The
    /// backdrop grid follows the same rule: it belongs to the match the way the tongs
    /// do, and it is the only thing here that is not a sprite.
    pub(crate) fn update_entity_visibility(&self, ctx: &mut GameContext) {
        let visible = !matches!(
            self.state,
            GameState::TitleScreen { .. }
                | GameState::DifficultySelect { .. }
                | GameState::ChaosSelect { .. }
                | GameState::Achievements
        );
        let entities = [self.playfield.left_paddle, self.playfield.right_paddle].into_iter().flatten()
            .chain(self.balls.all())
            .chain(self.playfield.wall_strips.iter().copied())
            .chain(self.playfield.left_grill)
            .chain(self.playfield.right_grill)
            .chain(self.power_ups.active.entities())
            .chain(self.transient_visuals.iter().copied());
        set_sprites_visible(ctx.world, entities, visible);
        if let Some(backdrop) = self.playfield.backdrop {
            if let Some(grid) = ctx.world.get_mut::<GridBackdrop>(backdrop) {
                grid.visible = visible;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spawning::{goal_fire_machine, spawn_effect};

    /// A one-cell sheet carrying the meatball's clip names, so the machines that run
    /// against it select a clip the engine knows.
    fn meatball_sheet() -> SpriteSheet {
        SpriteSheet {
            texture: TextureHandle { id: 1 },
            grid: SheetGrid::new(1, 1),
            clips: vec![
                (MEATBALL_IDLE.to_string(), AnimationClip::new(vec![0], 10.0)),
                (MEATBALL_TOASTED.to_string(), AnimationClip::new(vec![0], 10.0)),
                (MEATBALL_ON_FIRE.to_string(), AnimationClip::new(vec![0], 10.0)),
            ],
            path: String::new(),
        }
    }

    /// The state `score_ball` leaves behind: a fresh meatball served, and the fire its
    /// point lit.
    fn scored_goal() -> (PongGame, World, EntityId) {
        let mut world = World::new();
        let mut game = PongGame::default();
        game.sheets.meatball = meatball_sheet();
        game.balls.primary = Some(game.spawn_ball(&mut world, "Ball"));
        game.state = GameState::Serving;

        let fire = spawn_effect(
            &mut world, "Goal Fire", &MEATBALL, &game.sheets.meatball, Vec2::ZERO,
            goal_fire_machine(), Some(GOAL_FIRE_LIFETIME));
        game.transient_visuals.push(fire);
        (game, world, fire)
    }

    #[test]
    fn test_a_goal_fire_burns_through_serving_and_ends_at_the_serve() {
        let (mut game, mut world, fire) = scored_goal();

        // A goal calls `respawn_for_serve` in the same frame it scores; the fire is not
        // that function's to clear.
        game.respawn_for_serve(&mut world);
        assert!(world.entities().contains(&fire), "the fire burns on through Serving");
        assert_eq!(game.state, GameState::Serving);

        // The launch is where it ends.
        game.serve(&mut world);
        assert!(!world.entities().contains(&fire), "the serve clears what the point left");
        assert!(game.transient_visuals.is_empty());
    }

    #[test]
    fn test_a_goal_fire_expires_on_its_own_with_another_ball_still_live() {
        let (mut game, mut world, fire) = scored_goal();
        let extra = game.spawn_ball(&mut world, "Ball 2");
        game.balls.extras.push(extra);

        // A test may own an engine system the game must not: the frame tail already
        // runs lifetimes, and a second one would halve every lifetime in play.
        let mut lifetimes = LifetimeSystem;
        lifetimes.update(&mut world, GOAL_FIRE_LIFETIME - 0.1);
        assert!(world.entities().contains(&fire), "still burning just before its time");

        lifetimes.update(&mut world, 0.2);
        assert!(!world.entities().contains(&fire), "the fire ends itself");
        assert!(world.entities().contains(&extra), "and takes nothing else with it");
    }

    #[test]
    fn test_starting_a_match_gives_the_backdrop_the_chosen_modes_colour() {
        use crate::spawning::spawn_backdrop;
        let mut world = World::new();
        let mut game = PongGame::default();
        // Spawned as `init()` spawns it: with whatever mode the game booted in.
        let born_with = game.current_theme();
        game.playfield.backdrop = Some(spawn_backdrop(&mut world, &born_with));

        game.settings.chaos = ChaosMode::Insane;
        game.start_game(&mut world);

        let grid = world.get::<GridBackdrop>(game.playfield.backdrop.expect("backdrop")).expect("grid");
        assert_eq!(grid.color, backdrop_color(&game.current_theme()), "the pick reaches the grid");
        assert_ne!(grid.color, backdrop_color(&born_with), "and it is not the boot mode's colour");
    }

    #[test]
    fn test_clearing_the_transient_visuals_tolerates_an_effect_that_already_despawned() {
        let (mut game, mut world, fire) = scored_goal();
        world.remove_entity(&fire).ok();

        // The list is allowed to be stale: an effect ends itself, and the game only
        // finds out when it tries to clear it.
        game.clear_transient_visuals(&mut world);

        assert!(game.transient_visuals.is_empty());
    }
}
