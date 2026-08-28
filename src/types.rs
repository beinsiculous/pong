use engine_core::prelude::*;
use std::collections::HashMap;


#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Side { Left, Right }

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum GameMode { SinglePlayer, TwoPlayer }

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Difficulty { Easy, Medium, Hard }

impl Difficulty {
    pub(crate) fn ai_speed(self) -> f32 {
        match self {
            Difficulty::Easy => 180.0,
            Difficulty::Medium => 255.0,
            Difficulty::Hard => 380.0,
        }
    }

    pub(crate) fn ai_dead_zone(self) -> f32 {
        match self {
            Difficulty::Easy => 15.0,
            Difficulty::Medium => 2.0,
            Difficulty::Hard => 0.5,
        }
    }

    /// Locale key for this difficulty's display name.
    pub(crate) fn label_key(self) -> &'static str {
        match self {
            Difficulty::Easy => "diff.easy",
            Difficulty::Medium => "diff.medium",
            Difficulty::Hard => "diff.hard",
        }
    }
}

/// Locale key for a chaos mode's display name (the engine's
/// `ChaosMode::label()` is fixed English; menus translate through this).
pub(crate) fn chaos_label_key(mode: ChaosMode) -> &'static str {
    match mode {
        ChaosMode::Normal => "chaos.normal",
        ChaosMode::Insane => "chaos.insane",
        ChaosMode::Ridiculous => "chaos.ridiculous",
        ChaosMode::Insiculous => "chaos.insiculous",
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GameState {
    TitleScreen { selection: u8 },
    DifficultySelect { selection: u8 },
    ChaosSelect { selection: u8 },
    Achievements,
    Serving,
    Playing,
    GameOver { left_wins: bool },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PowerUpKind { SpeedBoost, MultiBall }

impl PowerUpKind {
    /// Editor-hierarchy display name for a spawned power-up entity.
    pub(crate) fn entity_name(self) -> &'static str {
        match self {
            PowerUpKind::SpeedBoost => "Power-Up (Speed Boost)",
            PowerUpKind::MultiBall => "Power-Up (Multi-Ball)",
        }
    }
}

/// Handles to the long-lived playfield entities, spawned once in `init()`.
#[derive(Default)]
pub(crate) struct Playfield {
    pub(crate) background: Option<EntityId>,
    pub(crate) left_paddle: Option<EntityId>,
    pub(crate) right_paddle: Option<EntityId>,
    pub(crate) walls: Vec<EntityId>,
    pub(crate) left_goal: Option<EntityId>,
    pub(crate) right_goal: Option<EntityId>,
}

/// Every ball currently in play. The primary ball always exists during a
/// match; extras come from Ridiculous mode and the multi-ball power-up.
/// When the primary is scored, an extra is promoted in its place.
#[derive(Default)]
pub(crate) struct Balls {
    pub(crate) primary: Option<EntityId>,
    pub(crate) extras: Vec<EntityId>,
    /// Per-ball speed multiplier (used by Insane mode — doubles on each
    /// paddle hit). Absent entries default to 1.0.
    pub(crate) speed_mult: HashMap<EntityId, f32>,
}

impl Balls {
    /// Snapshot of every live ball, primary first.
    pub(crate) fn all(&self) -> Vec<EntityId> {
        self.primary.into_iter().chain(self.extras.iter().copied()).collect()
    }

    /// Forget a ball that is being destroyed. If it was the primary, an
    /// extra is promoted so the match keeps a primary while any ball lives.
    pub(crate) fn remove(&mut self, ball: EntityId) {
        self.speed_mult.remove(&ball);
        if self.primary == Some(ball) {
            self.primary = self.extras.pop();
        } else {
            self.extras.retain(|&b| b != ball);
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.primary.is_none() && self.extras.is_empty()
    }
}

/// Match score plus who scored/touched last (serve direction, ball tint).
pub(crate) struct Scoreboard {
    pub(crate) left: u32,
    pub(crate) right: u32,
    pub(crate) last_scorer: Side,
    pub(crate) last_touch: Option<Side>,
}

impl Scoreboard {
    pub(crate) fn award_point(&mut self, side: Side) {
        match side {
            Side::Left => self.left += 1,
            Side::Right => self.right += 1,
        }
        self.last_scorer = side;
    }

    pub(crate) fn reset(&mut self) {
        self.left = 0;
        self.right = 0;
        self.last_scorer = Side::Right;
        self.last_touch = None;
    }
}

impl Default for Scoreboard {
    fn default() -> Self {
        Self { left: 0, right: 0, last_scorer: Side::Right, last_touch: None }
    }
}

/// Live power-up entities and their timers (tracking/collection mechanics
/// live in the engine's `Pickups`/`EffectTimer`; this holds Pong's usage).
pub(crate) struct PowerUpState {
    pub(crate) active: Pickups<PowerUpKind>,
    pub(crate) speed_boost: EffectTimer,
    pub(crate) spawn_timer: f32,
}

impl Default for PowerUpState {
    fn default() -> Self {
        Self {
            active: Pickups::new(),
            speed_boost: EffectTimer::default(),
            spawn_timer: crate::constants::POWERUP_INITIAL_DELAY,
        }
    }
}

/// What the player picked in the menus before the match started.
pub(crate) struct MatchSettings {
    pub(crate) mode: GameMode,
    pub(crate) difficulty: Difficulty,
    pub(crate) chaos: ChaosMode,
}

impl Default for MatchSettings {
    fn default() -> Self {
        Self {
            mode: GameMode::SinglePlayer,
            difficulty: Difficulty::Medium,
            chaos: ChaosMode::Normal,
        }
    }
}

/// Texture ids loaded in `init()`.
#[derive(Default)]
pub(crate) struct Textures {
    /// White 1x1 texture for walls, background, and particles.
    pub(crate) white: u32,
    /// PNG texture for the rounded-capsule paddle sprite. The right paddle
    /// mirrors it horizontally via a negative `Sprite.scale.x`.
    pub(crate) paddle: u32,
    /// PNG texture for the circular ball sprite.
    pub(crate) ball: u32,
}

pub struct PongGame {
    pub(crate) physics: PhysicsSystem,
    pub(crate) state: GameState,
    pub(crate) settings: MatchSettings,
    pub(crate) playfield: Playfield,
    pub(crate) balls: Balls,
    pub(crate) score: Scoreboard,
    pub(crate) power_ups: PowerUpState,
    pub(crate) textures: Textures,
    pub(crate) frame_count: u32,

    /// Deforming spring-mass grid drawn under the gameplay sprites.
    /// Built in `init()` after we know the chaos mode (the grid color is
    /// theme-specific).
    pub(crate) grid: Option<GridMesh>,
    /// When true, every collider in the world is outlined in bright magenta
    /// lines. Toggle with F1. Useful for confirming collider geometry lines
    /// up with sprite art.
    pub(crate) debug_colliders: bool,
    /// Engine pause menu (Esc/Start toggles during a match).
    pub(crate) pause: PauseMenu,
    /// Scroll offset (px) of the achievements page — its content is taller
    /// than the window; W/S move it.
    pub(crate) achievements_scroll: f32,
    /// Beep played on every paddle hit (the H7 web-audio demo sound).
    /// `None` when the asset is missing — the game plays silent.
    pub(crate) paddle_beep: Option<SoundHandle>,
}

impl PongGame {
    /// Presentation tokens for the currently selected chaos mode.
    pub(crate) fn current_theme(&self) -> ChaosTheme {
        ChaosTheme::for_mode(self.settings.chaos)
    }
}

impl Default for PongGame {
    fn default() -> Self {
        Self {
            physics: PhysicsSystem::with_config(PhysicsConfig::top_down()),
            state: GameState::TitleScreen { selection: 0 },
            settings: MatchSettings::default(),
            playfield: Playfield::default(),
            balls: Balls::default(),
            score: Scoreboard::default(),
            power_ups: PowerUpState::default(),
            textures: Textures::default(),
            frame_count: 0,
            grid: None,
            debug_colliders: false,
            pause: PauseMenu::new(),
            achievements_scroll: 0.0,
            paddle_beep: None,
        }
    }
}
