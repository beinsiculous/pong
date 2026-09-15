//! Ball housekeeping: keeping every ball at its intended speed, spawning
//! extra balls (multi-ball power-up), and tearing balls down.

use engine_core::prelude::*;
use crate::constants::*;
use crate::types::*;
use super::entity_position;

/// Direction for a freshly served ball: fixed horizontal sign, pseudo-random
/// vertical spread derived from the frame counter. `salt` decorrelates
/// multiple balls served on the same frame.
pub(crate) fn serve_direction(frame: u32, salt: u32, dir_x: f32) -> Vec2 {
    let hash = frame.wrapping_mul(2654435761).wrapping_add(salt);
    let t = ((hash >> 16) as f32) / 65535.0;
    Vec2::new(dir_x, t * 1.2 - 0.6).normalize()
}

impl PongGame {
    pub(crate) fn maintain_all_ball_velocities(&mut self) {
        for ball in self.balls.all() {
            self.maintain_ball_velocity(ball);
        }
    }

    /// Re-pin a ball's horizontal speed to its target (base speed × boosts)
    /// and clamp vertical speed, so restitution noise never slows the rally
    /// down or turns it into a vertical stalemate.
    fn maintain_ball_velocity(&mut self, ball: EntityId) {
        let Some((vel, _)) = self.physics.get_body_velocity(ball) else { return };
        if vel.x.abs() < 0.1 { return; }

        let boost = if self.power_ups.speed_boost.active() { SPEED_BOOST_MULTIPLIER } else { 1.0 };
        let chaos_mult = self.balls.speed_mult.get(&ball).copied().unwrap_or(1.0);

        let speed = BALL_INITIAL_SPEED * boost * chaos_mult;
        let max_vy = BALL_MAX_SPEED * boost * chaos_mult;

        let fixed_vx = vel.x.signum() * speed;
        let vy = vel.y.clamp(-max_vy, max_vy);
        let new_vel = Vec2::new(fixed_vx, vy);

        if (new_vel - vel).length() > 1.0 {
            self.physics.set_velocity(ball, new_vel, 0.0);
        }
    }

    /// Multi-ball power-up: spawn a new ball at the source ball's position,
    /// fired toward the opposite side — the meatball visibly splits.
    pub(crate) fn spawn_extra_ball(&mut self, ctx: &mut GameContext, source_ball: EntityId) {
        let pos = entity_position(ctx.world, source_ball).unwrap_or(Vec2::ZERO);
        let source_vx = self.physics.get_body_velocity(source_ball)
            .map(|(v, _)| v.x)
            .unwrap_or(BALL_INITIAL_SPEED);

        let name = self.next_extra_ball_name();
        let entity = self.spawn_ball(ctx.world, &name);
        if let Some(transform) = ctx.world.get_mut::<Transform2D>(entity) {
            transform.position = pos;
        }

        // Use the same direction formula as the initial serve so multi-ball
        // spawns have real vertical spread instead of a shallow mirror of the
        // source ball's trajectory. Fires opposite the source for gameplay.
        let dir_x = if source_vx >= 0.0 { -1.0 } else { 1.0 };
        let dir = serve_direction(self.frame_count, 0xB5297A4D, dir_x);
        self.physics.set_velocity(entity, dir * BALL_INITIAL_SPEED, 0.0);

        self.balls.extras.push(entity);
    }

    /// Destroy one ball and drop all bookkeeping for it (promoting an extra
    /// to primary if needed).
    pub(crate) fn destroy_ball(&mut self, world: &mut World, ball: EntityId) {
        self.balls.remove(ball);
        self.physics.destroy_entity(world, ball);
    }

    pub(crate) fn destroy_all_extra_balls(&mut self, world: &mut World) {
        for ball in std::mem::take(&mut self.balls.extras) {
            self.balls.speed_mult.remove(&ball);
            self.physics.destroy_entity(world, ball);
        }
    }
}
