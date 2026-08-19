//! The in-match update loop, split by responsibility:
//!
//! - [`paddles`] — player input and CPU AI paddle movement
//! - [`balls`] — ball velocity maintenance, extra-ball spawning/teardown
//! - [`scoring`] — goal detection, point awards, win condition
//! - [`flow`] — match lifecycle (serve, game over, reset, entity visibility)

mod balls;
mod flow;
mod paddles;
mod scoring;

use engine_core::prelude::*;
use crate::types::*;

pub(crate) fn entity_position(world: &World, entity: EntityId) -> Option<Vec2> {
    world.get::<Transform2D>(entity).map(|t| t.position)
}

pub(crate) fn entity_y(world: &World, entity: EntityId) -> f32 {
    world.get::<Transform2D>(entity).map(|t| t.position.y).unwrap_or(0.0)
}

impl PongGame {
    pub(crate) fn update_gameplay(&mut self, ctx: &mut GameContext) {
        if self.balls.primary.is_none()
            || self.playfield.left_paddle.is_none()
            || self.playfield.right_paddle.is_none()
        {
            return;
        }

        // F1 toggles the collider debug overlay. Magenta outlines render on
        // top of sprites so any sprite-vs-collider mismatch is obvious.
        if ctx.input.is_key_just_pressed(KeyCode::F1) {
            self.debug_colliders = !self.debug_colliders;
        }

        // Pause gate: while paused the whole match is frozen — no physics
        // step, no input, no timers; the overlay is drawn in the UI pass.
        if matches!(self.state, GameState::Serving | GameState::Playing) {
            let action = self.pause.update(ctx.players, ctx.input, ctx.window_size);
            ctx.time_scale = self.pause.time_scale();
            match action {
                PauseAction::Restart => { self.start_game(ctx.world); return; }
                PauseAction::QuitToTitle => { self.reset_to_title(ctx.world); return; }
                PauseAction::ExitGame => { ctx.exit_requested = true; return; }
                // Skip the rest of the frame so the resuming keypress can't
                // leak into gameplay; the world unfreezes next frame.
                PauseAction::Resumed => return,
                PauseAction::Idle => {}
            }
            if self.pause.is_active() {
                // Keep the frozen scene visible under the pause overlay:
                // re-emit the grid without advancing it (dt 0).
                engine_core::grid::step_and_emit_grid(
                    self.grid.as_mut(), ctx.world, ctx.lines, 0.0, self.debug_colliders,
                );
                return;
            }
        }

        self.update_paddles(ctx);
        self.physics.update(ctx.world, ctx.delta_time);

        // Drain this frame's collision events once (take = the buffer is
        // consumed, not borrowed). Every consumer below shares this Vec, and
        // no borrow of `self.physics` is held while reacting.
        let collisions: Vec<CollisionData> = self.physics.take_collision_events();

        self.handle_gameplay_input(ctx);
        self.maintain_all_ball_velocities();
        self.check_goals(ctx, &collisions);
        self.check_powerup_collisions(ctx, &collisions);
        self.update_powerup_spawns(ctx);
        self.update_speed_boost(ctx.delta_time);
        self.check_win_condition(ctx);

        // Step + render the deforming grid after gameplay so it reacts to
        // this frame's collisions.
        engine_core::grid::step_and_emit_grid(
            self.grid.as_mut(), ctx.world, ctx.lines, ctx.delta_time, self.debug_colliders,
        );
    }

    /// Push a radial shockwave into the deforming grid (paddle hits, goals).
    pub(crate) fn ripple_grid(&mut self, position: Vec2, strength: f32, radius: f32) {
        if let Some(grid) = self.grid.as_mut() {
            grid.apply_impulse(&GridImpulse::Radial {
                position,
                strength,
                radius,
                attractive: false,
            });
        }
    }
}
