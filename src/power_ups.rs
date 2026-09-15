//! Power-up timing and pickup effects. Entity creation lives in
//! `spawning.rs`; the multi-ball spawn itself lives in `gameplay::balls`.

use engine_core::prelude::*;
use crate::constants::*;
use crate::gameplay::{entity_position, set_clip_state};
use crate::spawning::{collect_machine, spawn_effect};
use crate::types::*;

/// The positions of the pickups a collection took, in the tracker's own order.
///
/// `Pickups::collect` reports and removes exactly the same pickups in the same order,
/// so a caller zips the two to learn where each collected pickup was standing — by the
/// time the call returns, that entity and its position are gone from the world.
fn collected_positions(tracked: &[(EntityId, Vec2)], remaining: &[EntityId]) -> Vec<Vec2> {
    tracked
        .iter()
        .filter(|(entity, _)| !remaining.contains(entity))
        .map(|(_, position)| *position)
        .collect()
}

impl PongGame {
    pub(crate) fn update_powerup_spawns(&mut self, ctx: &mut GameContext) {
        if !matches!(self.state, GameState::Playing) {
            return;
        }
        if self.power_ups.active.len() >= MAX_POWERUPS {
            return;
        }

        self.power_ups.spawn_timer -= ctx.delta_time;
        if self.power_ups.spawn_timer > 0.0 {
            return;
        }

        // Pick random kind
        let kind = if hash_u32(self.frame_count).is_multiple_of(2) {
            PowerUpKind::SpeedBoost
        } else {
            PowerUpKind::MultiBall
        };

        // Random position in the middle area (avoid paddles and edges)
        let x = hash_f32(self.frame_count.wrapping_add(1)) * 400.0 - 200.0;
        let y = hash_f32(self.frame_count.wrapping_add(2)) * 400.0 - 200.0;
        self.spawn_power_up(ctx.world, kind, Vec2::new(x, y));

        // Reset timer to random interval
        let t = hash_f32(self.frame_count.wrapping_add(3));
        self.power_ups.spawn_timer = POWERUP_SPAWN_MIN + t * (POWERUP_SPAWN_MAX - POWERUP_SPAWN_MIN);
    }

    pub(crate) fn check_powerup_collisions(
        &mut self,
        ctx: &mut GameContext,
        collisions: &[CollisionData],
    ) {
        let all_balls = self.balls.all();

        // Where the live pickups are, taken before the collection destroys the ones it
        // reaches: the puff that replaces a pickup has to stand where the pickup stood.
        let tracked: Vec<(EntityId, Vec2)> = self
            .power_ups
            .active
            .entities()
            .filter_map(|entity| entity_position(ctx.world, entity).map(|position| (entity, position)))
            .collect();

        // Engine-side collection: each pickup grants its effect exactly once, even if
        // two balls touch it in the same frame.
        let collected =
            self.power_ups
                .active
                .collect(collisions, &all_balls, &mut self.physics, ctx.world);

        // The effects first, for every collection the engine reported — a pickup's
        // reward never waits on where its puff goes.
        for &(kind, ball_id) in &collected {
            match kind {
                PowerUpKind::SpeedBoost => {
                    // The flame toasts the meatball it caught; the boost's expiry is
                    // what un-toasts it.
                    self.power_ups.speed_boost.start(SPEED_BOOST_DURATION);
                    set_clip_state(ctx.world, ball_id, MEATBALL_TOASTED);
                }
                PowerUpKind::MultiBall => {
                    self.spawn_extra_ball(ctx, ball_id);
                }
            }
        }

        // Then the puffs, one where each collected pickup stood. `collect` reports in
        // the tracker's order and `collected_positions` keeps it, and every pickup
        // `spawn_power_up` tracks carries a `Transform2D`, so the snapshot is never short
        // and each kind pairs with its own position. A pickup draws its cell centred on
        // its collider with no anchor, so its puff does the same.
        let remaining: Vec<EntityId> = self.power_ups.active.entities().collect();
        let taken = collected_positions(&tracked, &remaining);
        for ((kind, _), position) in collected.iter().zip(taken) {
            let puff = spawn_effect(
                ctx.world, "Pickup Puff", Self::pickup_spec(*kind), self.pickup_sheet(*kind),
                position, collect_machine(), None);
            if let Some(sprite) = ctx.world.get_mut::<Sprite>(puff) {
                sprite.offset = Vec2::ZERO;
            }
            self.transient_visuals.push(puff);
        }
    }

    pub(crate) fn update_speed_boost(&mut self, ctx: &mut GameContext) {
        // The one other half of the flame's cause: when the boost runs out, the
        // meatball stops being toasted. `tick` reports expiry exactly once, so nothing
        // else in the game returns a meatball to `idle`.
        if self.power_ups.speed_boost.tick(ctx.delta_time) {
            for ball in self.balls.all() {
                set_clip_state(ctx.world, ball, MEATBALL_IDLE);
            }
        }
    }

    pub(crate) fn destroy_all_powerups(&mut self, world: &mut World) {
        self.power_ups.active.clear(&mut self.physics, world);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn started(a: EntityId, b: EntityId) -> CollisionData {
        CollisionData {
            event: CollisionEvent { entity_a: a, entity_b: b, started: true, stopped: false },
            contacts: vec![],
        }
    }

    /// Two pickups — a flame at (10, 0) and a knife at (-10, 0) — tracked, and the balls
    /// that each of them touches in the same frame.
    fn two_pickups_two_balls() -> (PongGame, World, Vec<EntityId>, Vec<CollisionData>) {
        let mut world = World::new();
        let mut game = PongGame::default();
        let flame = world.spawn().with(Transform2D::new(Vec2::new(10.0, 0.0))).id();
        let knife = world.spawn().with(Transform2D::new(Vec2::new(-10.0, 0.0))).id();
        let ball_a = world.create_entity();
        let ball_b = world.create_entity();
        game.power_ups.active.track(flame, PowerUpKind::SpeedBoost);
        game.power_ups.active.track(knife, PowerUpKind::MultiBall);
        let collisions = vec![started(ball_a, flame), started(ball_b, knife)];
        (game, world, vec![ball_a, ball_b], collisions)
    }

    #[test]
    fn test_each_collected_pickup_leaves_one_puff_at_its_own_position() {
        let (mut game, mut world, balls, collisions) = two_pickups_two_balls();

        // The same two steps `check_powerup_collisions` runs, without a `GameContext`:
        // snapshot while the pickups are still there, collect, diff.
        let tracked: Vec<(EntityId, Vec2)> = game
            .power_ups
            .active
            .entities()
            .filter_map(|entity| entity_position(&world, entity).map(|position| (entity, position)))
            .collect();
        let collected =
            game.power_ups.active.collect(&collisions, &balls, &mut game.physics, &mut world);
        let remaining: Vec<EntityId> = game.power_ups.active.entities().collect();
        let taken = collected_positions(&tracked, &remaining);

        assert_eq!(collected.len(), 2, "one collection per pickup");
        assert_eq!(taken.len(), 2, "and one position per collection");
        // `collect` reports in the tracker's order, so the flame's position pairs with
        // the flame's kind and the knife's with the knife's — the coupling the puff at
        // each position depends on.
        assert_eq!(collected[0].0, PowerUpKind::SpeedBoost);
        assert_eq!(taken[0], Vec2::new(10.0, 0.0));
        assert_eq!(collected[1].0, PowerUpKind::MultiBall);
        assert_eq!(taken[1], Vec2::new(-10.0, 0.0));
        assert!(game.power_ups.active.is_empty(), "both pickups are gone from the tracker");
    }

    #[test]
    fn test_two_balls_touching_one_pickup_are_reported_once() {
        let mut world = World::new();
        let mut game = PongGame::default();
        let flame = world.spawn().with(Transform2D::new(Vec2::new(10.0, 0.0))).id();
        let ball_a = world.create_entity();
        let ball_b = world.create_entity();
        game.power_ups.active.track(flame, PowerUpKind::SpeedBoost);

        let tracked: Vec<(EntityId, Vec2)> = game
            .power_ups
            .active
            .entities()
            .filter_map(|entity| entity_position(&world, entity).map(|position| (entity, position)))
            .collect();
        let collected = game.power_ups.active.collect(
            &[started(ball_a, flame), started(ball_b, flame)],
            &[ball_a, ball_b],
            &mut game.physics,
            &mut world,
        );
        let remaining: Vec<EntityId> = game.power_ups.active.entities().collect();

        assert_eq!(collected.len(), 1, "the pickup is collected once, whichever ball reached it");
        assert_eq!(collected_positions(&tracked, &remaining), vec![Vec2::new(10.0, 0.0)]);
    }
}
