//! Goal detection and scoring: how a tong and a grill react to a hit and to a goal,
//! point awards, the between-points respawn, and the match win condition.

use engine_core::prelude::*;
use crate::constants::*;
use crate::effects;
use crate::spawning::{goal_fire_machine, spawn_effect};
use crate::types::*;
use super::{entity_position, ripple_grid, set_clip_state};

/// One frame's collisions, sorted into the two things scoring does with them.
pub(crate) struct FrameCollisions {
    /// Balls that arrived on a tong this frame, with the side that returned them.
    pub(crate) paddle_hits: Vec<(EntityId, Side)>,
    /// Balls a goal sensor caught, with the side that gets the point.
    pub(crate) balls_scored: Vec<(EntityId, Side)>,
}

/// The side a ball that left the court conceded on: the goal it went out past, read
/// from its own x. A position that is not a number is not on the right, so it reads as
/// the left — the rule this path has always scored by.
pub(crate) fn escaped_goal_conceded(position: Vec2) -> Side {
    if position.x >= 0.0 {
        Side::Right
    } else {
        Side::Left
    }
}

/// Where a goal's fire burns: on the **conceded** side's goal line, the meatball's
/// edge on it and in front of the grill behind the tong, at the height the ball
/// crossed — held inside the walls, and level with the court's middle when there is
/// no crossing to read (a ball the engine could not place).
///
/// The side is the goal's, never the scorer's: the fire marks the mouth that was
/// beaten (D14).
pub(crate) fn goal_fire_position(conceded: Side, crossed_at: Option<Vec2>) -> Vec2 {
    let x = match conceded {
        Side::Left => -(COURT_HALF_W - BALL_RADIUS),
        Side::Right => COURT_HALF_W - BALL_RADIUS,
    };
    let limit = COURT_HALF_H - BALL_RADIUS;
    let y = crossed_at
        .filter(|at| at.y.is_finite())
        .map_or(0.0, |at| at.y.clamp(-limit, limit));
    Vec2::new(x, y)
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

        let found = self.handle_paddle_hits_and_collect_goals(collisions, &all_balls);
        self.spawn_paddle_hit_visuals(ctx, &found.paddle_hits);

        for &(ball, side) in &found.balls_scored {
            self.score_ball(ctx, ball, side);
        }

        // If no balls remain, reset to serving
        if self.balls.is_empty() && (any_escape_scored || !found.balls_scored.is_empty()) {
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

            // A ball off the court still crossed a line: which one its own x decides.
            self.concede_goal(ctx, ball, escaped_goal_conceded(pos), Some(pos));
            any_scored = true;
        }
        any_scored
    }

    /// React to this frame's collisions: a tong that really returned a ball reports
    /// the hit (and an Insane-mode one doubles that ball's speed), and any ball that
    /// crossed a goal sensor is reported with who gets the point.
    ///
    /// Reads no context of its own: what a hit looks and sounds like is the pass that
    /// draws it, and what is decided here is which events were hits at all.
    pub(crate) fn handle_paddle_hits_and_collect_goals(
        &mut self,
        collisions: &[CollisionData],
        all_balls: &[EntityId],
    ) -> FrameCollisions {
        let left_paddle = self.playfield.left_paddle.unwrap();
        let right_paddle = self.playfield.right_paddle.unwrap();
        let left_goal = self.playfield.left_goal.unwrap();
        let right_goal = self.playfield.right_goal.unwrap();

        let insane = self.settings.chaos.is_insane();
        let mut found = FrameCollisions { paddle_hits: Vec::new(), balls_scored: Vec::new() };
        let mut insane_hits: Vec<EntityId> = Vec::new();

        // Only a `started` is a hit. The engine reports a pair by diffing the step's
        // contact set against the last one, so a jaw whose collider is rebuilt under a
        // ball it already touches emits no new start — a rebuild never echoes a hit.
        for collision in collisions {
            if !collision.event.started { continue; }
            for &b in all_balls {
                let mut hit_paddle = false;
                for (tong, side) in [(left_paddle, Side::Left), (right_paddle, Side::Right)] {
                    if !collision.event.involves(b, tong) { continue; }
                    self.score.last_touch = Some(side);
                    found.paddle_hits.push((b, side));
                    hit_paddle = true;
                }
                if hit_paddle && insane {
                    insane_hits.push(b);
                }
                let already_scored = found.balls_scored.iter().any(|(bb, _)| *bb == b);
                if !already_scored {
                    if collision.event.involves(b, left_goal) {
                        found.balls_scored.push((b, Side::Right));
                    } else if collision.event.involves(b, right_goal) {
                        found.balls_scored.push((b, Side::Left));
                    }
                }
            }
        }

        // Apply Insane speed doubling — bump the per-ball multiplier, then
        // immediately boost current velocity so the new clamp takes effect.
        for b in insane_hits {
            let mult = self.balls.speed_mult.entry(b).or_insert(1.0);
            *mult *= 2.0;
            if let Some((vel, ang)) = self.physics.get_body_velocity(b) {
                self.physics.set_velocity(b, vel * 2.0, ang);
            }
        }

        found
    }

    /// Spawn paddle-hit visuals: a directional particle burst plus a grid
    /// ripple. Runs after the speed-boost pass so velocities are settled
    /// before positions are read. The burst keeps the paddle-side colour — every
    /// sprite is the art's own now, so this is the last place a side's colour shows.
    fn spawn_paddle_hit_visuals(&mut self, ctx: &mut GameContext, hits: &[(EntityId, Side)]) {
        let theme = self.current_theme();

        let mut hit_events: Vec<(Vec2, Vec4, Vec2)> = Vec::new();
        for &(ball, side) in hits {
            if let Some(beep) = self.paddle_beep {
                ctx.audio.play(beep).ok();
            }
            let (paddle_color, paddle_x) = match side {
                Side::Left => (LEFT_COLOR, -PADDLE_X),
                Side::Right => (RIGHT_COLOR, PADDLE_X),
            };
            let Some(ball_pos) = entity_position(ctx.world, ball) else { continue };
            // Normal points from the paddle toward the ball — i.e. the
            // direction the ball is bouncing in. That's the cone direction
            // for the spray.
            let normal = (ball_pos - Vec2::new(paddle_x, ball_pos.y)).normalize_or_zero();
            hit_events.push((ball_pos, paddle_color, normal));
        }
        for (pos, color, normal) in hit_events {
            let burst = effects::paddle_hit_burst(color, normal, &theme, self.sheets.white);
            ctx.particles.spawn_burst(pos, &burst);
            ripple_grid(ctx.world, pos, 240.0, 80.0);
        }
    }

    /// A goal the sensor caught: mark where the ball died with the scorer's burst,
    /// then concede it.
    fn score_ball(&mut self, ctx: &mut GameContext, ball: EntityId, scorer: Side) {
        // Captured before the ball is destroyed, so the burst lands where it died.
        let crossed_at = entity_position(ctx.world, ball);
        if let Some(at) = crossed_at {
            let theme = self.current_theme();
            // Explosion takes the *scorer's* color — visual reward for the player.
            let explosion_color = match scorer {
                Side::Left => LEFT_COLOR,
                Side::Right => RIGHT_COLOR,
            };
            let explosion = effects::goal_explosion(explosion_color, &theme, self.sheets.white);
            ctx.particles.spawn_burst(at, &explosion);
            ripple_grid(ctx.world, at, 800.0, 180.0);
        }
        self.concede_goal(ctx, ball, scorer.opposite(), crossed_at);
    }

    /// One goal's consequences, whichever path found it: the grill behind the
    /// conceding tong flares, that tong wears its angry pose in the facing it has,
    /// the fire burns on the conceded goal line, the point is awarded, and the ball
    /// is torn down.
    ///
    /// The fire is a looping effect with a lifetime of its own, so it outlives the
    /// immediate respawn (`respawn_for_serve` is called in this same frame) and is
    /// ended by the next serve or by its timer.
    fn concede_goal(
        &mut self,
        ctx: &mut GameContext,
        ball: EntityId,
        conceded: Side,
        crossed_at: Option<Vec2>,
    ) {
        let fire_at = goal_fire_position(conceded, crossed_at);
        let fire = spawn_effect(
            ctx.world, "Goal Fire", &MEATBALL, &self.sheets.meatball, fire_at,
            goal_fire_machine(), Some(GOAL_FIRE_LIFETIME));
        self.transient_visuals.push(fire);

        self.react_to_goal_against(ctx.world, conceded);
        self.score.award_point(conceded.opposite());
        self.destroy_ball(ctx.world, ball);
    }

    /// The side that conceded a goal: its grill flares and its tong takes the angry
    /// pose, in the facing it is wearing so the reaction is drawn the way that tong
    /// stands. `scored_on` outranks the jaw's own motion, and plays out before the
    /// tong opens again.
    fn react_to_goal_against(&mut self, world: &mut World, conceded: Side) {
        if let Some(grill) = self.playfield.grill(conceded) {
            set_clip_state(world, grill, GRILL_SCORE);
        }
        if let Some(tong) = self.playfield.paddle(conceded) {
            let facing = self.tongs.get(conceded).facing;
            set_clip_state(world, tong, &tong_state(TONG_SCORED_ON, facing));
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
        // A serve is a clean slate for the jaws too: a bite asked for in the last
        // rally and since released is not carried into this one, so no tong shuts
        // itself after the twitch a goal against it plays. A stick still pushing is
        // still asking, and gets what it asks for.
        self.reset_jaw_intents();

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

    /// A game with a jaw at each end and a ball in play. The collision pass reads
    /// nothing off these entities but their ids, so none of them needs a component —
    /// only the ids have to be the ones the events name.
    fn playing() -> (PongGame, EntityId, EntityId) {
        let mut game = PongGame::default();
        let ball = EntityId::new();
        let left_tong = EntityId::new();
        game.playfield.left_paddle = Some(left_tong);
        game.playfield.right_paddle = Some(EntityId::new());
        game.playfield.left_goal = Some(EntityId::new());
        game.playfield.right_goal = Some(EntityId::new());
        game.balls.primary = Some(ball);
        (game, ball, left_tong)
    }

    /// One contact event between a ball and a tong, as the drained slice carries them.
    fn contact(ball: EntityId, tong: EntityId, started: bool, stopped: bool) -> CollisionData {
        CollisionData {
            event: CollisionEvent { entity_a: ball, entity_b: tong, started, stopped },
            contacts: Vec::new(),
        }
    }

    #[test]
    fn test_only_a_started_contact_is_a_hit() {
        let (mut game, ball, tong) = playing();

        // The ball arrives: a hit, credited to the tong that returned it.
        let arrived = [contact(ball, tong, true, false)];
        let found = game.handle_paddle_hits_and_collect_goals(&arrived, &[ball]);
        assert_eq!(found.paddle_hits, vec![(ball, Side::Left)]);
        assert_eq!(game.score.last_touch, Some(Side::Left));

        // The pair going on touching (the engine reports it every step, neither started
        // nor stopped) and the pair parting are not hits, and credit nobody.
        game.score.last_touch = None;
        let ongoing = [contact(ball, tong, false, false)];
        let found = game.handle_paddle_hits_and_collect_goals(&ongoing, &[ball]);
        assert!(found.paddle_hits.is_empty(), "an ongoing contact is not a hit");
        let parted = [contact(ball, tong, false, true)];
        let found = game.handle_paddle_hits_and_collect_goals(&parted, &[ball]);
        assert!(found.paddle_hits.is_empty(), "and neither is the pair parting");
        assert_eq!(game.score.last_touch, None);

        // A hit and its bounce in one frame — two physics sub-steps — is still one
        // hit, and the next arrival is another.
        let hit_and_gone = [contact(ball, tong, true, false), contact(ball, tong, false, true)];
        let found = game.handle_paddle_hits_and_collect_goals(&hit_and_gone, &[ball]);
        assert_eq!(found.paddle_hits, vec![(ball, Side::Left)]);
        let returned = [contact(ball, tong, true, false)];
        let found = game.handle_paddle_hits_and_collect_goals(&returned, &[ball]);
        assert_eq!(found.paddle_hits, vec![(ball, Side::Left)], "the return after it is a hit too");
    }

    #[test]
    fn test_a_goal_burns_on_the_conceded_goal_line() {
        let line = COURT_HALF_W - BALL_RADIUS;
        let wall = COURT_HALF_H - BALL_RADIUS;

        // The fire stands on the line the point was conceded on, at the height the
        // ball crossed — never at the scorer's end.
        assert_eq!(
            goal_fire_position(Side::Left, Some(Vec2::new(-COURT_HALF_W, 0.0))),
            Vec2::new(-line, 0.0)
        );
        assert_eq!(
            goal_fire_position(Side::Right, Some(Vec2::new(COURT_HALF_W, 12.0))),
            Vec2::new(line, 12.0)
        );

        // A crossing past the wall burns on the wall, and one the engine could not
        // place burns level with the court's middle rather than off the court.
        assert_eq!(
            goal_fire_position(Side::Right, Some(Vec2::new(COURT_HALF_W, WIN_H))),
            Vec2::new(line, wall)
        );
        assert_eq!(goal_fire_position(Side::Left, None), Vec2::new(-line, 0.0));
        assert_eq!(
            goal_fire_position(Side::Left, Some(Vec2::new(f32::NAN, f32::NAN))),
            Vec2::new(-line, 0.0)
        );
    }

    #[test]
    fn test_a_ball_that_left_the_court_conceded_on_the_side_it_went_out() {
        // Which goal it went past is its own x's to say, and a position that is not a
        // number is not on the right.
        assert_eq!(escaped_goal_conceded(Vec2::new(WIN_W, 0.0)), Side::Right);
        assert_eq!(escaped_goal_conceded(Vec2::new(-WIN_W, 0.0)), Side::Left);
        assert_eq!(escaped_goal_conceded(Vec2::new(0.0, WIN_H)), Side::Right);
        assert_eq!(escaped_goal_conceded(Vec2::new(f32::NAN, f32::NAN)), Side::Left);
    }
}
