//! The in-match update loop, split by responsibility:
//!
//! - [`paddles`] — player input and CPU AI paddle movement
//! - [`jaws`] — the jaws' intent, the facing, and the collider the frame wears
//! - [`balls`] — ball velocity maintenance, extra-ball spawning/teardown
//! - [`scoring`] — goal detection, point awards, win condition
//! - [`flow`] — match lifecycle (serve, game over, reset, entity visibility)

mod balls;
mod flow;
mod jaws;
mod paddles;
mod scoring;

use engine_core::prelude::*;
use crate::constants::*;
use crate::types::*;

pub(crate) fn entity_position(world: &World, entity: EntityId) -> Option<Vec2> {
    world.get::<Transform2D>(entity).map(|t| t.position)
}

pub(crate) fn entity_y(world: &World, entity: EntityId) -> f32 {
    world.get::<Transform2D>(entity).map(|t| t.position.y).unwrap_or(0.0)
}

/// Move `entity`'s clip machine to `state`. An entity without one is left alone, as is
/// an unknown state name (the machine warns and holds its ground). Re-asserting the
/// state the machine is already in does not restart its clip.
pub(crate) fn set_clip_state(world: &mut World, entity: EntityId, state: &str) {
    if let Some(machine) = world.get_mut::<ClipStateMachine>(entity) {
        let _ = machine.transition_to(state);
    }
}

/// Push a radial shockwave into the court's backdrop grid (paddle hits, goals). The
/// engine applies it to every backdrop on its next running frame.
pub(crate) fn ripple_grid(world: &mut World, position: Vec2, strength: f32, radius: f32) {
    ripple(world, GridImpulse::Radial { position, strength, radius, attractive: false });
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
                PauseAction::ExitGame => { ctx.request_exit(); return; }
                // Skip the rest of the frame so the resuming keypress can't
                // leak into gameplay; the world unfreezes next frame.
                PauseAction::Resumed => return,
                PauseAction::Idle => {}
            }
            if self.pause.is_active() {
                // The frozen scene stays visible under the pause overlay: the engine
                // holds the backdrop grid still with the rest of the world (it steps on
                // the time-scaled delta), so only the collider overlay is still ours.
                self.emit_collider_overlay(ctx);
                return;
            }
        }

        self.update_paddles(ctx);
        // The jaws are worked before the step, so the pose a frame draws is the
        // pose its physics runs against.
        self.update_tongs(ctx);
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
        self.update_speed_boost(ctx);
        self.check_win_condition(ctx);

        self.emit_collider_overlay(ctx);
    }

    /// Outline every collider in bright magenta while F1 is on. The backdrop grid is
    /// the engine's to draw now, so the collider overlay is the only line-buffer work
    /// the game still owns — and it draws last, over the grid and the art.
    pub(crate) fn emit_collider_overlay(&self, ctx: &mut GameContext) {
        if self.debug_colliders {
            debug::draw_colliders(
                ctx.world, ctx.lines, DEBUG_COLLIDER_COLOR, DEBUG_COLLIDER_EMISSIVE);
        }
    }
}
