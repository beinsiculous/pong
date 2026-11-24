//! Exercise real fixed-step physics between faster rendered frames.
use super::*;

#[test]
fn real_physics_movement_stays_quiet_between_steps_at_high_refresh_rates() {
    for rate in [75, 90, 120, 144, 165, 240] {
        let seconds = 1.0 / rate as f32;
        let mut harness = in_a_match(&repository_assets());
        harness.context(|game, _| game.settings.mode = crate::types::GameMode::TwoPlayer);
        harness.step(seconds, &[InputEvent::KeyPressed(KeyCode::KeyW)]);
        let tong = harness.game().playfield.left_paddle.unwrap();
        let mut zero_displacement_frames = 0;
        for _ in 0..rate / 2 {
            let before = harness.world().get::<Transform2D>(tong).unwrap().position.y;
            harness.step(seconds, &[]);
            let after = harness.world().get::<Transform2D>(tong).unwrap().position.y;
            if before == after { zero_displacement_frames += 1; }
        }
        assert!(zero_displacement_frames > 0, "exercise physics-free frames");
        assert_eq!(harness.game().music.counts.moves, 1, "steady hold at {rate} Hz");
        harness.step(seconds, &[InputEvent::KeyReleased(KeyCode::KeyW), InputEvent::KeyPressed(KeyCode::KeyS)]);
        harness.steps(rate / 4, seconds);
        assert_eq!(harness.game().music.counts.moves, 2, "reversal at {rate} Hz");
        harness.steps(rate * 4, seconds);
        assert_eq!(harness.game().music.counts.moves, 2, "wall at {rate} Hz");
        let position = harness.world().get::<Transform2D>(tong).unwrap().position.y;
        assert!((position + crate::constants::PADDLE_MAX_Y).abs() < 0.01);
    }
}
