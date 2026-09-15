//! Goal detection and scoring: how a tong and a grill react to a hit and to a goal,
//! point awards, the between-points respawn, and the match win condition.

use engine_core::prelude::*;
use crate::constants::*;
use crate::effects;
use crate::spawning::{goal_fire_machine, spawn_effect};
use crate::types::*;
use super::{clip_state_is, entity_position, ripple_grid, set_clip_state};

/// The tong on `side` chomps the ball it just returned — but only from `open`. A
/// contact during `closing` or `opening` still bounces (that is the physics' business)
/// and is not a new cause, so it neither restarts the clip nor stacks a second chomp.
fn chomp_tong(world: &mut World, tong: EntityId) {
    if clip_state_is(world, tong, TONG_OPEN) {
        set_clip_state(world, tong, TONG_CLOSING);
    }
}

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
            self.respawn_for_serve(ctx.world);
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
            // A tunnelled goal is still a goal to the side that conceded it: the grill
            // flares and the tong reacts, even though the ball is already off the court.
            self.react_to_goal_against(ctx.world, side.opposite());
            self.score.award_point(side);
            self.destroy_ball(ctx.world, ball);
            any_scored = true;
        }
        any_scored
    }

    /// React to this frame's collisions: a tong that returns a ball chomps (and an
    /// Insane-mode hit doubles that ball's speed), and any ball that crossed a goal
    /// sensor is reported with who gets the point.
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
                    chomp_tong(ctx.world, left_paddle);
                    hit_paddle = true;
                } else if collision.event.involves(b, right_paddle) {
                    self.score.last_touch = Some(Side::Right);
                    chomp_tong(ctx.world, right_paddle);
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
    /// before positions are read. The burst keeps the paddle-side colour — every
    /// sprite is the art's own now, so this is the last place a side's colour shows.
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
            let burst = effects::paddle_hit_burst(color, normal, &theme, self.sheets.white);
            ctx.particles.spawn_burst(pos, &burst);
            ripple_grid(ctx.world, pos, 240.0, 80.0);
        }
    }

    /// Award the point for one scored ball: burn it where it crossed, flare the grill
    /// and react the tong it beat, then tear the ball down.
    fn score_ball(&mut self, ctx: &mut GameContext, ball: EntityId, side: Side) {
        // Capture the ball's last position before we destroy it, so the explosion and
        // the fire land where it died.
        let crossed_at = entity_position(ctx.world, ball)
            .unwrap_or_else(|| match side {
                // Fallback: goal location, in case the entity was already gone.
                Side::Right => Vec2::new(-PADDLE_X, 0.0),
                Side::Left => Vec2::new(PADDLE_X, 0.0),
            });
        let theme = self.current_theme();
        // Explosion takes the *scorer's* color — visual reward for the player.
        let explosion_color = match side {
            Side::Left => LEFT_COLOR,
            Side::Right => RIGHT_COLOR,
        };
        let explosion = effects::goal_explosion(explosion_color, &theme, self.sheets.white);
        ctx.particles.spawn_burst(crossed_at, &explosion);
        ripple_grid(ctx.world, crossed_at, 800.0, 180.0);

        // The meatball burns where it crossed: a looping effect with a lifetime of its
        // own, so it outlives the immediate respawn (`respawn_for_serve` is called in
        // this same frame) and is ended by the next serve or by its timer.
        let fire = spawn_effect(
            ctx.world, "Goal Fire", &MEATBALL, &self.sheets.meatball, crossed_at,
            goal_fire_machine(), Some(GOAL_FIRE_LIFETIME));
        self.transient_visuals.push(fire);

        self.react_to_goal_against(ctx.world, side.opposite());

        self.score.award_point(side);
        self.destroy_ball(ctx.world, ball);
    }

    /// The side that conceded a goal: its grill flares and its tong takes the angry
    /// pose. `scored_on` outranks the chomp a contact may have started this frame, and
    /// the tong refuses the next contact until it has played out.
    fn react_to_goal_against(&mut self, world: &mut World, conceded: Side) {
        if let Some(grill) = self.playfield.grill(conceded) {
            set_clip_state(world, grill, GRILL_SCORE);
        }
        if let Some(tong) = self.playfield.paddle(conceded) {
            set_clip_state(world, tong, TONG_SCORED_ON);
        }
    }

    /// All balls gone — clear transient match state and spawn a fresh primary ball at
    /// center for the next serve. The detached effects are the serve's to clear, not
    /// this one's: a goal's fire is meant to burn through `Serving`.
    pub(crate) fn respawn_for_serve(&mut self, world: &mut World) {
        self.destroy_all_powerups(world);
        self.power_ups.speed_boost.stop();
        self.score.last_touch = None;
        self.balls.speed_mult.clear();

        let fresh = self.spawn_ball(world, "Ball");
        self.balls.primary = Some(fresh);
        self.physics.reset_body(fresh, Vec2::ZERO);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spawning::tong_machine;

    /// A tong with the real state table and a stand-in animation carrying the real clip
    /// names at ten frames a second — two frames for the looping `open`, three for each
    /// one-shot, four for `scored_on`. One step is therefore one 100 ms frame.
    fn tong(world: &mut World) -> EntityId {
        world.spawn()
            .with(SpriteAnimation::new(SheetGrid::new(4, 1))
                .with_clip(TONG_OPEN, AnimationClip::new(vec![0, 1], 10.0))
                .with_clip(TONG_CLOSING, AnimationClip::new(vec![0, 1, 2], 10.0).with_looping(false))
                .with_clip(TONG_OPENING, AnimationClip::new(vec![0, 1, 2], 10.0).with_looping(false))
                .with_clip(TONG_SCORED_ON, AnimationClip::new(vec![0, 1, 2, 3], 10.0).with_looping(false)))
            .with(tong_machine())
            .id()
    }

    /// One engine frame: advance the animations, then let the clip machine look at
    /// them — the order the engine's frame tail runs them in.
    fn step(world: &mut World, frames: usize) {
        for _ in 0..frames {
            for entity in world.entities() {
                if let Some(animation) = world.get_mut::<SpriteAnimation>(entity) {
                    animation.update(0.1);
                }
            }
            ClipStateMachineSystem.update(world, 0.1);
        }
    }

    fn state_of(world: &World, entity: EntityId) -> String {
        world.get::<ClipStateMachine>(entity).expect("the entity has a machine").state().to_string()
    }

    fn clip_of(world: &World, entity: EntityId) -> Option<String> {
        world.get::<SpriteAnimation>(entity).and_then(|animation| animation.current_clip.clone())
    }

    #[test]
    fn test_a_contact_chomps_once_and_a_second_contact_mid_chomp_does_not_restart_it() {
        let mut world = World::new();
        let tong = tong(&mut world);
        // The machine selects its starting state's clip on the first frame it runs.
        step(&mut world, 1);
        assert_eq!(state_of(&world, tong), TONG_OPEN);

        // The first contact: the chomp starts.
        chomp_tong(&mut world, tong);
        step(&mut world, 1);
        let mut played = vec![(state_of(&world, tong), clip_of(&world, tong))];

        // A second contact 100 ms later, mid-chomp: refused, and the clip runs on.
        chomp_tong(&mut world, tong);
        step(&mut world, 1);
        played.push((state_of(&world, tong), clip_of(&world, tong)));

        // The chomp plays out: `closing` for its three frames, `opening` for its three,
        // then back to `open`. A restart would show up here as a fourth `closing`.
        for _ in 0..5 {
            step(&mut world, 1);
            played.push((state_of(&world, tong), clip_of(&world, tong)));
        }

        assert_eq!(
            played,
            vec![
                (TONG_CLOSING.to_string(), Some(TONG_CLOSING.to_string())),
                (TONG_CLOSING.to_string(), Some(TONG_CLOSING.to_string())),
                (TONG_CLOSING.to_string(), Some(TONG_CLOSING.to_string())),
                (TONG_OPENING.to_string(), Some(TONG_OPENING.to_string())),
                (TONG_OPENING.to_string(), Some(TONG_OPENING.to_string())),
                (TONG_OPENING.to_string(), Some(TONG_OPENING.to_string())),
                (TONG_OPEN.to_string(), Some(TONG_OPEN.to_string())),
            ],
            "one chomp, one pass through each clip: the second contact restarted nothing"
        );
    }

    #[test]
    fn test_a_goal_outranks_a_contact_and_plays_scored_on_through_before_reopening() {
        let mut world = World::new();
        let tong = tong(&mut world);
        step(&mut world, 1);

        // The goal against lands, and a contact arrives while the tong is reacting to
        // it: a tong that conceded is not `open`, so the contact is refused.
        set_clip_state(&mut world, tong, TONG_SCORED_ON);
        step(&mut world, 1);
        assert_eq!(state_of(&world, tong), TONG_SCORED_ON);
        chomp_tong(&mut world, tong);
        assert_eq!(state_of(&world, tong), TONG_SCORED_ON, "a goal outranks a contact");

        // The pose plays out once and the tong reopens, ready for the next ball.
        step(&mut world, 6);
        assert_eq!(state_of(&world, tong), TONG_OPEN);
        assert_eq!(clip_of(&world, tong).as_deref(), Some(TONG_OPEN));
    }
}
