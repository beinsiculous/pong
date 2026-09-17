//! Paddle movement: player control via the engine's per-player input
//! bindings (keyboard, dpad, or analog stick), ball-tracking AI for the CPU.

use engine_core::prelude::*;
use crate::constants::*;
use crate::types::*;
use super::entity_y;

/// Paddle speed from a merged movement axis in `-1.0..=1.0` (+1 = up).
/// Analog input scales the speed; overdriven input is clamped.
fn paddle_dy(axis: f32) -> f32 {
    axis.clamp(-1.0, 1.0) * PADDLE_SPEED
}

impl PongGame {
    pub(crate) fn update_paddles(&mut self, ctx: &GameContext) {
        let (Some(left), Some(right)) = (self.playfield.left_paddle, self.playfield.right_paddle)
        else { return };
        self.update_left_paddle(ctx, left);
        self.update_right_paddle(ctx, right);
    }

    fn update_left_paddle(&mut self, ctx: &GameContext, paddle: EntityId) {
        self.move_paddle(ctx, paddle, -PADDLE_X, paddle_dy(self.tong_stick(ctx, Side::Left).y));
    }

    fn update_right_paddle(&mut self, ctx: &GameContext, paddle: EntityId) {
        let dy = if self.is_cpu_tong(Side::Right) {
            self.ai_dy(ctx, paddle)
        } else {
            paddle_dy(self.tong_stick(ctx, Side::Right).y)
        };
        self.move_paddle(ctx, paddle, PADDLE_X, dy);
    }

    /// Whether the CPU plays this tong: in single player the right tong is the AI's,
    /// and in two player both tongs have a player behind them.
    pub(crate) fn is_cpu_tong(&self, side: Side) -> bool {
        self.settings.mode == GameMode::SinglePlayer && side == Side::Right
    }

    /// The stick a tong reads, both axes at once (+x right, +y up).
    ///
    /// In single player the lone human gets both players' devices (WASD, arrows, and
    /// either pad); in two player each tong is its own player's. The tong steers by
    /// `y` and works its jaw with `x`, so both come from here and one human cannot
    /// have the two read different devices.
    pub(crate) fn tong_stick(&self, ctx: &GameContext, side: Side) -> Vec2 {
        let axis = |player: PlayerId| {
            Vec2::new(
                ctx.players.move_x(player, ctx.input),
                ctx.players.move_y(player, ctx.input),
            )
        };
        match self.settings.mode {
            GameMode::SinglePlayer => axis(PlayerId::P1) + axis(PlayerId::P2),
            GameMode::TwoPlayer => axis(match side {
                Side::Left => PlayerId::P1,
                Side::Right => PlayerId::P2,
            }),
        }
    }

    /// CPU control: chase the primary ball's Y at the difficulty's speed,
    /// with a dead zone so easier CPUs wobble less precisely.
    pub(crate) fn ai_dy(&self, ctx: &GameContext, paddle: EntityId) -> f32 {
        let Some(ball) = self.balls.primary else { return 0.0 };
        let diff = entity_y(ctx.world, ball) - entity_y(ctx.world, paddle);
        if diff.abs() > self.settings.difficulty.ai_dead_zone() {
            diff.signum() * self.settings.difficulty.ai_speed()
        } else {
            0.0
        }
    }

    fn move_paddle(&mut self, ctx: &GameContext, paddle: EntityId, x: f32, dy: f32) {
        let y = entity_y(ctx.world, paddle);
        let new_y = (y + dy * ctx.delta_time).clamp(-PADDLE_MAX_Y, PADDLE_MAX_Y);
        self.physics.set_kinematic_target(paddle, Vec2::new(x, new_y), 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paddle_dy_scales_linearly_with_axis() {
        assert_eq!(paddle_dy(1.0), PADDLE_SPEED);
        assert_eq!(paddle_dy(-1.0), -PADDLE_SPEED);
        assert_eq!(paddle_dy(0.5), PADDLE_SPEED * 0.5);
    }

    #[test]
    fn paddle_dy_clamps_overdriven_axis() {
        // Merged key + stick input can sum past 1.0 before clamping
        assert_eq!(paddle_dy(1.8), PADDLE_SPEED);
        assert_eq!(paddle_dy(-1.8), -PADDLE_SPEED);
    }

    #[test]
    fn paddle_dy_zero_axis_holds_still() {
        assert_eq!(paddle_dy(0.0), 0.0);
    }
}
