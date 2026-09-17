use engine_core::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Side { Left, Right }

impl Side {
    /// The other side — the tong that conceded when this one scores.
    pub(crate) fn opposite(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

/// Which way a tong's gripping ends point. `Up` is the left tong's pictured
/// orientation — tips up, hinge down — and the art is measured from it; `Down` is the
/// mirror the right tong is drawn in. Each tong carries both, as ten clips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Facing {
    Up,
    Down,
}

impl Facing {
    /// The suffix this facing appends to a tong clip's name.
    pub(crate) fn suffix(self) -> &'static str {
        match self {
            Facing::Up => "up",
            Facing::Down => "down",
        }
    }

    /// `point` as this facing draws it. The `_down` jaw is the measured `_up` one
    /// mirrored in y, which is how the art draws it too — the two tongs are one body.
    pub(crate) fn oriented(self, point: Vec2) -> Vec2 {
        match self {
            Facing::Up => point,
            Facing::Down => Vec2::new(point.x, -point.y),
        }
    }

    /// Split a tong clip (or the machine state named after it) into its base clip and
    /// its facing, or `None` for a name that carries no facing suffix.
    pub(crate) fn split(clip: &str) -> Option<(&str, Facing)> {
        let (base, facing) = clip.rsplit_once('_')?;
        match facing {
            "up" => Some((base, Facing::Up)),
            "down" => Some((base, Facing::Down)),
            _ => None,
        }
    }
}

/// A tong clip's name for one facing — `open` + `up` is `open_up`. The ten names a
/// tong's machine states and its sheet's clips share, spelled in one place.
pub(crate) fn tong_state(clip: &str, facing: Facing) -> String {
    format!("{clip}_{}", facing.suffix())
}

/// What the player's stick or the CPU's policy asks a tong's jaw for. The player's
/// answer is held until the stick asks the other way (D8), so this is the jaw's
/// intent rather than a per-frame pulse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Jaw {
    Open,
    Closed,
}

impl Jaw {
    /// The clip a jaw resting in this position plays.
    pub(crate) fn clip(self) -> &'static str {
        match self {
            Jaw::Open => TONG_OPEN,
            Jaw::Closed => TONG_CLOSED,
        }
    }
}

/// One tong's control state: the jaw it is asked for, and the facing its machine is
/// in. Both are read and written once per frame, by `gameplay::jaws`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TongControl {
    pub(crate) jaw: Jaw,
    pub(crate) facing: Facing,
}

impl TongControl {
    pub(crate) fn new(facing: Facing) -> Self {
        Self { jaw: Jaw::Open, facing }
    }
}

/// Both tongs' controls, spawned facing the way the art draws them: the left tong's
/// tips up, the right's down.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Tongs {
    pub(crate) left: TongControl,
    pub(crate) right: TongControl,
}

impl Tongs {
    pub(crate) fn get(&self, side: Side) -> &TongControl {
        match side {
            Side::Left => &self.left,
            Side::Right => &self.right,
        }
    }

    pub(crate) fn get_mut(&mut self, side: Side) -> &mut TongControl {
        match side {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }
}

impl Default for Tongs {
    fn default() -> Self {
        Self {
            left: TongControl::new(Facing::Up),
            right: TongControl::new(Facing::Down),
        }
    }
}

// --- the clip states --------------------------------------------------------------------
// One name per subject: the states of each `ClipStateMachine` table, and the clips the
// sheets' sidecars (`assets/sprites/*.sheet.ron`) declare. The sidecar is the contract —
// a rename there is a rename here, or `transition_to` warns and the machine holds its
// ground. The tong's five clips are also its five states, one per jaw pose pair: the
// machine names a clip per facing through `tong_state`, and `jaw::clip_poses`
// turns the frame it is drawing back into the pose the collider wears.

pub(crate) const TONG_OPEN: &str = "open";
pub(crate) const TONG_CLOSING: &str = "closing";
pub(crate) const TONG_CLOSED: &str = "closed";
pub(crate) const TONG_OPENING: &str = "opening";
pub(crate) const TONG_SCORED_ON: &str = "scored_on";
pub(crate) const MEATBALL_IDLE: &str = "idle";
pub(crate) const MEATBALL_TOASTED: &str = "toasted";
pub(crate) const MEATBALL_ON_FIRE: &str = "on_fire";
pub(crate) const GRILL_IDLE: &str = "idle";
pub(crate) const GRILL_SCORE: &str = "score";
pub(crate) const PICKUP_IDLE: &str = "idle";
pub(crate) const PICKUP_COLLECT: &str = "collect";

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

    /// How much ball flight the CPU tong wants between the ball closing on its face
    /// and the chomp being due: under this many seconds it shuts its jaw, and what it
    /// reopens for is measured from it. Both leads are past the jaw's own 300 ms
    /// `closing` clip, so a chomp the CPU commits to is shut before the ball lands.
    ///
    /// `None` for Easy, which plays flat (D13): its jaw rests closed and its intent is
    /// never anything else, so it has no lead to measure against.
    pub(crate) fn ai_chomp_lead(self) -> Option<f32> {
        match self {
            Difficulty::Easy => None,
            Difficulty::Medium => Some(0.45),
            Difficulty::Hard => Some(0.6),
        }
    }

    /// Whether this difficulty's CPU tong ever opens its jaw. Easy's does not: it
    /// holds the flat, classic return, and a match start lays it shut (D13).
    pub(crate) fn opens_its_jaw(self) -> bool {
        self.ai_chomp_lead().is_some()
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
    /// The court floor: a tilemap, not a sprite. Court art is never tinted, so it is
    /// not in the theme's colour writes (there are none left).
    pub(crate) court: Option<EntityId>,
    /// The strips that draw each wall. The walls' colliders are separate entities with
    /// no sprite, so nothing needs their handles.
    pub(crate) wall_strips: Vec<EntityId>,
    /// The engine-simulated spring grid drawn over the court (D5).
    pub(crate) backdrop: Option<EntityId>,
    pub(crate) left_paddle: Option<EntityId>,
    pub(crate) right_paddle: Option<EntityId>,
    pub(crate) left_grill: Option<EntityId>,
    pub(crate) right_grill: Option<EntityId>,
    pub(crate) left_goal: Option<EntityId>,
    pub(crate) right_goal: Option<EntityId>,
}

impl Playfield {
    /// The tong that defends `side`.
    pub(crate) fn paddle(&self, side: Side) -> Option<EntityId> {
        match side {
            Side::Left => self.left_paddle,
            Side::Right => self.right_paddle,
        }
    }

    /// The grill that stands behind `side`'s tong.
    pub(crate) fn grill(&self, side: Side) -> Option<EntityId> {
        match side {
            Side::Left => self.left_grill,
            Side::Right => self.right_grill,
        }
    }
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

/// The game's art: the 1x1 white texture the particle bursts draw with, and one
/// `SpriteSheet` per subject — its texture, how it is cut into cells, and its clips.
///
/// Every sheet's path, cell and measured anchor live beside each other in
/// `constants.rs`'s sheets block; each PNG and its `.sheet.ron` sidecar is a synced
/// copy of the deion_assets master (`assets/sprites/sync.list`), so no art here is
/// hand-authored and no art is loaded from anywhere else.
pub(crate) struct Sheets {
    /// White 1x1 texture for the particle bursts.
    pub(crate) white: u32,
    pub(crate) tong_left: SpriteSheet,
    pub(crate) tong_right: SpriteSheet,
    pub(crate) meatball: SpriteSheet,
    pub(crate) grill_left: SpriteSheet,
    pub(crate) grill_right: SpriteSheet,
    pub(crate) pickup_flame: SpriteSheet,
    pub(crate) pickup_knife: SpriteSheet,
    pub(crate) court: SpriteSheet,
    pub(crate) court_edge: SpriteSheet,
}

/// A sheet with no texture, one cell and no clips. `Sheets::default` holds these until
/// `init()` loads the real ones: the engine builds the game with `Default` and calls
/// `init` on the first frame, before any entity that could draw exists.
fn placeholder_sheet() -> SpriteSheet {
    SpriteSheet {
        texture: TextureHandle { id: 0 },
        grid: SheetGrid::new(1, 1),
        clips: Vec::new(),
        path: String::new(),
    }
}

impl Default for Sheets {
    fn default() -> Self {
        Self {
            white: 0,
            tong_left: placeholder_sheet(),
            tong_right: placeholder_sheet(),
            meatball: placeholder_sheet(),
            grill_left: placeholder_sheet(),
            grill_right: placeholder_sheet(),
            pickup_flame: placeholder_sheet(),
            pickup_knife: placeholder_sheet(),
            court: placeholder_sheet(),
            court_edge: placeholder_sheet(),
        }
    }
}

pub struct PongGame {
    pub(crate) physics: PhysicsSystem,
    pub(crate) state: GameState,
    pub(crate) settings: MatchSettings,
    pub(crate) playfield: Playfield,
    pub(crate) tongs: Tongs,
    pub(crate) balls: Balls,
    pub(crate) score: Scoreboard,
    pub(crate) power_ups: PowerUpState,
    pub(crate) sheets: Sheets,
    pub(crate) frame_count: u32,

    /// Every detached effect entity — a collected pickup's puff, a scored ball's fire.
    /// They are sprite-only and end themselves (a clip's `Despawn`, or `Lifetime`), so
    /// an id in here may already be gone from the world. Cleared by the serve and by
    /// every reset, but **not** by `respawn_for_serve`: a goal's fire is meant to burn
    /// through `Serving` until the next ball is launched.
    pub(crate) transient_visuals: Vec<EntityId>,
    /// When true, every collider in the world is outlined in bright magenta
    /// lines. Toggle with F1. Useful for confirming collider geometry lines
    /// up with sprite art.
    pub(crate) debug_colliders: bool,
    /// Engine pause menu (Esc/Start toggles during a match).
    pub(crate) pause: PauseMenu,
    /// Scroll offset (px) of the achievements page — its content is taller
    /// than the window; W/S move it.
    pub(crate) achievements_scroll: f32,
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
            tongs: Tongs::default(),
            balls: Balls::default(),
            score: Scoreboard::default(),
            power_ups: PowerUpState::default(),
            sheets: Sheets::default(),
            frame_count: 0,
            transient_visuals: Vec::new(),
            debug_colliders: false,
            pause: PauseMenu::new(),
            achievements_scroll: 0.0,
        }
    }
}
