//! The tongs' jaws: what each jaw is asked for, which way the tong faces, and the
//! collider the frame it is drawing wears.
//!
//! The jaw is the player's now — a press that sticks, toward the court to close and
//! away to open (D8) — and the CPU's is a policy measured on the ball's flight (D13).
//! Either way a contact changes nothing: physics bounces the ball and the jaw goes on
//! doing what it was told.
//!
//! The collider follows the drawn frame (D11). The engine advances animations in the
//! frame tail *after* the game's update, so the pose read here is the one drawn last
//! game frame — a lag of one frame, a sixth of the shortest pose.

use engine_core::prelude::*;
use crate::jaw::*;
use crate::spawning::lay_tong_jaw;
use crate::types::*;
use super::{entity_position, entity_y, set_clip_state};

impl PongGame {
    /// Work both tongs' jaws for this frame. Runs after the tongs are steered, so the
    /// facing reads the same stick the movement did, and before the physics step, so a
    /// collider rebuilt here is in the step's contact events rather than a frame late.
    pub(crate) fn update_tongs(&mut self, ctx: &mut GameContext) {
        let (Some(left), Some(right)) =
            (self.playfield.left_paddle, self.playfield.right_paddle)
        else {
            return;
        };
        for (side, tong) in [(Side::Left, left), (Side::Right, right)] {
            self.update_tong(ctx, side, tong);
        }
    }

    fn update_tong(&mut self, ctx: &mut GameContext, side: Side, tong: EntityId) {
        let cpu = self.is_cpu_tong(side);
        let stick = if cpu { Vec2::ZERO } else { self.tong_stick(ctx, side) };
        let mut control = *self.tongs.get(side);

        let jaw = if cpu {
            cpu_jaw(side, self.ball_travel(ctx), self.settings.difficulty.ai_chomp_lead(), control.jaw)
        } else {
            human_jaw(side, stick.x, control.jaw)
        };
        let asked = if cpu {
            cpu_facing(self.ball_offset_y(ctx, tong))
        } else {
            facing_for_axis(stick.y)
        };

        drive_tong(ctx.world, tong, &mut control, jaw, asked);
        *self.tongs.get_mut(side) = control;
        apply_tong_collider(ctx.world, tong);
    }

    /// The primary ball's horizontal position and speed — all the CPU's chomp policy
    /// reads. A ball that is not there reads as travel of zero, which moves nothing.
    fn ball_travel(&self, ctx: &GameContext) -> (f32, f32) {
        let Some(ball) = self.balls.primary else { return (0.0, 0.0) };
        let x = entity_position(ctx.world, ball).map_or(0.0, |position| position.x);
        let vx = self.physics.get_body_velocity(ball).map_or(0.0, |(velocity, _)| velocity.x);
        (x, vx)
    }

    /// How far above the tong the primary ball is — what the CPU turns its face by.
    /// A ball that is not there reads as level, which turns nothing.
    fn ball_offset_y(&self, ctx: &GameContext, tong: EntityId) -> f32 {
        let Some(ball) = self.balls.primary else { return 0.0 };
        entity_y(ctx.world, ball) - entity_y(ctx.world, tong)
    }

    /// The jaw a tong rests at in this match, and the jaw `scored_on` returns it to:
    /// open, except for a CPU that never works its jaw (D13).
    pub(crate) fn rest_jaw(&self, side: Side) -> Jaw {
        if self.is_cpu_tong(side) && !self.settings.difficulty.opens_its_jaw() {
            Jaw::Closed
        } else {
            Jaw::Open
        }
    }

    /// Forget what each jaw was last asked for: from the next frame both tongs ask for
    /// the jaw their match rests them at, until a stick or the CPU's policy says
    /// otherwise.
    pub(crate) fn reset_jaw_intents(&mut self) {
        for side in [Side::Left, Side::Right] {
            let rest = self.rest_jaw(side);
            self.tongs.get_mut(side).jaw = rest;
        }
    }

    /// Lay each tong's jaw the way this match rests it, and dress the collider to
    /// match. The resting jaw is where `scored_on` returns to, so it is part of the
    /// table rather than a runtime flag: the machine is rebuilt, not nudged.
    pub(crate) fn lay_tong_jaws(&mut self, world: &mut World) {
        let (Some(left), Some(right)) =
            (self.playfield.left_paddle, self.playfield.right_paddle)
        else {
            return;
        };
        for (side, tong) in [(Side::Left, left), (Side::Right, right)] {
            let rest = self.rest_jaw(side);
            let mut control = *self.tongs.get(side);
            control.jaw = rest;
            *self.tongs.get_mut(side) = control;
            lay_tong_jaw(world, tong, control.facing, rest);
        }
    }
}

/// The player's jaw for this frame: pushing the stick toward the court closes, pulling
/// it away opens, and a stick resting inside the dead zone leaves the held intent
/// alone — so the jaw stays where it was put until the stick asks the other way (D8).
///
/// The court is to the right of the left tong and to the left of the right one, so
/// both players push at the ball to bite.
fn human_jaw(side: Side, axis: f32, held: Jaw) -> Jaw {
    let toward_court = match side {
        Side::Left => axis,
        Side::Right => -axis,
    };
    if toward_court > JAW_AXIS_DEAD_ZONE {
        Jaw::Closed
    } else if toward_court < -JAW_AXIS_DEAD_ZONE {
        Jaw::Open
    } else {
        held
    }
}

/// The facing a vertical axis asks for, or `None` inside the dead zone — where a tong
/// keeps the face it has rather than flapping on a stick that rests near zero.
fn facing_for_axis(axis: f32) -> Option<Facing> {
    if axis > JAW_AXIS_DEAD_ZONE {
        Some(Facing::Up)
    } else if axis < -JAW_AXIS_DEAD_ZONE {
        Some(Facing::Down)
    } else {
        None
    }
}

/// The facing the CPU asks for: toward a ball beyond the tong's own end, and a hold
/// for one alongside it. The chase reverses every frame or two on a ball near the
/// tong's height — that is the steering's dead zone at work, a pixel or two wide — and
/// a face that followed each reversal would flap; a ball past the tong's end is one it
/// is really travelling to meet.
fn cpu_facing(ball_offset_y: f32) -> Option<Facing> {
    if ball_offset_y > CPU_FACE_TURN_DISTANCE {
        Some(Facing::Up)
    } else if ball_offset_y < -CPU_FACE_TURN_DISTANCE {
        Some(Facing::Down)
    } else {
        None
    }
}

/// The CPU's jaw: shut while the ball closes on its face inside the difficulty's lead,
/// open while the ball is far enough away that a whole open and close fits before it
/// comes back, and shut otherwise (D13).
///
/// The ball's horizontal speed has no cap — `BALL_MAX_SPEED` bounds only its vertical
/// one — so against a fast enough ball the return is never long enough and the CPU
/// simply holds its jaw shut. A ball that still arrives mid-transition meets the pose
/// that is drawn, which is a jaw on its way closed rather than a hole.
fn cpu_jaw(side: Side, ball: (f32, f32), lead: Option<f32>, held: Jaw) -> Jaw {
    let Some(lead) = lead else { return Jaw::Closed };
    let (ball_x, ball_vx) = ball;
    // A ball going nowhere — waiting to be served, or not placeable — is no reason
    // to open and none to shut: the jaw stays where it is.
    if !ball_x.is_finite() || !ball_vx.is_finite() || ball_vx == 0.0 {
        return held;
    }
    let face = tong_face_x(side);
    let arriving = match side {
        Side::Left => ball_vx < 0.0,
        Side::Right => ball_vx > 0.0,
    };
    let time_to_face = (face - ball_x).abs() / ball_vx.abs();
    if arriving {
        if time_to_face <= lead {
            Jaw::Closed
        } else {
            Jaw::Open
        }
    } else {
        // Out and back: to the far tong's face, then the whole court to this one.
        let far_face = tong_face_x(side.opposite());
        let return_distance = (far_face - ball_x).abs() + (far_face - face).abs();
        if return_distance / ball_vx.abs() > lead + TONG_JAW_CYCLE {
            Jaw::Open
        } else {
            Jaw::Closed
        }
    }
}

/// Work one tong's machine for the frame: the jaw it is now asked for, in the facing
/// its stick now asks for.
///
/// Both move only from a resting jaw. A `closing`, `opening` or `scored_on` plays out
/// first, so a press released mid-chomp does not turn it into an `opening` halfway
/// through, and the face never turns under a clip — a transition restarts the clip it
/// lands in, and the engine's `transition_to` restarts one entered afresh.
pub(crate) fn drive_tong(
    world: &mut World,
    tong: EntityId,
    control: &mut TongControl,
    jaw: Jaw,
    asked_facing: Option<Facing>,
) {
    let Some(state) = world
        .get::<ClipStateMachine>(tong)
        .map(|machine| machine.state().to_string())
    else {
        return;
    };
    let Some((clip, _)) = Facing::split(&state) else { return };
    let resting = match clip {
        TONG_OPEN => Jaw::Open,
        TONG_CLOSED => Jaw::Closed,
        // Mid-motion, and not this frame's to interrupt.
        _ => return,
    };

    let facing = asked_facing.unwrap_or(control.facing);
    let target = match resting {
        Jaw::Open if jaw == Jaw::Closed => tong_state(TONG_CLOSING, facing),
        Jaw::Closed if jaw == Jaw::Open => tong_state(TONG_OPENING, facing),
        // The same jaw, in the facing the stick now asks for.
        resting => tong_state(resting.clip(), facing),
    };

    control.jaw = jaw;
    control.facing = facing;
    if target != state {
        set_clip_state(world, tong, &target);
    }
}

/// Dress a tong's collider in the pose of the frame it is drawing.
///
/// The write is skipped when the shape already is the wanted one: a pose a clip holds
/// over several frames — the whole of `closed`, both frames of `open` — is one
/// collider, and a rebuild is spent only where the drawn jaw actually changed.
pub(crate) fn apply_tong_collider(world: &mut World, tong: EntityId) {
    let Some((clip, frame)) = world.get::<SpriteAnimation>(tong).and_then(|animation| {
        Some((animation.current_clip.clone()?, animation.current_frame))
    }) else {
        return;
    };
    let Some((clip, facing)) = Facing::split(&clip) else { return };
    let Some(&pose) = clip_poses(clip).get(frame) else { return };
    let wanted = tong_pose_collider(pose, facing);

    if let Some(collider) = world.get_mut::<Collider>(tong) {
        if collider.shape != wanted {
            collider.shape = wanted;
        }
    }
}

#[cfg(test)]
mod tests;
