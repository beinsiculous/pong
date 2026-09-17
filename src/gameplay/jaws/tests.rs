//! The jaws under test: the intent, the machine, the collider following the frame,
//! and a jaw morphing under a ball it is touching, in real physics.

use super::*;
use crate::constants::{BALL_INITIAL_SPEED, BALL_RADIUS, PADDLE_W, PADDLE_X, TONG_LEFT, TONG_RIGHT};
use crate::jaw::tong_collider;
use crate::spawning::spawn_paddle;

/// The tong sheet's clips without a texture: the real names and the real frame
/// counts, at ten frames a second — two frames for each resting clip, three for
/// the two one-shots, four for `scored_on`. One step is therefore one 100 ms frame.
fn tong_sheet() -> SpriteSheet {
    let mut sheet = SpriteSheet {
        texture: TextureHandle { id: 1 },
        grid: SheetGrid::new(4, 7),
        clips: Vec::new(),
        path: String::new(),
    };
    for facing in [Facing::Up, Facing::Down] {
        let mut clip = |name: String, frames: usize, looping: bool| {
            let indices = (0..frames as u32).collect::<Vec<u32>>();
            let clip = AnimationClip::new(indices, 10.0).with_looping(looping);
            sheet.clips.push((name, clip));
        };
        clip(tong_state(TONG_OPEN, facing), 2, true);
        clip(tong_state(TONG_CLOSED, facing), 2, true);
        clip(tong_state(TONG_CLOSING, facing), 3, false);
        clip(tong_state(TONG_OPENING, facing), 3, false);
        clip(tong_state(TONG_SCORED_ON, facing), 4, false);
    }
    sheet
}

/// A tong spawned the way the game spawns one, resting at `rest`.
fn tong(world: &mut World, facing: Facing, rest: Jaw) -> EntityId {
    let sheet = tong_sheet();
    let tong = spawn_paddle(world, "Tong", PADDLE_X, &TONG_RIGHT, &sheet, facing);
    lay_tong_jaw(world, tong, facing, rest);
    tong
}

/// One engine frame's tail: advance the animations, then let the clip machine look
/// at them — the order the engine runs them in, after the game's own update.
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
    world.get::<ClipStateMachine>(entity).expect("a tong has a machine").state().to_string()
}

/// The clip and clip-relative frame a tong is drawing — what the outline has to
/// follow.
fn drawn(world: &World, entity: EntityId) -> (String, usize) {
    let animation = world.get::<SpriteAnimation>(entity).expect("a tong animates");
    (animation.current_clip.clone().expect("a tong draws a clip"), animation.current_frame)
}

#[test]
fn test_pushing_at_the_ball_shuts_a_tong_and_pulling_away_opens_it() {
    // Toward the court is +x for the left tong and -x for the right one, so both
    // players push at the ball to bite.
    assert_eq!(human_jaw(Side::Left, 1.0, Jaw::Open), Jaw::Closed);
    assert_eq!(human_jaw(Side::Right, -1.0, Jaw::Open), Jaw::Closed);
    assert_eq!(human_jaw(Side::Left, -1.0, Jaw::Closed), Jaw::Open);
    assert_eq!(human_jaw(Side::Right, 1.0, Jaw::Closed), Jaw::Open);

    // Inside the dead zone the jaw stays where it was put, whichever that is, so
    // a stick that rests or is released mid-return changes nothing.
    for held in [Jaw::Open, Jaw::Closed] {
        assert_eq!(human_jaw(Side::Left, 0.0, held), held);
        assert_eq!(human_jaw(Side::Left, JAW_AXIS_DEAD_ZONE, held), held);
        assert_eq!(human_jaw(Side::Right, -JAW_AXIS_DEAD_ZONE, held), held);
    }
}

#[test]
fn test_the_facing_follows_the_axis_and_turns_only_at_rest() {
    // Past the dead zone the stick's sign picks the face; inside it the face holds
    // rather than flapping on a stick that rests near zero.
    assert_eq!(facing_for_axis(1.0), Some(Facing::Up));
    assert_eq!(facing_for_axis(-1.0), Some(Facing::Down));
    assert_eq!(facing_for_axis(JAW_AXIS_DEAD_ZONE), None, "the band is a hold, not a pick");
    assert_eq!(facing_for_axis(0.0), None);

    // The CPU turns toward a ball beyond its own end and holds for one alongside —
    // the steering may reverse every frame on that ball, the face does not.
    assert_eq!(cpu_facing(CPU_FACE_TURN_DISTANCE + 1.0), Some(Facing::Up));
    assert_eq!(cpu_facing(-CPU_FACE_TURN_DISTANCE - 1.0), Some(Facing::Down));
    assert_eq!(cpu_facing(CPU_FACE_TURN_DISTANCE), None, "level with the tong's end holds");
    assert_eq!(cpu_facing(Difficulty::Hard.ai_dead_zone() * 2.0), None, "a chase wobble holds");
    assert_eq!(cpu_facing(0.0), None);

    let mut world = World::new();
    let tong = tong(&mut world, Facing::Up, Jaw::Open);
    let mut control = TongControl::new(Facing::Up);
    step(&mut world, 1);

    // From `open`, a downward stick turns the face: the same clip, the other way
    // up, and the tong stays open through it.
    drive_tong(&mut world, tong, &mut control, Jaw::Open, Some(Facing::Down));
    assert_eq!(control.facing, Facing::Down);
    assert_eq!(state_of(&world, tong), tong_state(TONG_OPEN, Facing::Down));

    // Mid-close the face waits: a clip is not restarted under the tong.
    drive_tong(&mut world, tong, &mut control, Jaw::Closed, Some(Facing::Down));
    step(&mut world, 1);
    assert_eq!(state_of(&world, tong), tong_state(TONG_CLOSING, Facing::Down));
    drive_tong(&mut world, tong, &mut control, Jaw::Closed, Some(Facing::Up));
    assert_eq!(
        state_of(&world, tong),
        tong_state(TONG_CLOSING, Facing::Down),
        "the face waits out the clip it is in"
    );
    assert_eq!(control.facing, Facing::Down, "and the stick's wish does not move it");

    // Once the jaw has come to rest the face turns, on the spot.
    step(&mut world, 3);
    assert_eq!(state_of(&world, tong), tong_state(TONG_CLOSED, Facing::Down));
    drive_tong(&mut world, tong, &mut control, Jaw::Closed, Some(Facing::Up));
    assert_eq!(state_of(&world, tong), tong_state(TONG_CLOSED, Facing::Up));
    assert_eq!(control.facing, Facing::Up);
}

#[test]
fn test_the_cpu_shuts_its_jaw_by_its_lead_and_opens_only_when_the_return_allows_it() {
    let medium = Difficulty::Medium.ai_chomp_lead();
    let hard = Difficulty::Hard.ai_chomp_lead();
    assert_eq!(Difficulty::Easy.ai_chomp_lead(), None, "Easy never works its jaw at all");

    // The right tong's face is at 339, so a ball at 239 doing 250 px/s is 0.4 s
    // away and one at 214 is 0.5 s away.
    let approaching = |x: f32| (x, BALL_INITIAL_SPEED);
    assert_eq!(cpu_jaw(Side::Right, approaching(239.0), medium, Jaw::Open), Jaw::Closed, "0.4 s out");
    assert_eq!(cpu_jaw(Side::Right, approaching(214.0), medium, Jaw::Open), Jaw::Open, "0.5 s out");
    assert_eq!(cpu_jaw(Side::Right, approaching(214.0), hard, Jaw::Open), Jaw::Closed, "and Hard's lead");

    // Leaving: 1017 px of court to cross and come back at 250 px/s is four seconds,
    // so the jaw opens; at Insane's top speeds it is a quarter of one, and the CPU
    // holds its jaw shut rather than open into a ball it cannot shut on.
    assert_eq!(cpu_jaw(Side::Right, (0.0, -BALL_INITIAL_SPEED), medium, Jaw::Closed), Jaw::Open);
    assert_eq!(cpu_jaw(Side::Right, (0.0, -4000.0), medium, Jaw::Open), Jaw::Closed);

    // A ball going nowhere — waiting at centre for the serve — or one the engine
    // cannot place is not a reason to open, and not a reason to shut.
    for held in [Jaw::Open, Jaw::Closed] {
        assert_eq!(cpu_jaw(Side::Right, (0.0, 0.0), medium, held), held);
        assert_eq!(cpu_jaw(Side::Right, (0.0, f32::NAN), medium, held), held);
        assert_eq!(cpu_jaw(Side::Right, (f32::NAN, BALL_INITIAL_SPEED), medium, held), held);
    }
}

#[test]
fn test_a_press_shuts_the_jaw_once_and_a_returned_ball_changes_nothing() {
    let mut world = World::new();
    let mut game = PongGame::default();
    let sheet = tong_sheet();
    let tong = spawn_paddle(&mut world, "Left Tong", -PADDLE_X, &TONG_LEFT, &sheet, Facing::Up);
    let ball = world.spawn().id();
    game.playfield.left_paddle = Some(tong);
    game.playfield.right_paddle = Some(world.spawn().id());
    game.playfield.left_goal = Some(world.spawn().id());
    game.playfield.right_goal = Some(world.spawn().id());
    game.balls.primary = Some(ball);
    let contact = |started: bool, stopped: bool| CollisionData {
        event: CollisionEvent { entity_a: ball, entity_b: tong, started, stopped },
        contacts: Vec::new(),
    };
    let open = tong_state(TONG_OPEN, Facing::Up);
    let closing = tong_state(TONG_CLOSING, Facing::Up);
    let closed = tong_state(TONG_CLOSED, Facing::Up);

    let mut control = TongControl::new(Facing::Up);
    step(&mut world, 1);
    assert_eq!(state_of(&world, tong), open);

    // The player pushes at the ball and keeps pushing. Two balls arrive 100 ms
    // apart, the second on a jaw already closing, and neither hit moves the jaw
    // off its clip.
    let mut played = Vec::new();
    for frame in 0..7 {
        let events = match frame {
            1 => vec![contact(true, false)],
            2 => vec![contact(true, false)],
            _ => Vec::new(),
        };
        let found = game.handle_paddle_hits_and_collect_goals(&events, &[ball]);
        if frame == 1 {
            assert_eq!(found.paddle_hits, vec![(ball, Side::Left)], "a return is a hit");
        }
        if frame == 2 {
            assert_eq!(found.paddle_hits, vec![(ball, Side::Left)], "and so is the second");
        }
        drive_tong(&mut world, tong, &mut control, Jaw::Closed, None);
        step(&mut world, 1);
        played.push(state_of(&world, tong));
    }

    assert_eq!(
        played,
        vec![
            closing.clone(),
            closing.clone(),
            closing.clone(),
            closed.clone(),
            closed.clone(),
            closed.clone(),
            closed,
        ],
        "one press, one pass through `closing`, and `closed` holds: nothing restarted it"
    );
}

#[test]
fn test_the_collider_wears_the_pose_of_the_frame_that_is_drawn() {
    let mut world = World::new();
    let tong = tong(&mut world, Facing::Up, Jaw::Open);
    let mut physics = PhysicsSystem::with_config(PhysicsConfig::top_down());
    let mut control = TongControl::new(Facing::Up);

    // The jaw's whole travel, shut and open again, one game frame at a time.
    let mut jaws = vec![Jaw::Closed; 6];
    jaws.extend(vec![Jaw::Open; 6]);

    let mut rebuilds = 0;
    let mut pose_changes = 0;
    let mut drawn_pose = Some(JawPose::Open);
    let mut shut_frames = 0;
    // A tong is born wearing its resting pose; the frame tail is what selects the
    // clip it draws from then on.
    step(&mut world, 1);
    for jaw in jaws {
        drive_tong(&mut world, tong, &mut control, jaw, None);
        apply_tong_collider(&mut world, tong);
        physics.update(&mut world, 0.1);
        rebuilds += physics.external_edits_pushed_last_update();

        let (clip, frame) = drawn(&world, tong);
        let (clip_name, facing) = Facing::split(&clip).expect("a tong clip names its facing");
        let pose = clip_poses(clip_name)[frame];
        let shape = world.get::<Collider>(tong).expect("a tong has a collider").shape.clone();
        assert_eq!(shape, tong_pose_collider(pose, facing), "the outline follows the frame");

        if pose == JawPose::Closed {
            shut_frames += 1;
            assert_eq!(shape, tong_collider(), "a shut jaw is the tong's own capsule");
        }
        if Some(pose) != drawn_pose {
            pose_changes += 1;
            drawn_pose = Some(pose);
        }
        step(&mut world, 1);
    }

    assert!(shut_frames > 0, "the travel really does pass through the shut jaw");
    assert_eq!(
        rebuilds, pose_changes,
        "a collider is rebuilt where the drawn jaw changed, and nowhere else"
    );
}

/// A meatball resting against the right tong, its centre at `resting_x`, the tong
/// starting in `shapes[0]` and driven through the rest one physics step each. The
/// ball must end court-side of the tong's face, and no faster than twice a serve.
fn morph_under_a_resting_meatball(resting_x: f32, shapes: &[(JawPose, Facing)]) {
    let bound = 2.0 * BALL_INITIAL_SPEED;
    let court_side = tong_face_x(Side::Right);

    let mut world = World::new();
    let mut physics = PhysicsSystem::with_config(PhysicsConfig::top_down());
    let sheet = tong_sheet();
    let tong = spawn_paddle(&mut world, "Right Tong", PADDLE_X, &TONG_RIGHT, &sheet, Facing::Up);
    let ball = world.spawn()
        .with(Transform2D::new(Vec2::new(resting_x, TONG_TIP_Y)))
        .with(RigidBody::new_dynamic()
            .with_gravity_scale(0.0)
            .with_rotation_locked(true)
            .with_linear_damping(0.0)
            .with_angular_damping(0.0)
            .with_ccd(true))
        .with(Collider::circle_collider(BALL_RADIUS).with_friction(0.0).with_restitution(1.0))
        .id();

    let mut touching = false;
    for &(pose, facing) in shapes {
        if let Some(collider) = world.get_mut::<Collider>(tong) {
            collider.shape = tong_pose_collider(pose, facing);
        }
        physics.update(&mut world, 1.0 / 60.0);
        for collision in physics.take_collision_events() {
            if collision.event.involves(ball, tong) {
                touching = true;
            }
        }
    }
    assert!(touching, "the meatball really was against the jaw while it moved");

    let position = world.get::<Transform2D>(ball).expect("the ball has a transform").position;
    let (velocity, _) = physics.get_body_velocity(ball).expect("the ball has a body");
    assert!(
        position.x < court_side,
        "the meatball stayed on the court side of the tong's face, at {}",
        position.x
    );
    assert!(
        velocity.length() <= bound,
        "and left no faster than twice a serve, at {}",
        velocity.length()
    );
}

#[test]
fn test_a_close_under_a_resting_meatball_leaves_it_on_the_court_and_unflung() {
    // The one moment the mechanic lives on: the shape under a ball that is already
    // touching it changes. Against the court-side pad's outer face, less the
    // ball's radius: the meatball's centre when it just touches an open jaw.
    morph_under_a_resting_meatball(
        PADDLE_X - TONG_TIP_OPEN.x - TONG_PAD_HALF_WIDTH - BALL_RADIUS,
        &[
            (JawPose::Open, Facing::Up),
            (JawPose::Wide, Facing::Up),
            (JawPose::Narrow, Facing::Up),
            (JawPose::Closed, Facing::Up),
            (JawPose::Narrow, Facing::Up),
            (JawPose::Wide, Facing::Up),
            (JawPose::Open, Facing::Up),
        ],
    );
}

#[test]
fn test_a_face_turning_under_a_resting_meatball_leaves_it_on_the_court_and_unflung() {
    // The largest jump a jaw makes in one frame is not a pose but the turn: the whole
    // open shape mirrored in y, the pad the ball rests on leaving for the other end.
    morph_under_a_resting_meatball(
        PADDLE_X - TONG_TIP_OPEN.x - TONG_PAD_HALF_WIDTH - BALL_RADIUS,
        &[(JawPose::Open, Facing::Up), (JawPose::Open, Facing::Down), (JawPose::Open, Facing::Down)],
    );
}

#[test]
fn test_an_opening_jaw_sweeping_into_a_resting_meatball_pushes_it_off_unflung() {
    // The other direction: a ball grazing the shut face while the arms open into
    // it, the jaw's reach growing under the ball by up to ten pixels. Rapier
    // resolves that as an overlap, and the ball must still leave slowly.
    morph_under_a_resting_meatball(
        PADDLE_X - PADDLE_W / 2.0 - BALL_RADIUS,
        &[
            (JawPose::Closed, Facing::Up),
            (JawPose::Narrow, Facing::Up),
            (JawPose::Wide, Facing::Up),
            (JawPose::Open, Facing::Up),
            (JawPose::Open, Facing::Up),
        ],
    );
}
