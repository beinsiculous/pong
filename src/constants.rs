use engine_core::prelude::*;

pub(crate) const WIN_W: f32 = 800.0;
pub(crate) const WIN_H: f32 = 600.0;

// --- the sheets ---------------------------------------------------------------------
// The bounds are measured from the synced PNGs (the executor's `report-4.md` carries
// the measurement): the opaque box of the named reference frame inside its own cell.

const TONG_CELL: Vec2 = Vec2::new(64.0, 96.0);
/// The tong's `closed` frame — frame 5 of the sheet: 22 x 78, centred in its cell.
const TONG_CLOSED_BOUNDS: (Vec2, Vec2) = (Vec2::new(21.0, 9.0), Vec2::new(43.0, 87.0));

const MEATBALL_CELL: Vec2 = Vec2::new(48.0, 64.0);
/// The meatball's `idle` frame's body — frame 0 of the sheet: 31 x 31, sitting low so
/// `on_fire` has room above it. Identical in every `idle` and `toasted` frame, so the
/// anchor is stable across the animation.
const MEATBALL_BODY_BOUNDS: (Vec2, Vec2) = (Vec2::new(9.0, 27.0), Vec2::new(40.0, 58.0));

const GRILL_CELL: Vec2 = Vec2::new(32.0, 96.0);
/// A grill's `idle` frame — frame 0: 24 x 88, centred in its cell.
const GRILL_BOUNDS: (Vec2, Vec2) = (Vec2::new(4.0, 4.0), Vec2::new(28.0, 92.0));

const PICKUP_CELL: Vec2 = Vec2::new(32.0, 32.0);
/// The knife's `idle` frame — frame 0: 28 x 28, a pixel right of centre.
const KNIFE_IDLE_BOUNDS: (Vec2, Vec2) = (Vec2::new(3.0, 2.0), Vec2::new(31.0, 30.0));
/// The flame's `idle` frame — frame 0: 16 x 27, a pixel right of centre and 1.5 low.
const FLAME_IDLE_BOUNDS: (Vec2, Vec2) = (Vec2::new(9.0, 4.0), Vec2::new(25.0, 31.0));

const COURT_TILE_CELL: Vec2 = Vec2::new(64.0, 64.0);
const COURT_EDGE_CELL: Vec2 = Vec2::new(64.0, 16.0);

/// The left tong. The right tong is its own sheet, not a mirrored copy — the art
/// carries both halves of the character.
pub(crate) const TONG_LEFT: SheetSpec = SheetSpec {
    path: "sprites/ai_tong_left.png",
    cell: TONG_CELL,
    bounds: TONG_CLOSED_BOUNDS,
};
pub(crate) const TONG_RIGHT: SheetSpec = SheetSpec {
    path: "sprites/ai_tong_right.png",
    cell: TONG_CELL,
    bounds: TONG_CLOSED_BOUNDS,
};
pub(crate) const MEATBALL: SheetSpec = SheetSpec {
    path: "sprites/ai_tong_meatball.png",
    cell: MEATBALL_CELL,
    bounds: MEATBALL_BODY_BOUNDS,
};
pub(crate) const GRILL_LEFT: SheetSpec = SheetSpec {
    path: "sprites/ai_tong_grill_left_32x96.png",
    cell: GRILL_CELL,
    bounds: GRILL_BOUNDS,
};
pub(crate) const GRILL_RIGHT: SheetSpec = SheetSpec {
    path: "sprites/ai_tong_grill_right_32x96.png",
    cell: GRILL_CELL,
    bounds: GRILL_BOUNDS,
};
pub(crate) const PICKUP_FLAME: SheetSpec = SheetSpec {
    path: "sprites/ai_tong_pickup_flame_32x32.png",
    cell: PICKUP_CELL,
    bounds: FLAME_IDLE_BOUNDS,
};
pub(crate) const PICKUP_KNIFE: SheetSpec = SheetSpec {
    path: "sprites/ai_tong_pickup_knife_32x32.png",
    cell: PICKUP_CELL,
    bounds: KNIFE_IDLE_BOUNDS,
};
pub(crate) const COURT_TILE: SheetSpec = SheetSpec {
    path: "sprites/ai_tong_court_64x64.png",
    cell: COURT_TILE_CELL,
    bounds: (Vec2::ZERO, COURT_TILE_CELL),
};
pub(crate) const COURT_EDGE: SheetSpec = SheetSpec {
    path: "sprites/ai_tong_court_edge_64x16.png",
    cell: COURT_EDGE_CELL,
    bounds: (Vec2::ZERO, COURT_EDGE_CELL),
};

// --- the playfield ------------------------------------------------------------------
// Every number below is re-derived from the measured art above, in the order the
// geometry nests: the window, the court's edge, the walls, then the things standing
// inside it.

/// The court's edge, where the walls end and a goal line sits.
pub(crate) const COURT_HALF_W: f32 = WIN_W / 2.0;

/// The wall's centre line and the thickness of its one continuous collider. The
/// collider stays a single `box_collider(WIN_W, WALL_THICKNESS)` per wall — the
/// strips drawn on it are 64 px cells, and abutting strip-sized boxes would give the
/// ball a seam to catch on.
pub(crate) const WALL_INSET: f32 = 10.0;
pub(crate) const WALL_THICKNESS: f32 = 20.0;

/// The walls' inner face: the court's top and bottom edge.
pub(crate) const COURT_HALF_H: f32 = WIN_H / 2.0 - WALL_INSET - WALL_THICKNESS / 2.0;

/// The goal sensor: a full-height box whose inner edge sits on the goal line, so a
/// point is awarded the moment the meatball's edge crosses it.
pub(crate) const GOAL_SENSOR_W: f32 = 20.0;
pub(crate) const GOAL_SENSOR_X: f32 = COURT_HALF_W + GOAL_SENSOR_W / 2.0;

/// The tong's collider — the closed tong's measured box as a vertical capsule: 22
/// wide, 78 tall. Its rounded caps give an edge hit a real angle, and its footprint
/// is exactly the art's.
pub(crate) const PADDLE_W: f32 = TONG_CLOSED_BOUNDS.1.x - TONG_CLOSED_BOUNDS.0.x;
pub(crate) const PADDLE_H: f32 = TONG_CLOSED_BOUNDS.1.y - TONG_CLOSED_BOUNDS.0.y;


/// The meatball's collider: the body's measured width, and one circle on it.
pub(crate) const BALL_SIZE: f32 = MEATBALL_BODY_BOUNDS.1.x - MEATBALL_BODY_BOUNDS.0.x;
pub(crate) const BALL_RADIUS: f32 = BALL_SIZE / 2.0;

/// Whole pixels of daylight between a meatball resting on a tong's face and the goal
/// line. The tong's face is `PADDLE_X + PADDLE_W/2`, a resting meatball reaches
/// `BALL_SIZE` further, and the goal sensor's box begins at the line: without this
/// clearance a ball the player is holding would already be scoring.
const GOAL_LINE_DAYLIGHT: f32 = 8.0;

/// The tongs' centre. The section derived 350: the closed tong's 64 px cell then
/// reaches 382, its collider box 361 and a resting meatball's far edge 392 — all
/// inside the goal line at 400, where the un-narrowed 370 left the cell at 402 and a
/// resting meatball overlapping the sensor.
pub(crate) const PADDLE_X: f32 = COURT_HALF_W - PADDLE_W / 2.0 - BALL_SIZE - GOAL_LINE_DAYLIGHT;

/// How far a tong may travel before its art would cross the wall.
pub(crate) const PADDLE_MAX_Y: f32 = COURT_HALF_H - PADDLE_H / 2.0;

pub(crate) const PADDLE_SPEED: f32 = 450.0;

pub(crate) const BALL_INITIAL_SPEED: f32 = 250.0;
pub(crate) const BALL_MAX_SPEED: f32 = 500.0;

/// The grill behind a tong: its 32 px cell flush with the goal line, drawn behind
/// the tong so the goal mouth reads as fire behind the player.
pub(crate) const GRILL_X: f32 = COURT_HALF_W - GRILL_CELL.x / 2.0;

/// Depth of the court floor, of the wall strips, and of the grills. Sprites default
/// to 0; each of these sits below the one above it.
pub(crate) const COURT_TILE_DEPTH: f32 = -2.0;
pub(crate) const WALL_DEPTH: f32 = -1.0;
pub(crate) const GRILL_DEPTH: f32 = -0.5;

/// The court floor's tile, which also sets how many cells cover the window — the
/// counts are `ceil(WIN / COURT_TILE_PX)` each way, computed where the map is built.
pub(crate) const COURT_TILE_PX: f32 = 64.0;

/// A goal's fire: the scored meatball burns at its last position for this long.
pub(crate) const GOAL_FIRE_LIFETIME: f32 = 1.5;

/// The backdrop grid's alpha, well under the preset's resting value: the lattice
/// reads over the court art without veiling it (D5).
pub(crate) const BACKDROP_ALPHA: f32 = 0.25;

pub(crate) const WIN_SCORE: u32 = 7;

/// Paddle-side colours, now only the tint of a paddle-hit particle burst — every art
/// sprite is drawn white, so a side's colour shows nowhere else.
pub(crate) const LEFT_COLOR: Vec4 = Vec4::new(1.0, 0.3, 0.3, 1.0);
pub(crate) const RIGHT_COLOR: Vec4 = Vec4::new(0.3, 0.5, 1.0, 1.0);

/// Collider outlines for the F1 overlay: bright magenta at a high emissive so they
/// bloom over the art the way they did over the neon sprites.
pub(crate) const DEBUG_COLLIDER_COLOR: Vec4 = Vec4::new(1.0, 0.2, 1.0, 0.9);
pub(crate) const DEBUG_COLLIDER_EMISSIVE: f32 = 2.0;

// Power-ups
/// The pickup collider. Unchanged at 24 while the art's 32 px cell is drawn centred
/// on it, so the pickup's own one-pixel anchor is deliberately not applied.
pub(crate) const POWERUP_SIZE: f32 = 24.0;
pub(crate) const SPEED_BOOST_DURATION: f32 = 5.0;
pub(crate) const SPEED_BOOST_MULTIPLIER: f32 = 1.8;
pub(crate) const POWERUP_SPAWN_MIN: f32 = 5.0;
pub(crate) const POWERUP_SPAWN_MAX: f32 = 12.0;
pub(crate) const POWERUP_INITIAL_DELAY: f32 = 8.0;
pub(crate) const MAX_POWERUPS: usize = 3;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spawning::{
        collect_machine, goal_fire_machine, grill_machine, meatball_machine, pickup_machine,
        tong_machine,
    };
    use crate::types::{Facing, Jaw};
    use engine_core::assets::sprite_sheet::prepare_sheet;

    /// The reference frame's opaque size, as the sheet declares it.
    fn footprint(spec: &SheetSpec) -> Vec2 {
        spec.bounds.1 - spec.bounds.0
    }

    /// Every tong machine a sheet has to answer for: both facings, at both jaws a tong
    /// can rest at.
    fn tong_machines() -> Vec<ClipStateMachine> {
        let mut machines = Vec::new();
        for facing in [Facing::Up, Facing::Down] {
            for rest in [Jaw::Open, Jaw::Closed] {
                machines.push(tong_machine(facing, rest));
            }
        }
        machines
    }

    #[test]
    fn test_every_state_plays_a_clip_its_sheet_really_has() {
        // The synced PNGs and their sidecars, read through the engine's own GPU-free
        // load path — the one check that ties the game's tables to the committed art.
        // The cell in the spec, over the real PNG, must also cut the sheet into the
        // rows and columns the art really has: a wrong cell would show up here as the
        // wrong grid, not as a silently stretched sprite.
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let sheets = [
            (&TONG_LEFT, tong_machines(), (4, 7)),
            (&TONG_RIGHT, tong_machines(), (4, 7)),
            (&MEATBALL, vec![meatball_machine(), goal_fire_machine()], (4, 3)),
            (&GRILL_LEFT, vec![grill_machine()], (4, 2)),
            (&GRILL_RIGHT, vec![grill_machine()], (4, 2)),
            (&PICKUP_FLAME, vec![pickup_machine(), collect_machine()], (4, 2)),
            (&PICKUP_KNIFE, vec![pickup_machine(), collect_machine()], (4, 2)),
            (&COURT_TILE, Vec::new(), (1, 1)),
            (&COURT_EDGE, Vec::new(), (1, 1)),
        ];

        for (spec, machines, expected_grid) in sheets {
            let prepared = prepare_sheet(&base, spec.path)
                .unwrap_or_else(|error| panic!("{} does not load: {error}", spec.path));
            assert_eq!(
                (prepared.sheet.grid.cols, prepared.sheet.grid.rows),
                expected_grid,
                "{}: the declared {}x{} cell does not cut the PNG into {expected_grid:?}",
                spec.path, spec.cell.x, spec.cell.y
            );
            assert_eq!(prepared.filter, TextureFilter::Nearest, "{}: pixel art", spec.path);

            let clips: Vec<&str> =
                prepared.sheet.clips.iter().map(|(name, _)| name.as_str()).collect();
            for machine in &machines {
                for (state, row) in machine.states() {
                    assert!(
                        clips.contains(&row.clip.as_str()),
                        "{}: state '{state}' plays '{}', which the sheet does not have ({clips:?})",
                        spec.path, row.clip
                    );
                }
            }
        }
    }

    #[test]
    fn test_every_sheet_is_measured_inside_its_own_cell() {
        // A transposed or overflowing box still yields a plausible offset, so the
        // measurement itself is checked before anything is derived from it.
        for spec in [
            &TONG_LEFT, &TONG_RIGHT, &MEATBALL, &GRILL_LEFT, &GRILL_RIGHT, &PICKUP_FLAME,
            &PICKUP_KNIFE, &COURT_TILE, &COURT_EDGE,
        ] {
            assert!(spec.is_within_cell(), "{} is measured outside its cell", spec.path);
        }
    }

    #[test]
    fn test_the_colliders_are_the_measured_art() {
        // Each collider is the reference frame's own opaque box, measured from the
        // synced PNG — never a guessed size, and never scaled.
        assert_eq!(footprint(&TONG_LEFT), Vec2::new(PADDLE_W, PADDLE_H));
        assert_eq!(footprint(&TONG_RIGHT), Vec2::new(PADDLE_W, PADDLE_H));
        assert_eq!(footprint(&MEATBALL).x, BALL_SIZE);
        assert_eq!(BALL_SIZE, BALL_RADIUS * 2.0);

        // The art lands on the collider: the tongs and the grills are centred in their
        // cells, and the meatball's body — which sits low so the fire has room above it
        // — draws the cell half a pixel right and ten and a half up.
        assert_eq!(TONG_LEFT.sprite_offset(), Vec2::ZERO);
        assert_eq!(TONG_RIGHT.sprite_offset(), Vec2::ZERO);
        assert_eq!(GRILL_LEFT.sprite_offset(), Vec2::ZERO);
        assert_eq!(MEATBALL.sprite_offset(), Vec2::new(-0.5, 10.5));
    }

    #[test]
    fn test_a_tong_and_its_goal_sensor_never_intersect() {
        // The sensor's box begins on the goal line, and both the tong's collider box and
        // its whole 64 px cell stop short of it. At the un-narrowed 370 the cell reached
        // 402 into a sensor starting at 400.
        let sensor_inner_x = GOAL_SENSOR_X - GOAL_SENSOR_W / 2.0;
        assert_eq!(sensor_inner_x, COURT_HALF_W, "the sensor's box starts at the goal line");

        let collider_edge = PADDLE_X + PADDLE_W / 2.0;
        let cell_edge = PADDLE_X + TONG_CELL.x / 2.0;
        assert!(collider_edge < sensor_inner_x, "the tong's collider at {collider_edge}");
        assert!(cell_edge < sensor_inner_x, "and the art's whole cell at {cell_edge}");
    }

    #[test]
    fn test_a_meatball_resting_on_a_tong_is_inside_the_goal_line() {
        // The point is awarded the moment a meatball's edge crosses the line, so a ball
        // the player is holding must not be there: the tong's face, plus one meatball's
        // width, plus the daylight, is as far as the tong may stand.
        let resting_edge = PADDLE_X + PADDLE_W / 2.0 + BALL_SIZE;
        assert!(resting_edge + GOAL_LINE_DAYLIGHT <= COURT_HALF_W);
    }

    #[test]
    fn test_the_derived_playfield_numbers() {
        // The numbers the section derived from the measured art, beside the geometry
        // they have to satisfy. A change to the art or to the court walks past these.
        assert_eq!(PADDLE_W, 22.0);
        assert_eq!(PADDLE_H, 78.0);
        assert_eq!(BALL_RADIUS, 15.5);
        assert_eq!(PADDLE_X, 350.0);
        assert_eq!(PADDLE_MAX_Y, 241.0);
        assert_eq!(GOAL_SENSOR_X, 410.0);
        assert_eq!(GRILL_X, 384.0);
        assert_eq!(COURT_HALF_H, 280.0);

        // The tong's art top reaches the wall's inner face; the grill's cell is flush
        // with the goal line; the court's edge is where the sensor's box begins.
        assert_eq!(PADDLE_MAX_Y + PADDLE_H / 2.0, COURT_HALF_H);
        assert_eq!(GRILL_X + GRILL_CELL.x / 2.0, COURT_HALF_W);
    }
}
