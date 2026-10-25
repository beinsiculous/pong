//! Tong's music through the real game and the engine's harness, whose audio manager
//! plays the committed song on its virtual clock: the sidecar's shape, the three moments
//! and the sources they trigger on, the pause jam keyed to the menu's navigation, the beat
//! pulse, a match start deciding by the song in the slot, and a missing bank leaving the
//! game silent and counting every trigger it refuses.

use std::path::{Path, PathBuf};
use std::time::Duration;

use engine_core::prelude::*;
use engine_core::test_support::{DrawCommand, GameHarness, InputEvent};
use music_midi::{PartMode, PositionSnapshot, Quantize};

use crate::constants::{BALL_RADIUS, GOAL_SENSOR_W, GOAL_SENSOR_X, SONG_PATH};
use crate::types::{GameState, PongGame, Side};

const FRAME: f32 = 1.0 / 60.0;

fn repository_assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
}

fn harness_on(assets: &Path) -> GameHarness<PongGame> {
    let config = crate::game_config(assets.to_str().expect("the asset path is UTF-8"));
    let mut harness = GameHarness::new(PongGame::default(), config);
    harness.step(FRAME, &[]);
    harness
}

/// Frames with wall time between them, so the virtual clock renders blocks and the
/// conductor drains what the frames sent.
fn settle(harness: &mut GameHarness<PongGame>) {
    for _ in 0..6 {
        std::thread::sleep(Duration::from_millis(3));
        harness.step(FRAME, &[]);
    }
}

fn conductor(harness: &GameHarness<PongGame>) -> PositionSnapshot {
    harness.audio().song_snapshot().expect("a song is in the slot").conductor
}

fn key(harness: &mut GameHarness<PongGame>, code: KeyCode) {
    harness.step(FRAME, &[InputEvent::KeyPressed(code)]);
    harness.step(FRAME, &[InputEvent::KeyReleased(code)]);
}

/// A two-player match waiting for its serve.
fn in_a_match(assets: &Path) -> GameHarness<PongGame> {
    let mut harness = harness_on(assets);
    harness.context(|game, ctx| game.start_match(ctx));
    settle(&mut harness);
    harness
}

fn own_song_label(assets: &Path) -> String {
    format!("{}/{SONG_PATH}", assets.display())
}

#[test]
fn the_committed_sidecar_shares_one_unquantized_loop_part_between_hit_and_goal_and_jams_on_another() {
    let path = repository_assets().join(format!("{SONG_PATH}.ron"));
    let text = std::fs::read_to_string(&path).expect("the sidecar is committed");
    let file = music_midi::parse_song_file("ai_tong.mid.ron", &text).expect("the sidecar parses");
    let part_of = |action: &str| {
        let binding = file.bindings.iter().find(|binding| binding.action == action).expect(action);
        file.parts.iter().find(|part| part.name == binding.part).expect("the binding's part")
    };

    let refrain = part_of("hit");
    assert_eq!(part_of("goal").name, refrain.name, "a goal continues the tune the hits walk");
    assert_eq!((refrain.mode, refrain.quantize), (PartMode::Loop, Quantize::Off));
    let jam = part_of("menu");
    assert_ne!(jam.name, refrain.name);
    assert_eq!((jam.mode, jam.quantize), (PartMode::Loop, Quantize::Off));
    assert!(file.provenance.is_some(), "the composer's save requires it");
    assert_eq!(file.soundfont, "soundfonts/tong.sf2");
}

#[test]
fn the_song_plays_after_init() {
    let mut harness = harness_on(&repository_assets());
    assert!(!harness.game().music.game.song_failed());
    let before = conductor(&harness).blocks_rendered;
    settle(&mut harness);
    assert!(conductor(&harness).blocks_rendered > before, "the virtual clock renders the song");
    assert_eq!(harness.audio().song_label(), Some(own_song_label(&repository_assets()).as_str()));
}

#[test]
fn a_hit_is_one_trigger_on_its_tongs_source_and_a_goal_one_on_the_games() {
    let assets = repository_assets();
    let mut harness = in_a_match(&assets);
    let (left, right) = {
        let game = harness.game();
        (game.playfield.left_paddle.expect("left tong"), game.playfield.right_paddle.expect("right tong"))
    };
    let ball = harness.game().balls.primary.expect("a meatball");
    let accepted = conductor(&harness).accepted_triggers;
    assert_eq!(conductor(&harness).live_sources, 0);

    harness.context(|game, ctx| game.spawn_paddle_hit_visuals(ctx, &[(ball, Side::Left)]));
    settle(&mut harness);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 1);
    assert_eq!(conductor(&harness).live_sources, 1);
    let source = harness.sound_sources().source_of(left).expect("the tong's activation");

    harness.context(|game, ctx| game.spawn_paddle_hit_visuals(ctx, &[(ball, Side::Left)]));
    settle(&mut harness);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 2);
    assert_eq!(harness.sound_sources().source_of(left), Some(source), "one activation walks the cursor");
    assert_eq!(conductor(&harness).live_sources, 1, "a second hit adds no source");

    harness.context(|game, ctx| game.concede_goal(ctx, ball, Side::Left, None));
    settle(&mut harness);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 3);
    assert_eq!(conductor(&harness).live_sources, 2, "the goal plays on the game's source");
    settle(&mut harness);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 3, "and its count outlives the respawn");

    let second = harness.context(|game, ctx| game.spawn_ball(ctx.world, "Second Ball"));
    harness.context(|game, ctx| {
        game.spawn_paddle_hit_visuals(ctx, &[(ball, Side::Right), (second, Side::Right)])
    });
    settle(&mut harness);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 5, "two balls in one frame are two hits");
    assert!(harness.sound_sources().source_of(right).is_some());
    assert_eq!(harness.game().music.counts.hits, 4);
    assert_eq!(harness.game().music.counts.goals, 1);
}

/// Where the pause menu drew `label` in the last frame.
fn drawn_at(harness: &GameHarness<PongGame>, label: &str) -> Vec2 {
    harness
        .ui_commands()
        .iter()
        .find_map(|command| match command {
            DrawCommand::Text { data, .. } if data.text.contains(label) => Some(data.position + Vec2::new(4.0, data.height / 2.0)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("'{label}' was drawn"))
}

fn selected_row(harness: &GameHarness<PongGame>) -> Option<String> {
    harness.ui_commands().iter().find_map(|command| match command {
        DrawCommand::Text { data, .. } if data.text.starts_with('▶') => Some(data.text.clone()),
        _ => None,
    })
}

#[test]
fn the_pause_menu_jams_one_unit_a_move_and_nothing_on_open_confirm_or_the_pointer() {
    let mut harness = in_a_match(&repository_assets());
    let jams = |harness: &GameHarness<PongGame>| harness.game().music.game.jam_units();

    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::Escape), InputEvent::KeyPressed(KeyCode::KeyW)]);
    harness.step(FRAME, &[InputEvent::KeyReleased(KeyCode::Escape), InputEvent::KeyReleased(KeyCode::KeyW)]);
    assert!(harness.game().pause.is_active());
    assert_eq!(jams(&harness), 0, "the frame it opens jams nothing, a move on it included");

    let accepted = conductor(&harness).accepted_triggers;
    key(&mut harness, KeyCode::KeyS);
    assert_eq!(jams(&harness), 1, "one move, one unit");
    settle(&mut harness);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 1, "the song plays on under the pause menu");

    let resume = drawn_at(&harness, "Resume");
    let restart = drawn_at(&harness, "Restart");
    for at in [restart, resume, restart, resume, restart] {
        harness.step(FRAME, &[InputEvent::MouseMoved(at.x, at.y)]);
    }
    assert!(selected_row(&harness).is_some_and(|row| row.contains("Restart")), "the selection follows the pointer");
    assert_eq!(jams(&harness), 1, "a pointer swept across the rows jams nothing");

    harness.step(FRAME, &[InputEvent::MouseMoved(resume.x, resume.y)]);
    key(&mut harness, KeyCode::Enter);
    assert!(!harness.game().pause.is_active(), "confirm on Resume resumes");
    assert_eq!(jams(&harness), 1, "and confirm jams nothing");

    harness.step(FRAME, &[InputEvent::MouseMoved(restart.x, restart.y)]);
    key(&mut harness, KeyCode::Escape);
    harness.steps(5, FRAME);
    assert!(harness.game().pause.is_active());
    assert_eq!(jams(&harness), 1, "reopened under a resting pointer, nothing");
    key(&mut harness, KeyCode::Escape);
    assert!(!harness.game().pause.is_active());
    assert_eq!(jams(&harness), 1, "and cancel nothing");
}

#[test]
fn a_beat_ripples_the_grills_while_the_match_runs_and_never_under_the_pause_menu() {
    let mut harness = in_a_match(&repository_assets());
    let pulsed = |harness: &GameHarness<PongGame>| harness.game().music.game.pulsed();
    let before = pulsed(&harness);
    for _ in 0..3 {
        std::thread::sleep(Duration::from_millis(550));
        harness.step(FRAME, &[]);
    }
    assert!(pulsed(&harness) > before, "a beat every half second at 120 BPM");

    key(&mut harness, KeyCode::Escape);
    let paused_at = pulsed(&harness);
    for _ in 0..3 {
        std::thread::sleep(Duration::from_millis(550));
        harness.step(FRAME, &[]);
    }
    assert!(harness.game().pause.is_active());
    assert_eq!(pulsed(&harness), paused_at, "the frozen grid would discard the impulse");
}

/// A one-bar song beside the repository's assets, its sidecar on Tong's own bank and with
/// none of Tong's bindings: `looping` decides whether it ends.
struct OtherSong {
    directory: PathBuf,
}

impl OtherSong {
    fn new(name: &str, looping: bool) -> Self {
        let directory = std::env::temp_dir().join(format!("tong-other-song-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        std::fs::write(directory.join("other.mid"), one_bar_midi()).expect("written");
        let sidecar = format!(
            "SongFile(version: 1, soundfont: \"soundfonts/tong.sf2\", looping: {looping}, parts: [], instruments: [], bindings: [])"
        );
        std::fs::write(directory.join("other.mid.ron"), sidecar).expect("written");
        Self { directory }
    }

    fn path(&self) -> PathBuf {
        self.directory.join("other.mid")
    }
}

impl Drop for OtherSong {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).ok();
    }
}

/// Type 0, 480 ticks a quarter: one middle C held for a bar.
fn one_bar_midi() -> Vec<u8> {
    let track: &[u8] = &[
        0x00, 0x90, 60, 100, // note on
        0x8F, 0x00, 0x80, 60, 0, // 1920 ticks later, note off
        0x00, 0xFF, 0x2F, 0x00, // end of track
    ];
    let mut bytes = b"MThd".to_vec();
    bytes.extend_from_slice(&[0, 0, 0, 6, 0, 0, 0, 1, 0x01, 0xE0]);
    bytes.extend_from_slice(b"MTrk");
    bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
    bytes.extend_from_slice(track);
    bytes
}

#[test]
fn a_match_start_requests_tongs_song_into_an_empty_slot_and_takes_its_own_back_to_the_top() {
    let assets = repository_assets();
    let mut harness = harness_on(&assets);
    harness.audio_mut().stop_song();
    assert_eq!(harness.audio().song_label(), None);
    harness.context(|game, ctx| game.start_match(ctx));
    assert_eq!(harness.audio().song_label(), Some(own_song_label(&assets).as_str()));

    for _ in 0..3 {
        std::thread::sleep(Duration::from_millis(400));
        harness.step(FRAME, &[]);
    }
    assert!(harness.audio().song_position().expect("playing").beat >= 1);
    harness.context(|game, ctx| game.start_match(ctx));
    settle(&mut harness);
    let position = harness.audio().song_position().expect("playing");
    assert_eq!((position.bar, position.beat), (0, 0), "a match begins at the top of the arrangement");
}

#[test]
fn a_match_start_leaves_another_song_alone_unless_it_played_to_its_end() {
    let assets = repository_assets();
    let looping = OtherSong::new("looping", true);
    let mut harness = in_a_match(&assets);
    harness.audio_mut().play_song(looping.path()).expect("the other song loads");
    let label = looping.path().display().to_string();
    let ball = harness.game().balls.primary.expect("a meatball");
    let (accepted, refused) = (conductor(&harness).accepted_triggers, harness.sound_sources().refused_triggers());

    harness.context(|game, ctx| game.start_match(ctx));
    harness.context(|game, ctx| game.spawn_paddle_hit_visuals(ctx, &[(ball, Side::Left)]));
    settle(&mut harness);
    assert_eq!(harness.audio().song_label(), Some(label.as_str()), "the audition plays on");
    assert_eq!(conductor(&harness).accepted_triggers, accepted, "Tong's hit is not this song's binding");
    assert_eq!(harness.sound_sources().refused_triggers(), refused + 1, "refused, and counted");

    let ended = OtherSong::new("ended", false);
    harness.audio_mut().play_song(ended.path()).expect("the other song loads");
    for _ in 0..4 {
        std::thread::sleep(Duration::from_millis(700));
        harness.step(FRAME, &[]);
    }
    assert!(harness.audio().song_ended(), "a one-bar song at 120 BPM ends in two seconds");
    harness.context(|game, ctx| game.start_match(ctx));
    assert_eq!(harness.audio().song_label(), Some(own_song_label(&assets).as_str()), "an ended song is no song");
}

/// An asset folder holding every directory `init` reads but `soundfonts/`, each a link to
/// the repository's own, removed however the test ends.
#[cfg(unix)]
struct AssetsWithoutBank {
    root: PathBuf,
}

#[cfg(unix)]
impl AssetsWithoutBank {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("tong-missing-bank-{}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();
        let assets = root.join("assets");
        std::fs::create_dir_all(assets.join("ai")).expect("a scratch asset folder");
        let repository = repository_assets();
        for directory in ["sprites", "fonts", "locales", "ai/music"] {
            std::os::unix::fs::symlink(repository.join(directory), assets.join(directory)).expect("linked");
        }
        Self { root }
    }

    fn assets(&self) -> PathBuf {
        self.root.join("assets")
    }
}

#[cfg(unix)]
impl Drop for AssetsWithoutBank {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

#[cfg(unix)]
#[test]
fn without_its_bank_the_game_runs_silent_and_counts_every_trigger_it_refuses() {
    let scratch = AssetsWithoutBank::new();
    let mut harness = in_a_match(&scratch.assets());
    assert!(harness.game().music.game.song_failed(), "the song failed at init");
    assert_eq!(harness.game().music.game.failures(), 2, "and again at the match start, which asks every time");
    assert_eq!(harness.audio().song_label(), None);
    let ball = harness.game().balls.primary.expect("a meatball");
    let refused = harness.sound_sources().refused_triggers();

    harness.context(|game, ctx| {
        game.spawn_paddle_hit_visuals(ctx, &[(ball, Side::Left)]);
        game.spawn_paddle_hit_visuals(ctx, &[(ball, Side::Right)]);
    });
    settle(&mut harness);

    assert_eq!(harness.sound_sources().refused_triggers(), refused + 2, "every hit counted as refused");
    assert!(matches!(harness.game().state, GameState::Serving), "and the game runs on");
}

#[test]
fn a_reopen_on_the_frame_after_a_resume_jams_nothing() {
    let mut harness = in_a_match(&repository_assets());
    key(&mut harness, KeyCode::Escape);
    assert!(harness.game().pause.is_active());
    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::Enter)]);
    assert!(!harness.game().pause.is_active(), "Enter on Resume resumes");
    harness.step(
        FRAME,
        &[InputEvent::KeyReleased(KeyCode::Enter), InputEvent::KeyPressed(KeyCode::Escape), InputEvent::KeyPressed(KeyCode::KeyW)],
    );
    assert!(harness.game().pause.is_active(), "reopened on the very next frame");
    assert_eq!(harness.game().music.game.jam_units(), 0, "the opening frame jams nothing, a move on it included");
    harness.step(FRAME, &[InputEvent::KeyReleased(KeyCode::Escape), InputEvent::KeyReleased(KeyCode::KeyW)]);
}

#[test]
fn a_frame_with_a_paddle_hit_and_a_goal_triggers_the_hit_before_the_goal() {
    let mut harness = in_a_match(&repository_assets());
    let gap = 5.0;
    let speed = 450.0;
    let hitter = harness.game().balls.primary.expect("a meatball");
    let scorer = harness.context(|game, ctx| {
        game.state = GameState::Playing;
        let scorer = game.spawn_ball(ctx.world, "Second Ball");
        game.balls.extras.push(scorer);
        let face = crate::jaw::tong_face_x(Side::Left);
        game.physics.reset_body(hitter, Vec2::new(face + BALL_RADIUS + gap, 0.0));
        game.physics.set_velocity(hitter, Vec2::new(-speed, 0.0), 0.0);
        let sensor_inner = GOAL_SENSOR_X - GOAL_SENSOR_W / 2.0;
        game.physics.reset_body(scorer, Vec2::new(sensor_inner - BALL_RADIUS - gap, 180.0));
        game.physics.set_velocity(scorer, Vec2::new(speed, 0.0), 0.0);
        scorer
    });
    let accepted = conductor(&harness).accepted_triggers;

    let mut trace = Vec::new();
    for _ in 0..10 {
        harness.step(FRAME, &[]);
        trace = harness.game().music.triggered.clone();
        if !trace.is_empty() {
            break;
        }
    }
    assert_eq!(trace, ["hit", "goal"], "one physics step, the hit pass before the scoring pass");
    assert!(harness.world().validate_entity(&scorer).is_err(), "the scorer was torn down by its goal");
    settle(&mut harness);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 2, "both accepted");
    assert_eq!(conductor(&harness).live_sources, 2, "the tong's source and the game's");
}
