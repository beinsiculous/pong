//! Tong's music: the original Countertop Circuit, which the engine's conductor plays as a song.
//!
//! The background — syncopated synth bass, lead, chord stabs and drums — plays by itself from
//! `init` on, through the menus and under the pause menu. Four moments trigger the song's
//! bindings, each on its own source: a paddle hit plays the next unit of the shared
//! `refrain` part on the metallic contact synth (the tong's own source), a goal the next unit of the same
//! part on the low goal synth (the game's source, so it plays whole whatever the ball does next),
//! and a move in the pause menu the next unit of the `jam` counter-line (the menu's). The
//! actual tong displacement starts or reverses the independent quiet `movement` line. The
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
    pub(crate) moves: u64,
}

/// See the module docs.
#[derive(Debug, Clone)]
pub(crate) struct Music {
    pub(crate) game: GameMusic,
    pub(crate) counts: MusicCounts,
    movement: [MovementCue; 2],
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
            movement: [MovementCue::default(); 2],
            #[cfg(test)]
            triggered: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct MovementCue {
    direction: i8,
    cooldown: f32,
}

impl MovementCue {
    fn observe(&mut self, displacement: f32, seconds: f32, requested_speed: f32) -> bool {
        self.cooldown = (self.cooldown - seconds).max(0.0);
        let direction = if requested_speed.abs() > MOVEMENT_SOUND_MIN_SPEED {
            if requested_speed > 0.0 { 1 } else { -1 }
        } else { 0 };
        // Physics may run no step on a high-refresh frame. Keep held motion's
        // history, and wait for actual travel before consuming a new direction.
        if direction != 0 && displacement * direction as f32 <= 0.0 {
            return false;
        }
        let changed = direction != 0 && direction != self.direction;
        // A suppressed reversal is consumed now, never replayed later on a steady hold.
        self.direction = direction;
        if changed && self.cooldown == 0.0 {
            self.cooldown = MOVEMENT_SOUND_COOLDOWN;
            true
        } else { false }
    }
}

impl Music {
    pub(crate) fn reset_movement(&mut self) {
        self.movement = [MovementCue::default(); 2];
    }

    /// Require actual travel; use the clamped target speed across physics-free frames.
    pub(crate) fn movement(&mut self, ctx: &mut GameContext, index: usize, tong: EntityId, displacement: f32, requested_speed: f32) {
        if self.movement[index].observe(displacement, ctx.delta_time, requested_speed) {
            self.counts.moves += 1;
            self.record(MOVE_BINDING);
            let source = ctx.sources.entity(tong);
            ctx.sources.trigger(ctx.audio, MOVE_BINDING, source);
        }
    }

    /// Request the song before any gesture; on the web it starts on the first click or key.
    pub(crate) fn start(&mut self, ctx: &mut GameContext) {
        self.game.start(ctx);
    }

    /// A match starts: the engine's identity rule decides by the song in the slot.
    pub(crate) fn restart(&mut self, ctx: &mut GameContext) {
        self.reset_movement();
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

    /// A goal: the refrain's next unit on the low goal synth, on the game's source.
    pub(crate) fn goal(&mut self, ctx: &mut GameContext) {
        self.counts.goals += 1;
        self.record(GOAL_BINDING);
        let source = ctx.sources.game();
        ctx.sources.trigger(ctx.audio, GOAL_BINDING, source);
    }

    /// The pause menu's frame: a move up or down jams one unit of the `jam` counter-line.
    pub(crate) fn follow_pause(&mut self, ctx: &mut GameContext, paused: bool) {
        if paused { self.reset_movement(); }
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
            format!("hits={} goals={} moves={} jam_units={jam_units} pulsed={pulsed}", counts.hits, counts.goals, counts.moves)
        });
    }

    #[cfg(test)]
    fn record(&mut self, binding: &'static str) {
        self.triggered.push(binding);
    }

    #[cfg(not(test))]
    fn record(&mut self, _binding: &'static str) {}
}


#[cfg(test)]
mod movement_tests {
    use super::MovementCue;

    #[test]
    fn slow_movement_feedback_is_consistent_across_update_rates() {
        for rate in [30.0, 60.0, 120.0, 240.0] {
            let seconds = 1.0 / rate;
            let mut moving = MovementCue::default();
            assert!(moving.observe(20.0 * seconds, seconds, 20.0), "20 px/s at {rate} Hz");
            assert!(!moving.observe(20.0 * seconds, seconds, 20.0), "steady movement stays quiet");
            let mut drifting = MovementCue::default();
            assert!(!drifting.observe(10.0 * seconds, seconds, 10.0), "10 px/s at {rate} Hz");
        }
    }
}
