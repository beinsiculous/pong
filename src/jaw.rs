//! The tong's jaw, as the art draws it: the five poses its fourteen `_up` frames are,
//! the two arm capsules each open pose hangs from the one hinge, and the flat body the
//! closed jaw is. `gameplay::jaws` is what works these frame by frame.
//!
//! A jaw's geometry is measured, never guessed: every number comes from a per-row
//! scan of the synced PNG, alpha > 0, and the tests map each one back into the box
//! the art draws it in.

use engine_core::prelude::*;
use crate::constants::*;
use crate::types::{
    Facing, Jaw, Side, TONG_CLOSED, TONG_CLOSING, TONG_OPEN, TONG_OPENING, TONG_SCORED_ON,
};

// --- the jaw ---
// The five poses the tong's fourteen `_up` frames draw, measured from the synced PNG
// cell by cell. The numbers below are world units with the cell's centre as origin
// and Y up; every one of them
// was read as a cell pixel and converted, `x - 32` and `48 - y` for a pixel's centre.

/// Where a tong's two arms meet: the hinge both arms run into, on the cell's mirror
/// axis. Its drawn base is the cell's rows 81-86; the two arms are already one run at
/// row 81, and the base's run there is a pixel short of the arms' own union a row
/// above, so the hinge sits on the axis rather than on that run's centre. The arms
/// being mirror images of each other is what a proper jaw needs, and it is what the
/// tip measurements below are stated against.
pub(crate) const TONG_HINGE: Vec2 = Vec2::new(0.0, -33.5);

/// Half the arm's thickness at mid-length: the arms are drawn seven pixels thick
/// between the hinge and the tip pads.
pub(crate) const TONG_ARM_RADIUS: f32 = 3.5;

/// The drawn width of a tip pad: eleven pixels across, in every one of the five poses,
/// so the jaw's mouth is the whole of what opens and shuts. It is also the pad's cap
/// radius: the pad is drawn as a box with a semicircular top of this radius.
pub(crate) const TONG_PAD_HALF_WIDTH: f32 = 5.5;

/// The height every pose holds its tips at: where the arm meets its pad.
pub(crate) const TONG_TIP_Y: f32 = 27.5;

/// The pad's straight run, hinge end to tip end: the pad is drawn from row 26 up to
/// row 9 — its bottom edge at 21 here, its top at 39, level with the closed tong's
/// own top — and a capsule of the pad's half-width between these two covers exactly
/// that, its top cap the drawn dome.
pub(crate) const TONG_PAD_INNER_Y: f32 = 26.5;
pub(crate) const TONG_PAD_OUTER_Y: f32 = 33.5;

/// A pose's gripping tips, for a tong whose tips point up: half its mouth plus half a
/// pad out from the axis, both sides of it.
const fn tong_tips(mouth_gap: f32) -> Vec2 {
    Vec2::new(mouth_gap / 2.0 + TONG_PAD_HALF_WIDTH, TONG_TIP_Y)
}

/// The faces a pose's two tip pads present to each other, pad to pad: how far open the
/// jaw is, and the whole of what closes on the way to `Closed`. Every one of them is
/// narrower than the meatball, so no closing jaw can take a ball in — the bite is a
/// tip deflection. The arm capsules come a little closer still, their inner faces
/// 28 px apart when `Open`, and that is under the ball too.
pub(crate) const TONG_MOUTH_GAP: f32 = 24.0;
const TONG_MOUTH_GAP_WIDE: f32 = 18.0;
const TONG_MOUTH_GAP_NARROW: f32 = 10.0;
const TONG_MOUTH_GAP_TWITCH: f32 = 4.0;

/// Where each pose's gripping tips stand. The closed jaw's is not here: it is one flat
/// body rather than two arms, and it keeps the capsule the tong has always had.
pub(crate) const TONG_TIP_OPEN: Vec2 = tong_tips(TONG_MOUTH_GAP);
pub(crate) const TONG_TIP_WIDE: Vec2 = tong_tips(TONG_MOUTH_GAP_WIDE);
pub(crate) const TONG_TIP_NARROW: Vec2 = tong_tips(TONG_MOUTH_GAP_NARROW);
pub(crate) const TONG_TIP_TWITCH: Vec2 = tong_tips(TONG_MOUTH_GAP_TWITCH);

/// One of the five jaw poses the tong's frames draw. Its fourteen cells are these
/// five repeated; `clip_poses` says which pose a given frame draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JawPose {
    Open,
    Wide,
    Narrow,
    Twitch,
    Closed,
}

impl JawPose {
    /// Where this pose's gripping tips are, for a tong whose tips point up. `None`
    /// for the closed jaw, which is one flat body rather than two arms.
    pub(crate) fn tips(self) -> Option<Vec2> {
        match self {
            JawPose::Open => Some(TONG_TIP_OPEN),
            JawPose::Wide => Some(TONG_TIP_WIDE),
            JawPose::Narrow => Some(TONG_TIP_NARROW),
            JawPose::Twitch => Some(TONG_TIP_TWITCH),
            JawPose::Closed => None,
        }
    }
}

/// The poses a clip draws, one per frame, indexed by the animation's
/// **clip-relative** `current_frame` — a position within the clip, never a sheet cell
/// index, so a facing appended to the sheet's later rows reads the same table.
///
/// A name outside the five yields nothing, and the collider is then left as it is: a
/// clip the game has no poses for is a sheet the game has not been told about, and
/// holding the jaw's last outline beats inventing one.
pub(crate) fn clip_poses(clip: &str) -> &'static [JawPose] {
    match clip {
        TONG_OPEN => &[JawPose::Open, JawPose::Open],
        TONG_CLOSING => &[JawPose::Wide, JawPose::Narrow, JawPose::Closed],
        TONG_CLOSED => &[JawPose::Closed, JawPose::Closed],
        TONG_OPENING => &[JawPose::Narrow, JawPose::Wide, JawPose::Open],
        TONG_SCORED_ON => &[
            JawPose::Twitch,
            JawPose::Narrow,
            JawPose::Twitch,
            JawPose::Narrow,
        ],
        _ => &[],
    }
}

/// The collider a tong draws at `pose`: for the four open poses, one capsule per arm
/// from the hinge to its tip and one per pad standing on that tip — the pad is the
/// whole of what a ball meets at the jaw's end, and an arm's chord alone stops a
/// pad's dome short — and the closed tong's own flat capsule when the jaw is shut.
///
/// A capsule from the hinge to a tip is a chord across a drawn arm, not a trace of
/// it — the arms bow outward from that line — so the outline is at its loosest at
/// mid-length and exact at both ends. It is one shape for a jaw that moves every
/// frame, and it is what the F1 overlay is read against.
pub(crate) fn tong_pose_collider(pose: JawPose, facing: Facing) -> ColliderShape {
    let Some(tips) = pose.tips() else { return tong_collider() };
    let hinge = facing.oriented(TONG_HINGE);
    let mut parts = Vec::with_capacity(4);
    for mirror in [1.0, -1.0] {
        let tip = facing.oriented(Vec2::new(mirror * tips.x, tips.y));
        parts.push(ColliderShape::capsule(hinge, tip, TONG_ARM_RADIUS));
        parts.push(ColliderShape::capsule(
            facing.oriented(Vec2::new(mirror * tips.x, TONG_PAD_INNER_Y)),
            facing.oriented(Vec2::new(mirror * tips.x, TONG_PAD_OUTER_Y)),
            TONG_PAD_HALF_WIDTH,
        ));
    }
    ColliderShape::compound(parts)
}

/// The collider a jaw resting in this position wears, for a tong facing this way —
/// what a tong is born with, and what a match start lays it back to.
pub(crate) fn jaw_collider(jaw: Jaw, facing: Facing) -> ColliderShape {
    let pose = match jaw {
        Jaw::Open => JawPose::Open,
        Jaw::Closed => JawPose::Closed,
    };
    tong_pose_collider(pose, facing)
}

/// A tong's face: the edge of its collider that meets the court, which is what a ball
/// arrives at and what the CPU measures its time to.
pub(crate) fn tong_face_x(side: Side) -> f32 {
    let face = PADDLE_X - PADDLE_W / 2.0;
    match side {
        Side::Left => -face,
        Side::Right => face,
    }
}

/// How far a tong's two jaws are pushed toward the court before they move it: a stick
/// leaning inside this band leaves the intent where it was, which is what makes the
/// jaw stick rather than chatter (D8).
pub(crate) const JAW_AXIS_DEAD_ZONE: f32 = 0.5;

/// How far past a tong's own end the ball must be before the CPU turns its face to
/// it: half the tong's height, so a ball alongside the tong — where the chase wobbles
/// by a pixel or two — leaves the face where it is.
pub(crate) const CPU_FACE_TURN_DISTANCE: f32 = PADDLE_H / 2.0;

/// One full open and close of a jaw: the `opening` and `closing` clips at three frames
/// each. A CPU tong only opens when the ball's return leaves it this much and the
/// chomp's own lead to spare.
pub(crate) const TONG_JAW_CYCLE: f32 = 0.6;

/// The tong's collider at rest: the closed tong's measured box as a vertical capsule,
/// 22 wide and 78 tall. Its rounded caps give an edge hit a real angle, and its
/// footprint is the art's own.
///
/// The engine's `capsule_y` takes the **total** height and the cap radius — it derives
/// the straight segment itself — so this is the whole 78 px tong with 11 px caps, not
/// rapier's half-height form, which would build a capsule a third as tall.
pub(crate) fn tong_collider() -> ColliderShape {
    ColliderShape::capsule_y(PADDLE_H, PADDLE_W / 2.0)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{BALL_RADIUS, TONG_LEFT, TONG_RIGHT};
    use engine_core::assets::sprite_sheet::prepare_sheet;

    #[test]
    fn test_the_tong_collider_is_the_whole_measured_tong() {
        // `capsule_y` takes the total height: a 78-tall tong with 11 px caps has a
        // 28 px half-segment. Passing 28 as the height built a capsule 28 tall.
        match tong_collider() {
            ColliderShape::CapsuleY { half_height, radius } => {
                assert_eq!(radius, PADDLE_W / 2.0);
                assert_eq!(2.0 * half_height + 2.0 * radius, PADDLE_H, "the capsule spans the tong");
                assert_eq!(half_height, 28.0);
            }
            other => panic!("the tong is a vertical capsule, not {other:?}"),
        }
    }

    #[test]
    fn test_every_tong_clip_draws_the_poses_its_frames_do() {
        // The pose tables are indexed by a clip-relative frame, so a table that is a
        // frame short reads past its end and one that is long never reaches its tail:
        // each table must be exactly as long as the clip the sheet declares for it.
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        for spec in [&TONG_LEFT, &TONG_RIGHT] {
            let prepared = prepare_sheet(&base, spec.path)
                .unwrap_or_else(|error| panic!("{} does not load: {error}", spec.path));
            assert!(!prepared.sheet.clips.is_empty(), "{} declares its clips", spec.path);
            for (name, clip) in &prepared.sheet.clips {
                let (clip_name, _) = Facing::split(name)
                    .unwrap_or_else(|| panic!("{name}: a tong clip names the facing it draws"));
                assert_eq!(
                    clip_poses(clip_name).len(),
                    clip.frame_indices.len(),
                    "{}: '{name}' plays {} frames and its pose table has {}",
                    spec.path,
                    clip.frame_indices.len(),
                    clip_poses(clip_name).len()
                );
            }
        }
    }

    #[test]
    fn test_the_closing_jaw_shuts_and_the_opening_one_opens_the_same_way() {
        // A close ends shut and the held shut clip stays shut; an opening starts
        // narrow and widens. These are the art's own step order, so they are asserted
        // as written rather than derived from each other.
        assert_eq!(clip_poses(TONG_OPEN), &[JawPose::Open, JawPose::Open][..]);
        assert_eq!(clip_poses(TONG_CLOSING), &[JawPose::Wide, JawPose::Narrow, JawPose::Closed][..]);
        assert_eq!(clip_poses(TONG_CLOSED), &[JawPose::Closed, JawPose::Closed][..]);
        assert_eq!(clip_poses(TONG_OPENING), &[JawPose::Narrow, JawPose::Wide, JawPose::Open][..]);
        assert_eq!(
            clip_poses(TONG_SCORED_ON),
            &[JawPose::Twitch, JawPose::Narrow, JawPose::Twitch, JawPose::Narrow][..]
        );
        assert!(clip_poses("no_such_clip").is_empty(), "an unknown clip draws no pose");
    }

    /// The segments a collider is made of, as `(start, end, radius)`: an open jaw's
    /// are arm, pad, arm, pad.
    fn parts_of(shape: &ColliderShape) -> Vec<(Vec2, Vec2, f32)> {
        match shape {
            ColliderShape::Compound(parts) => parts
                .iter()
                .map(|part| match part {
                    ColliderShape::Capsule { a, b, radius } => (*a, *b, *radius),
                    other => panic!("a jaw's arm is a capsule, not {other:?}"),
                })
                .collect(),
            other => panic!("an open jaw is a compound of arms, not {other:?}"),
        }
    }

    #[test]
    fn test_a_pose_is_two_arms_hanging_from_one_hinge_each_wearing_a_pad() {
        for pose in [JawPose::Open, JawPose::Wide, JawPose::Narrow, JawPose::Twitch] {
            for facing in [Facing::Up, Facing::Down] {
                let parts = parts_of(&tong_pose_collider(pose, facing));
                assert_eq!(parts.len(), 4, "{pose:?} is two arms and two pads");
                let (hinge_a, tip_a, radius_a) = parts[0];
                let (pad_a_inner, pad_a_outer, pad_a_radius) = parts[1];
                let (hinge_b, tip_b, radius_b) = parts[2];
                let (pad_b_inner, pad_b_outer, pad_b_radius) = parts[3];
                assert_eq!(hinge_a, hinge_b, "{pose:?}: both arms hang from the one hinge");
                assert_eq!(radius_a, TONG_ARM_RADIUS, "{pose:?}: an arm is drawn that thick");
                assert_eq!(radius_b, radius_a);
                assert_eq!(tip_a.x, -tip_b.x, "{pose:?}: the jaw opens as far to each side");
                assert_eq!(tip_a.y, tip_b.y, "{pose:?}: and both tips are level");
                // Each pad stands on its own arm's tip, straight out along the tong.
                for (tip, inner, outer, radius) in
                    [(tip_a, pad_a_inner, pad_a_outer, pad_a_radius), (tip_b, pad_b_inner, pad_b_outer, pad_b_radius)]
                {
                    assert_eq!(inner.x, tip.x, "{pose:?}: a pad stands on its arm's tip");
                    assert_eq!(outer.x, tip.x);
                    assert_eq!(radius, TONG_PAD_HALF_WIDTH, "{pose:?}: a pad is drawn that wide");
                    assert!(
                        (outer.y - inner.y).abs() > 0.0 && (outer.y.abs() > inner.y.abs()),
                        "{pose:?}: the pad runs from its arm outward, {inner:?} to {outer:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_the_closed_jaw_keeps_the_capsule_the_tong_has_always_had() {
        // The flat returning face is the one shape that is not two arms, and it is the
        // measured tong rather than a pose of it.
        for facing in [Facing::Up, Facing::Down] {
            assert_eq!(tong_pose_collider(JawPose::Closed, facing), tong_collider());
            assert_eq!(jaw_collider(Jaw::Closed, facing), tong_collider());
            assert_eq!(jaw_collider(Jaw::Open, facing), tong_pose_collider(JawPose::Open, facing));
        }
    }

    #[test]
    fn test_the_down_jaw_is_the_up_jaw_mirrored_in_y() {
        // The art draws one body twice, mirrored, and the collider is measured once:
        // every shape a `_down` tong wears is the `_up` one with its y negated.
        fn mirrored(shape: &ColliderShape) -> ColliderShape {
            match shape {
                ColliderShape::Compound(parts) => {
                    ColliderShape::Compound(parts.iter().map(mirrored).collect())
                }
                ColliderShape::Capsule { a, b, radius } => {
                    ColliderShape::capsule(Vec2::new(a.x, -a.y), Vec2::new(b.x, -b.y), *radius)
                }
                // A vertical capsule is its own mirror: `closed` reads the same either way.
                other => other.clone(),
            }
        }
        for pose in [JawPose::Open, JawPose::Wide, JawPose::Narrow, JawPose::Twitch, JawPose::Closed] {
            let up = tong_pose_collider(pose, Facing::Up);
            assert_eq!(tong_pose_collider(pose, Facing::Down), mirrored(&up), "{pose:?}");
        }
    }

    /// A world point as the cell pixel it was measured as — the inverse of the
    /// conversion the constants above were written with. The cell's centre is pixel
    /// (31.5, 47.5) by index and the art is measured Y down.
    fn cell_pixel(point: Vec2) -> Vec2 {
        Vec2::new(point.x + 31.5, 47.5 - point.y)
    }


    #[test]
    fn test_every_jaw_endpoint_lands_inside_the_pose_the_art_draws_it_in() {
        // The measured opaque bounds of each pose in the left tong's `_up` cells, in
        // cell pixels with Y down (the per-row scan in the batch's report). Every pose
        // spans the same rows; it is its width that closes.
        let measured = [
            (JawPose::Open, 8.0, 55.0),
            (JawPose::Wide, 11.0, 52.0),
            (JawPose::Narrow, 15.0, 48.0),
            (JawPose::Twitch, 18.0, 45.0),
            (JawPose::Closed, 21.0, 42.0),
        ];
        let (top, bottom) = (9.0, 86.0);

        let hinge = cell_pixel(TONG_HINGE);
        assert!(
            (top..=bottom).contains(&hinge.y),
            "the hinge at {hinge:?} is inside the tong's rows"
        );
        for (pose, left, right) in measured {
            if let Some(tips) = pose.tips() {
                let tip = cell_pixel(tips);
                assert!(
                    (left..=right).contains(&tip.x) && (top..=bottom).contains(&tip.y),
                    "{pose:?}: the tip at {tip:?} is outside the x {left}-{right} the art draws it in"
                );
                // The pad reaches the art's very top row and its own width each side of
                // the tip, and not a pixel past either: the drawn pad is rows 9-26,
                // eleven wide, in every open pose.
                let pad_top = cell_pixel(Vec2::new(tips.x, TONG_PAD_OUTER_Y + TONG_PAD_HALF_WIDTH));
                let pad_bottom = cell_pixel(Vec2::new(tips.x, TONG_PAD_INNER_Y - TONG_PAD_HALF_WIDTH));
                assert_eq!(pad_top.y, 8.5, "{pose:?}: the pad's dome reaches row 9");
                assert_eq!(pad_bottom.y, 26.5, "{pose:?}: and its base is row 26");
                // The art's box is one pixel wider than the pad on each side: the eye's
                // highlight at row 20 pokes out. The pad's own outer edge is the box's
                // edge less that pixel.
                let pad_outer_x = tip.x + TONG_PAD_HALF_WIDTH;
                assert_eq!(pad_outer_x, right - 0.5, "{pose:?}: the pad's outer edge is the art's");
                let _ = left;
            }
            assert!(
                (left..=right).contains(&hinge.x),
                "{pose:?}: the hinge at {hinge:?} is inside the x {left}-{right} it is drawn in"
            );
        }
    }

    #[test]
    fn test_the_mouth_is_narrower_than_the_meatball() {
        // The bite is a tip deflection, never a catch: the pad capsules' inner faces
        // are the drawn mouth, and it leaves no room for a ball, so no closing jaw can
        // take one in.
        let parts = parts_of(&tong_pose_collider(JawPose::Open, Facing::Up));
        let (pad_a, _, radius) = parts[1];
        let (pad_b, _, _) = parts[3];
        let mouth = (pad_a.x - pad_b.x).abs() - 2.0 * radius;
        let ball = BALL_RADIUS * 2.0;

        assert_eq!(mouth, TONG_MOUTH_GAP, "the collider's mouth is the drawn mouth");
        assert!(mouth < ball, "the mouth leaves {mouth} for a {ball} ball");
    }
}
