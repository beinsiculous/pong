//! All entity creation. Every entity gets a `Name` component so the editor
//! hierarchy shows "Left Tong" instead of "Entity 7".
//!
//! Every art entity is built the same way: the sheet's first cell, the cell-sized
//! transform scale, and the sheet's measured anchor as the sprite offset, so the
//! artwork lands on the entity rather than at the cell's corner. Entities that move
//! under their own rules — the tongs, the meatball, the pickups — carry a
//! `ClipStateMachine` whose table names their states and clips.

use engine_core::prelude::*;
use crate::constants::*;
use crate::jaw::jaw_collider;
use crate::types::*;

/// The components every animated art entity carries: the sheet's first cell as its
/// sprite, the sheet's clips as its animation, and the anchor offset that puts the
/// reference frame on the entity. Which clip plays is the machine's business.
fn art_components(sheet: &SpriteSheet, spec: &SheetSpec) -> (Sprite, SpriteAnimation) {
    (sheet.sprite().with_offset(spec.sprite_offset()), sheet.animation())
}

/// A tong's states: five clips in each of the two facings, ten states in all.
///
/// The jaw opens and shuts as two one-shots — `closing` runs into `closed`, `opening`
/// back into `open` — and both ends of that are resting states the player holds it in.
/// A goal against plays `scored_on` and returns the tong to the jaw it rests at, which
/// is the open one for every tong but Easy's CPU, whose flat, classic return is built
/// in here (D13).
pub(crate) fn tong_machine(facing: Facing, rest: Jaw) -> ClipStateMachine {
    let rest_clip = rest.clip();
    let mut states = Vec::with_capacity(10);
    for side in [Facing::Up, Facing::Down] {
        let open = tong_state(TONG_OPEN, side);
        let closing = tong_state(TONG_CLOSING, side);
        let closed = tong_state(TONG_CLOSED, side);
        let opening = tong_state(TONG_OPENING, side);
        let scored_on = tong_state(TONG_SCORED_ON, side);
        states.push((open.clone(), ClipState::staying(open.clone())));
        states.push((closing.clone(), ClipState::new(closing, OnFinished::Next(closed.clone()))));
        states.push((closed.clone(), ClipState::staying(closed)));
        states.push((opening.clone(), ClipState::new(opening, OnFinished::Next(open))));
        states.push((
            scored_on.clone(),
            ClipState::new(scored_on, OnFinished::Next(tong_state(rest_clip, side))),
        ));
    }
    ClipStateMachine::new(tong_state(rest_clip, facing), states)
}

/// The meatball's states: `toasted` while a flame's speed boost runs, `idle`
/// otherwise. Both hold their last frame — the boost's timer is what ends the toast.
pub(crate) fn meatball_machine() -> ClipStateMachine {
    ClipStateMachine::new(
        MEATBALL_IDLE,
        vec![
            (MEATBALL_IDLE.to_string(), ClipState::staying(MEATBALL_IDLE)),
            (MEATBALL_TOASTED.to_string(), ClipState::staying(MEATBALL_TOASTED)),
        ],
    )
}

/// A grill's states: idling, and the flare a goal against plays.
pub(crate) fn grill_machine() -> ClipStateMachine {
    ClipStateMachine::new(
        GRILL_IDLE,
        vec![
            (GRILL_IDLE.to_string(), ClipState::staying(GRILL_IDLE)),
            (
                GRILL_SCORE.to_string(),
                ClipState::new(GRILL_SCORE, OnFinished::Next(GRILL_IDLE.to_string())),
            ),
        ],
    )
}

/// A pickup's state while it waits on the court. The `collect` clip belongs to the
/// puff that replaces it, not to the pickup: the engine's `Pickups::collect` destroys
/// the pickup and its body in the same call.
pub(crate) fn pickup_machine() -> ClipStateMachine {
    ClipStateMachine::new(PICKUP_IDLE, vec![(PICKUP_IDLE.to_string(), ClipState::staying(PICKUP_IDLE))])
}

/// The one-shot a collected pickup leaves behind: it plays `collect` and despawns.
pub(crate) fn collect_machine() -> ClipStateMachine {
    ClipStateMachine::new(
        PICKUP_COLLECT,
        vec![(PICKUP_COLLECT.to_string(), ClipState::new(PICKUP_COLLECT, OnFinished::Despawn))],
    )
}

/// The scored meatball's fire: `on_fire` loops, and the entity's own `Lifetime` is
/// what ends it, so the burn lasts exactly as long as the constant says.
pub(crate) fn goal_fire_machine() -> ClipStateMachine {
    ClipStateMachine::new(
        MEATBALL_ON_FIRE,
        vec![(MEATBALL_ON_FIRE.to_string(), ClipState::staying(MEATBALL_ON_FIRE))],
    )
}

/// Spawn a detached one-shot effect at `position`: the puff where a pickup was
/// collected, or the fire a scored meatball leaves. Sprite-only, and it ends itself —
/// a clip's `Despawn`, or `Lifetime` for a looping clip. The caller keeps the handle:
/// `PongGame::transient_visuals` is what lets a serve or a reset end it early. The
/// sheet's anchor applies, as it does to the entity the effect stands in for; a caller
/// whose entity applied none (a pickup) clears the offset after spawning.
pub(crate) fn spawn_effect(
    world: &mut World,
    name: &str,
    spec: &SheetSpec,
    sheet: &SpriteSheet,
    position: Vec2,
    machine: ClipStateMachine,
    lifetime: Option<f32>,
) -> EntityId {
    let (sprite, animation) = art_components(sheet, spec);
    let builder = world.spawn()
        .with(Name::new(name))
        .with(Transform2D::from_parts(position, 0.0, spec.scale()))
        .with(sprite)
        .with(animation)
        .with(machine);
    match lifetime {
        Some(seconds) => builder.with(Lifetime::new(seconds)).id(),
        None => builder.id(),
    }
}

/// Spawn one tong at `x`, facing `facing`. The left and right tongs are separate
/// sheets — the art carries both halves of the character — so nothing is mirrored
/// through a negative `Sprite.scale`; the `_down` clips are drawn cells, and the
/// collider's `_down` form is their mirror.
///
/// The tong is born resting on an open jaw, wearing that pose's collider, so the
/// outline is the art's from the first frame rather than one frame behind it. A match
/// start re-lays the jaw its difficulty rests at.
pub(crate) fn spawn_paddle(
    world: &mut World,
    name: &str,
    x: f32,
    spec: &SheetSpec,
    sheet: &SpriteSheet,
    facing: Facing,
) -> EntityId {
    let (sprite, animation) = art_components(sheet, spec);
    world.spawn()
        .with(Name::new(name))
        .with(Transform2D::from_parts(Vec2::new(x, 0.0), 0.0, spec.scale()))
        .with(sprite)
        .with(animation)
        .with(tong_machine(facing, Jaw::Open))
        .with(RigidBody::new_kinematic().with_rotation_locked(true))
        .with(Collider::new(jaw_collider(Jaw::Open, facing))
            .with_friction(0.0)
            .with_restitution(1.0))
        .id()
}

/// Re-lay a tong's jaw: the machine that rests at this jaw and returns to it after a
/// goal, and the collider that jaw wears. A jaw's rest is part of the machine's table
/// rather than a runtime flag, so a tong laid at another jaw gets a new machine — and,
/// the machine starting afresh, its clip is selected again from the top.
pub(crate) fn lay_tong_jaw(world: &mut World, tong: EntityId, facing: Facing, rest: Jaw) {
    if let Some(machine) = world.get_mut::<ClipStateMachine>(tong) {
        *machine = tong_machine(facing, rest);
    }
    if let Some(collider) = world.get_mut::<Collider>(tong) {
        collider.shape = jaw_collider(rest, facing);
    }
}

/// The backdrop grid's colour for a chaos theme: its grid colour at the low alpha D5
/// asks for, so the lattice reads over the court without veiling it.
pub(crate) fn backdrop_color(theme: &ChaosTheme) -> Vec4 {
    let grid = theme.grid_color;
    Vec4::new(grid.x, grid.y, grid.z, BACKDROP_ALPHA)
}

/// Spawn one wall's collider: a single continuous box the width of the court. The
/// strips that draw it are separate, visual-only entities — abutting strip-sized boxes
/// would give the ball a seam to catch on.
pub(crate) fn spawn_wall(world: &mut World, name: &str, y: f32) -> EntityId {
    world.spawn()
        .with(Name::new(name))
        .with(Transform2D::from_parts(Vec2::new(0.0, y), 0.0, Vec2::ONE))
        .with(RigidBody::new_static())
        .with(Collider::box_collider(WIN_W, WALL_THICKNESS).with_friction(0.0).with_restitution(1.0))
        .id()
}

/// Spawn the `ceil(WIN_W / cell)` strips that draw one wall along its centre line,
/// centred on the window. The court's edge sits at `COURT_HALF_W`, so the outermost
/// pair is half a strip past the visible area — the rail reads as continuous.
pub(crate) fn spawn_wall_strips(
    world: &mut World,
    sheet: &SpriteSheet,
    spec: &SheetSpec,
    name_prefix: &str,
    y: f32,
) -> Vec<EntityId> {
    let count = (WIN_W / spec.cell.x).ceil() as u32;
    let first_x = -(count as f32 - 1.0) * spec.cell.x / 2.0;
    let mut strips = Vec::with_capacity(count as usize);
    for index in 0..count {
        let x = first_x + index as f32 * spec.cell.x;
        strips.push(
            world.spawn()
                .with(Name::new(format!("{name_prefix} {index}")))
                .with(Transform2D::from_parts(Vec2::new(x, y), 0.0, spec.scale()))
                .with(sheet.sprite().with_offset(spec.sprite_offset()).with_depth(WALL_DEPTH))
                .id(),
        );
    }
    strips
}

/// Spawn the grill behind one tong at `x` (`-GRILL_X` on the left, `GRILL_X` on the
/// right): visual only — the goal sensor is what scores.
pub(crate) fn spawn_grill(
    world: &mut World,
    name: &str,
    x: f32,
    spec: &SheetSpec,
    sheet: &SpriteSheet,
) -> EntityId {
    let (sprite, animation) = art_components(sheet, spec);
    world.spawn()
        .with(Name::new(name))
        .with(Transform2D::from_parts(Vec2::new(x, 0.0), 0.0, spec.scale()))
        .with(sprite.with_depth(GRILL_DEPTH))
        .with(animation)
        .with(grill_machine())
        .id()
}

pub(crate) fn spawn_goal_sensor(world: &mut World, name: &str, x: f32) -> EntityId {
    world.spawn()
        .with(Name::new(name))
        .with(Transform2D::new(Vec2::new(x, 0.0)))
        .with(RigidBody::new_static())
        .with(Collider::box_collider(GOAL_SENSOR_W, WIN_H).as_sensor())
        .id()
}

/// Spawn the court floor: a `Tilemap` of the court tile, `ceil(WIN / tile)` cells each
/// way, centred on the window. Tiles are placed in pixel units from the entity's
/// position, which anchors the centre of tile (0, 0) — the top-left cell, with rows
/// growing downward.
pub(crate) fn spawn_court(world: &mut World, sheet: &SpriteSheet) -> EntityId {
    let columns = (WIN_W / COURT_TILE_PX).ceil() as u32;
    let rows = (WIN_H / COURT_TILE_PX).ceil() as u32;

    let mut map = Tilemap::new(columns, rows, COURT_TILE_PX);
    map.tileset = sheet.texture.id;
    // The court sheet is one cell; a sheet with more would cut into that many.
    map.tile_uv_size = Vec2::new(1.0 / sheet.grid.cols as f32, 1.0 / sheet.grid.rows as f32);
    map.tiles = vec![1; (columns * rows) as usize];
    map.depth = COURT_TILE_DEPTH;

    let anchor = Vec2::new(
        -(columns as f32 - 1.0) * COURT_TILE_PX / 2.0,
        (rows as f32 - 1.0) * COURT_TILE_PX / 2.0,
    );
    world.spawn().with(Name::new("Court")).with(Transform2D::new(anchor)).with(map).id()
}

/// Spawn the deforming grid drawn over the court (D5): the engine simulates and draws
/// it, gameplay events ripple it.
pub(crate) fn spawn_backdrop(world: &mut World, theme: &ChaosTheme) -> EntityId {
    world.spawn()
        .with(Name::new("Grid Backdrop"))
        .with(Transform2D::new(Vec2::ZERO))
        .with(GridBackdrop {
            draw_order: GridDrawOrder::OverSprites,
            color: backdrop_color(theme),
            ..GridBackdrop::default()
        })
        .id()
}


impl PongGame {
    /// Spawn a ball: the meatball sheet, its body's circle collider, and the idle /
    /// toasted states. The collider is a true circle so reflection off a tong's capsule
    /// caps matches what the player sees.
    pub(crate) fn spawn_ball(&self, world: &mut World, name: &str) -> EntityId {
        let spec = &MEATBALL;
        let sheet = &self.sheets.meatball;
        let (sprite, animation) = art_components(sheet, spec);
        world.spawn()
            .with(Name::new(name))
            .with(Transform2D::from_parts(Vec2::ZERO, 0.0, spec.scale()))
            .with(sprite)
            .with(animation)
            .with(meatball_machine())
            .with(RigidBody::new_dynamic()
                .with_gravity_scale(0.0)
                .with_rotation_locked(true)
                .with_linear_damping(0.0)
                .with_angular_damping(0.0)
                .with_ccd(true))
            .with(Collider::circle_collider(BALL_RADIUS)
                .with_friction(0.0)
                .with_restitution(1.0))
            .id()
    }

    /// The loaded sheet a pickup kind wears.
    pub(crate) fn pickup_sheet(&self, kind: PowerUpKind) -> &SpriteSheet {
        match kind {
            PowerUpKind::SpeedBoost => &self.sheets.pickup_flame,
            PowerUpKind::MultiBall => &self.sheets.pickup_knife,
        }
    }

    /// The measured anchor a pickup kind's sheet carries. The section keeps a pickup's
    /// 32 px cell centred on its unchanged collider, so this is deliberately never
    /// applied — it exists to say which subject the exception belongs to.
    pub(crate) fn pickup_spec(kind: PowerUpKind) -> &'static SheetSpec {
        match kind {
            PowerUpKind::SpeedBoost => &PICKUP_FLAME,
            PowerUpKind::MultiBall => &PICKUP_KNIFE,
        }
    }

    /// Editor-hierarchy name for the next extra ball ("Ball 2", "Ball 3", ...).
    pub(crate) fn next_extra_ball_name(&self) -> String {
        format!("Ball {}", self.balls.extras.len() + 2)
    }

    /// Spawn a power-up pickup at `pos`, with the sheet its kind wears, and track it.
    /// The art's 32 px cell is drawn centred on the unchanged `POWERUP_SIZE` collider,
    /// so the subject's own measured anchor is deliberately not applied.
    pub(crate) fn spawn_power_up(&mut self, world: &mut World, kind: PowerUpKind, pos: Vec2) {
        let spec = Self::pickup_spec(kind);
        let (sprite, animation) = art_components(self.pickup_sheet(kind), spec);
        let entity = world.spawn()
            .with(Name::new(kind.entity_name()))
            .with(Transform2D::from_parts(pos, 0.0, spec.scale()))
            .with(sprite.with_offset(Vec2::ZERO))
            .with(animation)
            .with(pickup_machine())
            .with(RigidBody::new_static())
            .with(Collider::circle_collider(POWERUP_SIZE / 2.0).as_sensor())
            .id();
        self.power_ups.active.track(entity, kind);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-cell sheet with no clips: enough to spawn an art entity headlessly.
    fn bare_sheet() -> SpriteSheet {
        SpriteSheet {
            texture: TextureHandle { id: 1 },
            grid: SheetGrid::new(1, 1),
            clips: Vec::new(),
            path: String::new(),
        }
    }

    #[test]
    fn test_a_tong_machine_names_ten_clips_and_returns_to_the_jaw_it_rests_at() {
        let clips = [TONG_OPEN, TONG_CLOSING, TONG_CLOSED, TONG_OPENING, TONG_SCORED_ON];
        for facing in [Facing::Up, Facing::Down] {
            for rest in [Jaw::Open, Jaw::Closed] {
                let machine = tong_machine(facing, rest);
                assert_eq!(machine.states().len(), 10, "five clips in each of two facings");
                assert_eq!(
                    machine.state(),
                    tong_state(rest.clip(), facing),
                    "a tong is born at the jaw its match rests it at"
                );

                let row = |clip: &str| {
                    let state = tong_state(clip, facing);
                    machine
                        .states()
                        .iter()
                        .find(|(name, _)| *name == state)
                        .map(|(_, row)| row.clone())
                        .unwrap_or_else(|| panic!("'{state}' is a state of this machine"))
                };
                for clip in clips {
                    // The state's name is the clip's name: the sidecar is the contract.
                    assert_eq!(row(clip).clip, tong_state(clip, facing));
                }

                assert!(matches!(row(TONG_OPEN).on_finished, OnFinished::Stay));
                assert!(matches!(row(TONG_CLOSED).on_finished, OnFinished::Stay));
                assert_eq!(
                    row(TONG_CLOSING).on_finished,
                    OnFinished::Next(tong_state(TONG_CLOSED, facing)),
                    "a close ends shut"
                );
                assert_eq!(
                    row(TONG_OPENING).on_finished,
                    OnFinished::Next(tong_state(TONG_OPEN, facing)),
                    "an opening ends open"
                );
                assert_eq!(
                    row(TONG_SCORED_ON).on_finished,
                    OnFinished::Next(tong_state(rest.clip(), facing)),
                    "and a goal played out comes back to the jaw the match rests at"
                );
            }
        }
    }

    #[test]
    fn test_the_grills_stand_behind_their_own_tongs() {
        let mut world = World::new();
        let sheet = bare_sheet();
        let left = spawn_grill(&mut world, "Left Grill", -GRILL_X, &GRILL_LEFT, &sheet);
        let right = spawn_grill(&mut world, "Right Grill", GRILL_X, &GRILL_RIGHT, &sheet);
        let x_of = |entity| world.get::<Transform2D>(entity).expect("a grill has a transform").position.x;
        assert_eq!(x_of(left), -GRILL_X, "the left grill is behind the left tong");
        assert_eq!(x_of(right), GRILL_X, "the right grill is behind the right tong");
    }
}
