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
//! The rules every game's song shares — the request, the match start's identity rule, the
//! jam's latch, the beat, the telemetry — are the engine's `GameMusic`; what is here is
//! Tong's: its moments, its counts and where its pulse lands.

use engine_core::prelude::*;

use crate::constants::*;
use crate::gameplay::ripple_grid;

/// What the game itself sent, beside the conductor's counters: what a listener's ear is
/// checked against. The jam and the pulse are counted by `GameMusic`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MusicCounts {
    pub(crate) hits: u64,
    pub(crate) goals: u64,
}

/// See the module docs.
#[derive(Debug, Clone)]
pub(crate) struct Music {
    pub(crate) game: GameMusic,
    pub(crate) counts: MusicCounts,
    /// The bindings this frame triggered, in the order the game sent them: production's
    /// call order recorded for the same-frame order tests, never reconstructed.
    #[cfg(test)]
    pub(crate) triggered: Vec<&'static str>,
}

impl Default for Music {
    fn default() -> Self {
        Self {
            game: GameMusic::new(SONG_PATH, "tong", "TONG_TELEMETRY"),
            counts: MusicCounts::default(),
            #[cfg(test)]
            triggered: Vec::new(),
        }
    }
}

impl Music {
    /// Request the song before any gesture; on the web it starts on the first click or key.
    pub(crate) fn start(&mut self, ctx: &mut GameContext) {
        self.game.start(ctx);
    }

    /// A match starts: the engine's identity rule decides by the song in the slot.
    pub(crate) fn restart(&mut self, ctx: &mut GameContext) {
        self.game.restart(ctx);
    }

    /// A tong struck a ball: the refrain's next unit, on the tong's own source. Sent
    /// whatever song holds the slot — one that binds `hit` plays it, one that does not
    /// refuses it, counted.
    pub(crate) fn hit(&mut self, ctx: &mut GameContext, tong: EntityId) {
        self.counts.hits += 1;
        self.record(HIT_BINDING);
        let source = ctx.sources.entity(tong);
        ctx.sources.trigger(ctx.audio, HIT_BINDING, source);
    }

    /// A goal: the refrain's next unit on the timpani, on the game's source.
    pub(crate) fn goal(&mut self, ctx: &mut GameContext) {
        self.counts.goals += 1;
        self.record(GOAL_BINDING);
        let source = ctx.sources.game();
        ctx.sources.trigger(ctx.audio, GOAL_BINDING, source);
    }

    /// The pause menu's frame: a move up or down jams one unit of the `jam` counter-line.
    pub(crate) fn follow_pause(&mut self, ctx: &mut GameContext, paused: bool) {
        if self.game.follow_pause(ctx, paused, MENU_BINDING) {
            self.record(MENU_BINDING);
        }
    }

    /// A running gameplay frame: on a bar line the full hit ripple under each grill, on
    /// another beat a quarter of it. Never under the pause menu, whose frozen grid discards
    /// what it is given.
    pub(crate) fn pulse(&mut self, ctx: &mut GameContext, grills: [Option<Vec2>; 2]) {
        let strength = match self.game.beat_pulse(ctx) {
            Some(BeatPulse::Bar) => HIT_RIPPLE_STRENGTH,
            Some(BeatPulse::Beat) => HIT_RIPPLE_STRENGTH * BEAT_PULSE_FRACTION,
            None => return,
        };
        for grill in grills.into_iter().flatten() {
            ripple_grid(ctx.world, grill, strength, HIT_RIPPLE_RADIUS);
        }
    }

    /// Every frame, whatever the state: the engine's frame ring, telemetry and refusal
    /// lines, the telemetry ending in Tong's counts.
    pub(crate) fn update(&mut self, ctx: &mut GameContext) {
        #[cfg(test)]
        self.triggered.clear();
        let (counts, jam_units, pulsed) = (self.counts, self.game.jam_units(), self.game.pulsed());
        self.game.update(ctx, || {
            format!("hits={} goals={} jam_units={jam_units} pulsed={pulsed}", counts.hits, counts.goals)
        });
    }

    #[cfg(test)]
    fn record(&mut self, binding: &'static str) {
        self.triggered.push(binding);
    }

    #[cfg(not(test))]
    fn record(&mut self, _binding: &'static str) {}
}
