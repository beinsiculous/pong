//! Menu screens: navigation and selection. Match lifecycle (starting the
//! game, theming) lives in `gameplay::flow`.

use engine_core::prelude::*;
use crate::types::*;

/// The title menu's rows, in order. Both halves of the menu derive from
/// `TITLE_ITEMS` (row count, navigation bound, confirm dispatch, drawn
/// labels), so adding/removing a row is a one-list change. The web build
/// drops Achievements — the site's game page shows the board instead.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TitleItem {
    SinglePlayer,
    TwoPlayer,
    Achievements,
    Language,
    Exit,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) const TITLE_ITEMS: &[TitleItem] = &[
    TitleItem::SinglePlayer,
    TitleItem::TwoPlayer,
    TitleItem::Achievements,
    TitleItem::Language,
    TitleItem::Exit,
];
#[cfg(target_arch = "wasm32")]
pub(crate) const TITLE_ITEMS: &[TitleItem] = &[
    TitleItem::SinglePlayer,
    TitleItem::TwoPlayer,
    TitleItem::Language,
    TitleItem::Exit,
];

/// Row index of `item` in this build's title menu (0 if absent).
pub(crate) fn title_index(item: TitleItem) -> u8 {
    TITLE_ITEMS.iter().position(|i| *i == item).unwrap_or(0) as u8
}

/// Panel layouts shared by the input half (mouse hit-testing here) and the
/// drawing half (`ui.rs`) — the geometry must match or clicks land beside
/// the drawn rows. Titles only affect the label, never the layout.
pub(crate) fn title_panel(title: &str, window_size: Vec2) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, 360.0, TITLE_ITEMS.len())
}
pub(crate) fn achievements_panel(title: &str, window_size: Vec2) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, window_size.x - 120.0, 15)
}
pub(crate) fn difficulty_panel(title: &str, window_size: Vec2) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, 360.0, 3)
}
pub(crate) fn chaos_panel(title: &str, window_size: Vec2) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, 400.0, 4)
}

impl PongGame {
    pub(crate) fn update_title_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let mouse = title_panel("", ctx.window_size).mouse_select(ctx.input);
        let selection = mouse.hovered.unwrap_or(selection);
        // An out-of-range stored selection (e.g. the shorter wasm menu)
        // clamps instead of panicking at the dispatch index below.
        let selection = selection.min(TITLE_ITEMS.len() as u8 - 1);
        let mut selection = input.navigate(selection, TITLE_ITEMS.len() as u8);
        if let Some(row) = mouse.clicked {
            selection = row;
        }
        self.state = GameState::TitleScreen { selection };

        if input.confirm || mouse.clicked.is_some() {
            match TITLE_ITEMS[selection as usize] {
                TitleItem::SinglePlayer => {
                    self.state = GameState::DifficultySelect { selection: 1 };
                }
                TitleItem::TwoPlayer => {
                    self.settings.mode = GameMode::TwoPlayer;
                    self.state = GameState::ChaosSelect { selection: 0 };
                }
                TitleItem::Achievements => {
                    self.achievements_scroll = 0.0;
                    self.state = GameState::Achievements;
                }
                TitleItem::Language => {
                    // Language: cycle locale, then re-register achievements
                    // so their names/descriptions pick up the new language
                    // (id-keyed insert — unlock state is untouched).
                    ctx.strings.cycle_locale();
                    crate::achievements::register_all(ctx.achievements, ctx.strings);
                }
                TitleItem::Exit => ctx.request_exit(),
            }
        }
    }

    pub(crate) fn update_achievements_input(&mut self, ctx: &mut GameContext) {
        let input = MenuInput::read(ctx.input);
        // The page is taller than the window: W/S (or a pad) scroll it.
        const SCROLL_STEP: f32 = 36.0;
        if input.down {
            self.achievements_scroll += SCROLL_STEP;
        }
        if input.up {
            self.achievements_scroll -= SCROLL_STEP;
        }
        self.achievements_scroll = self
            .achievements_scroll
            .clamp(0.0, crate::ui::achievements_max_scroll(ctx.window_size));
        // Whole-window dismiss: clicks on headers/margins count too, not
        // just the row bands (the page is one big info sheet).
        let click_dismiss = achievements_panel("", ctx.window_size).clicked_inside(ctx.input);
        if input.back || input.confirm || click_dismiss {
            self.state = GameState::TitleScreen {
                selection: title_index(TitleItem::Achievements),
            };
        }
    }

    pub(crate) fn update_difficulty_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let mouse = difficulty_panel("", ctx.window_size).mouse_select(ctx.input);
        let selection = mouse.hovered.unwrap_or(selection);
        let mut selection = input.navigate(selection, 3);
        if let Some(row) = mouse.clicked {
            selection = row;
        }
        self.state = GameState::DifficultySelect { selection };

        if input.back {
            self.state = GameState::TitleScreen { selection: 0 };
        } else if input.confirm || mouse.clicked.is_some() {
            self.settings.mode = GameMode::SinglePlayer;
            self.settings.difficulty = match selection {
                0 => Difficulty::Easy,
                1 => Difficulty::Medium,
                _ => Difficulty::Hard,
            };
            self.state = GameState::ChaosSelect { selection: 0 };
        }
    }

    pub(crate) fn update_chaos_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let mouse = chaos_panel("", ctx.window_size).mouse_select(ctx.input);
        let count = ChaosMode::ALL.len() as u8;
        let selection = mouse.hovered.unwrap_or(selection);
        let mut selection = input.navigate(selection, count);
        if let Some(row) = mouse.clicked {
            selection = row;
        }
        self.state = GameState::ChaosSelect { selection };

        if input.back {
            self.state = GameState::TitleScreen { selection: 0 };
        } else if input.confirm || mouse.clicked.is_some() {
            self.settings.chaos = ChaosMode::ALL[selection as usize];
            // Mirror the runtime selection into the engine context so any
            // code reading ctx.chaos_mode agrees with self.settings.chaos.
            ctx.chaos_mode = self.settings.chaos;
            self.start_game(ctx.world);
        }
    }
}
