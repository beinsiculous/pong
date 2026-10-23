//! Tong's music: Bizet's Toreador refrain, which the engine's conductor plays as a song.
//!
//! The background — the trombone's chords and the timpani's march — plays by itself from
//! `init` on, through the menus and under the pause menu. Three moments trigger the song's
//! bindings, each on its own source: a paddle hit plays the next unit of the shared
//! `refrain` part on the trumpet (the tong's own source), a goal the next unit of the same
//! part on the timpani (the game's source, so it plays whole whatever the ball does next),
//! and a move in the pause menu the next unit of the `jam` counter-line (the menu's). The
//! backdrop grid ripples under each grill on the song's bars and beats.
//!
//! There is no recorded fallback: a song that will not play leaves Tong as silent as it
//! always was, every trigger refused and counted.

use std::path::Path;

use engine_core::prelude::*;

use crate::constants::*;
use crate::gameplay::ripple_grid;

/// How many frames the frame-time ring holds: five seconds at 60 fps.
const FRAME_RING_LENGTH: usize = 300;

/// The last frames' `delta_time`, for the p95 and the mean a telemetry line prints.
#[derive(Debug, Clone)]
pub(crate) struct FrameRing {
    seconds: [f32; FRAME_RING_LENGTH],
    next: usize,
    filled: usize,
}

impl Default for FrameRing {
    fn default() -> Self {
        Self {
            seconds: [0.0; FRAME_RING_LENGTH],
            next: 0,
            filled: 0,
        }
    }
}

impl FrameRing {
    pub(crate) fn push(&mut self, delta_seconds: f32) {
        self.seconds[self.next] = delta_seconds;
        self.next = (self.next + 1) % FRAME_RING_LENGTH;
        self.filled = (self.filled + 1).min(FRAME_RING_LENGTH);
    }

    /// The 95th percentile frame time in milliseconds; zero with no frame yet.
    pub(crate) fn p95_milliseconds(&self) -> f32 {
        let mut sorted = self.seconds[..self.filled].to_vec();
        if sorted.is_empty() {
            return 0.0;
        }
        sorted.sort_by(f32::total_cmp);
        let index = ((sorted.len() as f32 * 0.95).ceil() as usize).clamp(1, sorted.len()) - 1;
        sorted[index] * 1000.0
    }

    pub(crate) fn mean_milliseconds(&self) -> f32 {
        if self.filled == 0 {
            return 0.0;
        }
        self.seconds[..self.filled].iter().sum::<f32>() / self.filled as f32 * 1000.0
    }
}

/// What the game itself sent, beside the conductor's counters: what a listener's ear is
/// checked against.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MusicCounts {
    pub(crate) hits: u64,
    pub(crate) goals: u64,
    pub(crate) jam_units: u64,
    pub(crate) pulsed: u64,
}

/// See the module docs.
#[derive(Debug, Clone, Default)]
pub(crate) struct Music {
    /// The refusals already reported by the once-a-second debug line.
    refused_seen: u64,
    frame_times: FrameRing,
    pub(crate) counts: MusicCounts,
    /// The last request of Tong's song failed; every trigger since is a counted refusal.
    pub(crate) song_failed: bool,
    /// Requests that failed: the first is logged at `error`, the rest at `debug`.
    pub(crate) failures: u64,
    /// Telemetry at `info` (asked for with `?telemetry=1` or `TONG_TELEMETRY=1`), else `debug`.
    telemetry_at_info: bool,
    seconds_since_telemetry: f32,
    seconds_since_refusals: f32,
    /// Whether the pause menu was up last frame: the frame it opens jams nothing.
    was_paused: bool,
}

/// The song's path under the game's asset base: the one path `start` requests and
/// `restart` compares the slot's song against.
pub(crate) fn song_path(ctx: &GameContext) -> String {
    format!("{}/{SONG_PATH}", ctx.assets.base_path())
}

impl Music {
    /// Request the song before any gesture — on the web it waits for the first click or key
    /// and starts then — and drop whatever the boot left in the source's maxima.
    pub(crate) fn start(&mut self, ctx: &mut GameContext) {
        self.telemetry_at_info = telemetry_asked();
        self.request(ctx);
        ctx.audio.take_song_maxima();
    }

    fn request(&mut self, ctx: &mut GameContext) {
        match ctx.audio.play_song(song_path(ctx)) {
            Ok(()) => self.song_failed = false,
            Err(error) => {
                self.song_failed = true;
                self.failures += 1;
                if self.failures == 1 {
                    log::error!("Tong's song could not play: {error}");
                } else {
                    log::debug!("Tong's song could not play again ({} times): {error}", self.failures);
                }
            }
        }
    }

    /// A match starts. Decided by the song in the slot: none — a preview that ended took
    /// it, or the output died — requests Tong's again; Tong's own, compared as a whole
    /// path, goes back to its top, so the first hit plays the refrain's first unit;
    /// another song is left alone, since in the editor a song the composer saved is the
    /// session's music and a match is how it is auditioned.
    pub(crate) fn restart(&mut self, ctx: &mut GameContext) {
        self.was_paused = false;
        let own = song_path(ctx);
        let in_slot = ctx.audio.song_label().map(str::to_string);
        match in_slot {
            None => self.request(ctx),
            Some(label) if Path::new(&label) == Path::new(&own) => {
                ctx.audio.seek_song(0);
                self.print_telemetry(ctx);
            }
            Some(_) => {}
        }
    }

    /// A tong struck a ball: the refrain's next unit, on the tong's own source. Sent
    /// whatever song holds the slot — one that binds `hit` plays it, one that does not
    /// refuses it, counted.
    pub(crate) fn hit(&mut self, ctx: &mut GameContext, tong: EntityId) {
        self.counts.hits += 1;
        let source = ctx.sources.entity(tong);
        ctx.sources.trigger(ctx.audio, HIT_BINDING, source);
    }

    /// A goal: the refrain's next unit on the timpani, on the game's source.
    pub(crate) fn goal(&mut self, ctx: &mut GameContext) {
        self.counts.goals += 1;
        let source = ctx.sources.game();
        ctx.sources.trigger(ctx.audio, GOAL_BINDING, source);
    }

    /// The pause menu's frame: a move up or down — read from the input the menu itself
    /// navigates by, never from its selection, which follows a resting pointer — jams one
    /// unit. The frame the menu opens jams nothing, nor do confirm, cancel and the mouse.
    pub(crate) fn follow_pause(&mut self, ctx: &mut GameContext, paused: bool) {
        let was_paused = std::mem::replace(&mut self.was_paused, paused);
        if !(paused && was_paused) {
            return;
        }
        let input = MenuInput::read(ctx.input);
        if input.up || input.down {
            self.counts.jam_units += 1;
            let source = ctx.sources.menu();
            ctx.sources.trigger(ctx.audio, MENU_BINDING, source);
        }
    }

    /// A running gameplay frame: on a bar line the full hit ripple under each grill, on
    /// another beat a quarter of it. Never under the pause menu, whose frozen grid discards
    /// what it is given.
    pub(crate) fn pulse(&mut self, ctx: &mut GameContext, grills: [Option<Vec2>; 2]) {
        let strength = if ctx.audio.beat_crossed(Subdivision::Bar) {
            HIT_RIPPLE_STRENGTH
        } else if ctx.audio.beat_crossed(Subdivision::Beat) {
            HIT_RIPPLE_STRENGTH * BEAT_PULSE_FRACTION
        } else {
            return;
        };
        self.counts.pulsed += 1;
        for grill in grills.into_iter().flatten() {
            ripple_grid(ctx.world, grill, strength, HIT_RIPPLE_RADIUS);
        }
    }

    /// Every frame, whatever the state: the frame time into the ring, a telemetry line every
    /// five seconds, and the refusals once a second at debug.
    pub(crate) fn update(&mut self, ctx: &mut GameContext) {
        self.frame_times.push(ctx.delta_time);
        self.seconds_since_telemetry += ctx.delta_time;
        self.seconds_since_refusals += ctx.delta_time;
        if self.seconds_since_telemetry >= TELEMETRY_SECONDS {
            self.print_telemetry(ctx);
        }
        if self.seconds_since_refusals >= 1.0 {
            self.seconds_since_refusals = 0.0;
            let refused = ctx.sources.refused_triggers();
            if refused != self.refused_seen {
                log::debug!("tong: {} triggers refused in all", refused);
                self.refused_seen = refused;
            }
        }
    }

    /// One `conductor=on` line, its window's maxima taken, with the game's own counts.
    fn print_telemetry(&mut self, ctx: &mut GameContext) {
        self.seconds_since_telemetry = 0.0;
        let Some(mut snapshot) = ctx.audio.song_snapshot() else {
            log::debug!("tong: no song in the slot");
            return;
        };
        let maxima = ctx.audio.take_song_maxima();
        snapshot.source.render_nanoseconds_longest = maxima.render_nanoseconds_longest;
        snapshot.source.drain_gap_nanoseconds_longest = maxima.drain_gap_nanoseconds_longest;
        let line = format!(
            "{} frame_p95={:.2}ms frame_mean={:.2}ms hits={} goals={} jam_units={} pulsed={} refused={}",
            snapshot.log_line(),
            self.frame_times.p95_milliseconds(),
            self.frame_times.mean_milliseconds(),
            self.counts.hits,
            self.counts.goals,
            self.counts.jam_units,
            self.counts.pulsed,
            ctx.sources.refused_triggers(),
        );
        if self.telemetry_at_info {
            log::info!("{line}");
        } else {
            log::debug!("{line}");
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn telemetry_asked() -> bool {
    engine_core::web::query_param("telemetry").as_deref() == Some("1")
}

#[cfg(not(target_arch = "wasm32"))]
fn telemetry_asked() -> bool {
    std::env::var("TONG_TELEMETRY").as_deref() == Ok("1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ring_reports_its_ninety_fifth_percentile_and_mean_over_its_last_frames_only() {
        let mut ring = FrameRing::default();
        assert_eq!(ring.p95_milliseconds(), 0.0);
        for _ in 0..FRAME_RING_LENGTH {
            ring.push(1.0);
        }
        for frame in 0..FRAME_RING_LENGTH {
            ring.push(if frame >= FRAME_RING_LENGTH - 15 { 0.050 } else { 0.010 });
        }
        assert!((ring.p95_milliseconds() - 10.0).abs() < 1e-3, "{}", ring.p95_milliseconds());
        ring.push(0.050);
        assert!((ring.p95_milliseconds() - 50.0).abs() < 1e-3, "sixteen slow frames of 300 reach the p95");
        assert!(ring.mean_milliseconds() < 20.0, "the second-long frames have left the ring");
    }
}
