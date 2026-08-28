//! Goal detection and scoring: paddle-hit reactions, point awards, the
//! between-points respawn, and the match win condition.

use engine_core::prelude::*;
use crate::constants::*;
use crate::effects;
use crate::types::*;
use super::entity_position;

impl PongGame {
    pub(crate) fn check_goals(&mut self, ctx: &mut GameContext, collisions: &[CollisionData]) {
        if self.playfield.left_goal.is_none()
            || self.playfield.right_goal.is_none()
            || self.playfield.left_paddle.is_none()
            || self.playfield.right_paddle.is_none()
        {
            return;
        }

        let any_escape_scored = self.score_escaped_balls(ctx);

        // Work from the post-escape-cleanup set so the logic below never
        // sees a torn-down ball.
        let all_balls = self.balls.all();

        let balls_scored = self.handle_paddle_hits_and_collect_goals(ctx, collisions, &all_balls);
        self.spawn_paddle_hit_visuals(ctx, collisions, &all_balls);

        for &(ball, side) in &balls_scored {
            self.score_ball(ctx, ball, side);
        }

        // If no balls remain, reset to serving
        if self.balls.is_empty() && (any_escape_scored || !balls_scored.is_empty()) {
            self.respawn_for_serve(ctx);
        }
    }

    /// Safety net: any ball whose transform escaped the playfield (or went
    /// NaN/infinite) counts as scored for the opposite side. At extreme
    /// Insane-mode speeds a ball can tunnel past the goal sensor before
    /// the physics engine emits an intersection event. Tear these down
    /// IMMEDIATELY so the rest of the frame (collision loop, paddle-hit
    /// boosts, powerup checks) never operates on an already-dead ball.
    fn score_escaped_balls(&mut self, ctx: &mut GameContext) -> bool {
        let mut any_scored = false;
        let bound_x = WIN_W / 2.0 + 60.0;
        let bound_y = WIN_H / 2.0 + 60.0;
        for ball in self.balls.all() {
            let Some(pos) = entity_position(ctx.world, ball) else { continue };
            let escaped = !pos.x.is_finite()
                || !pos.y.is_finite()
                || pos.x.abs() > bound_x
                || pos.y.abs() > bound_y;
            if !escaped { continue; }

            let side = if pos.x >= 0.0 { Side::Left } else { Side::Right };
            self.score.award_point(side);
            self.destroy_ball(ctx.world, ball);
            any_scored = true;
        }
        any_scored
    }

    /// React to this frame's collisions: tint balls to the color of the
    /// paddle that touched them (applying the Insane speed-doubling), and
    /// return which balls crossed a goal sensor and who gets the point.
    fn handle_paddle_hits_and_collect_goals(
        &mut self,
        ctx: &mut GameContext,
        collisions: &[CollisionData],
        all_balls: &[EntityId],
    ) -> Vec<(EntityId, Side)> {
        let left_paddle = self.playfield.left_paddle.unwrap();
        let right_paddle = self.playfield.right_paddle.unwrap();
        let left_goal = self.playfield.left_goal.unwrap();
        let right_goal = self.playfield.right_goal.unwrap();

        let insane = self.settings.chaos.is_insane();
        let mut balls_scored: Vec<(EntityId, Side)> = Vec::new();
        let mut paddle_hits: Vec<EntityId> = Vec::new();
        for collision in collisions {
            if !collision.event.started { continue; }
            for &b in all_balls {
                let mut hit_paddle = false;
                if collision.event.involves(b, left_paddle) {
                    self.score.last_touch = Some(Side::Left);
                    if let Some(sprite) = ctx.world.get_mut::<Sprite>(b) {
                        sprite.color = LEFT_COLOR;
                    }
                    hit_paddle = true;
                } else if collision.event.involves(b, right_paddle) {
                    self.score.last_touch = Some(Side::Right);
                    if let Some(sprite) = ctx.world.get_mut::<Sprite>(b) {
                        sprite.color = RIGHT_COLOR;
                    }
                    hit_paddle = true;
                }
                if hit_paddle {
                    if let Some(beep) = self.paddle_beep {
                        ctx.audio.play(beep).ok();
                    }
                    if insane {
                        paddle_hits.push(b);
                    }
                }
                let already_scored = balls_scored.iter().any(|(bb, _)| *bb == b);
                if !already_scored {
                    if collision.event.involves(b, left_goal) {
                        balls_scored.push((b, Side::Right));
                    } else if collision.event.involves(b, right_goal) {
                        balls_scored.push((b, Side::Left));
                    }
                }
            }
        }

        // Apply Insane speed doubling — bump the per-ball multiplier, then
        // immediately boost current velocity so the new clamp takes effect.
        for b in paddle_hits {
            let mult = self.balls.speed_mult.entry(b).or_insert(1.0);
            *mult *= 2.0;
            if let Some((vel, ang)) = self.physics.get_body_velocity(b) {
                self.physics.set_velocity(b, vel * 2.0, ang);
            }
        }

        balls_scored
    }

    /// Spawn paddle-hit visuals: a directional particle burst plus a grid
    /// ripple. Runs after the speed-boost pass so velocities are settled
    /// before positions are read.
    fn spawn_paddle_hit_visuals(
        &mut self,
        ctx: &mut GameContext,
        collisions: &[CollisionData],
        all_balls: &[EntityId],
    ) {
        let left_paddle = self.playfield.left_paddle.unwrap();
        let right_paddle = self.playfield.right_paddle.unwrap();
        let theme = self.current_theme();

        let mut hit_events: Vec<(Vec2, Vec4, Vec2)> = Vec::new();
        for collision in collisions {
            if !collision.event.started { continue; }
            for &b in all_balls {
                let (paddle_color, paddle_x) = if collision.event.involves(b, left_paddle) {
                    (LEFT_COLOR, -PADDLE_X)
                } else if collision.event.involves(b, right_paddle) {
                    (RIGHT_COLOR, PADDLE_X)
                } else {
                    continue;
                };
                let Some(ball_pos) = entity_position(ctx.world, b) else { continue };
                // Normal points from the paddle toward the ball — i.e. the
                // direction the ball is bouncing in. That's the cone direction
                // for the spray.
                let normal = (ball_pos - Vec2::new(paddle_x, ball_pos.y)).normalize_or_zero();
                hit_events.push((ball_pos, paddle_color, normal));
            }
        }
        for (pos, color, normal) in hit_events {
            let burst = effects::paddle_hit_burst(color, normal, &theme, self.textures.white);
            ctx.particles.spawn_burst(pos, &burst);
            self.ripple_grid(pos, 240.0, 80.0);
        }
    }

    /// Award the point for one scored ball, fire the explosion, and tear the
    /// ball down.
    fn score_ball(&mut self, ctx: &mut GameContext, ball: EntityId, side: Side) {
        // Capture the ball's last position before we destroy it so the
        // explosion burst spawns at the right place.
        let explosion_pos = entity_position(ctx.world, ball)
            .unwrap_or_else(|| match side {
                // Fallback: goal location, in case the entity was already gone.
                Side::Right => Vec2::new(-PADDLE_X, 0.0),
                Side::Left => Vec2::new(PADDLE_X, 0.0),
            });
        // Explosion takes the *scorer's* color — visual reward for the player.
        let explosion_color = match side {
            Side::Left => LEFT_COLOR,
            Side::Right => RIGHT_COLOR,
        };
        let theme = self.current_theme();
        let explosion = effects::goal_explosion(explosion_color, &theme, self.textures.white);
        ctx.particles.spawn_burst(explosion_pos, &explosion);
        self.ripple_grid(explosion_pos, 800.0, 180.0);

        self.score.award_point(side);
        self.destroy_ball(ctx.world, ball);
    }

    /// All balls gone — clear transient match state and spawn a fresh
    /// primary ball at center for the next serve.
    fn respawn_for_serve(&mut self, ctx: &mut GameContext) {
        self.destroy_all_powerups(ctx.world);
        self.power_ups.speed_boost.stop();
        self.score.last_touch = None;
        self.balls.speed_mult.clear();

        let fresh = self.spawn_ball(ctx.world, "Ball");
        self.balls.primary = Some(fresh);
        self.physics.reset_body(fresh, Vec2::ZERO);
        let ball_color = self.current_theme().accent_color;
        if let Some(s) = ctx.world.get_mut::<Sprite>(fresh) {
            s.color = ball_color;
        }
        self.state = GameState::Serving;
    }

    pub(crate) fn check_win_condition(&mut self, ctx: &mut GameContext) {
        if !matches!(self.state, GameState::Playing | GameState::Serving) { return; }

        let winner = if self.score.left >= WIN_SCORE {
            Some(true)
        } else if self.score.right >= WIN_SCORE {
            Some(false)
        } else {
            None
        };

        if let Some(left_wins) = winner {
            self.destroy_all_extra_balls(ctx.world);
            self.destroy_all_powerups(ctx.world);
            self.power_ups.speed_boost.stop();
            self.unlock_win_achievements(ctx, left_wins);
            self.state = GameState::GameOver { left_wins };
        }
    }
}
