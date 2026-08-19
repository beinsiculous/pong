//! Menu screens: navigation and selection. Match lifecycle (starting the
//! game, theming) lives in `gameplay::flow`.

use engine_core::prelude::*;
use crate::types::*;

/// Panel layouts shared by the input half (mouse hit-testing here) and the
/// drawing half (`ui.rs`) — the geometry must match or clicks land beside
/// the drawn rows. Titles only affect the label, never the layout.
pub(crate) fn title_panel(title: &str, window_size: Vec2) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, 360.0, 5)
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
        let mut selection = input.navigate(selection, 5);
        if let Some(row) = mouse.clicked {
            selection = row;
        }
        self.state = GameState::TitleScreen { selection };

        if input.confirm || mouse.clicked.is_some() {
            match selection {
                0 => self.state = GameState::DifficultySelect { selection: 1 },
                1 => {
                    self.settings.mode = GameMode::TwoPlayer;
                    self.state = GameState::ChaosSelect { selection: 0 };
                }
                2 => self.state = GameState::Achievements,
                3 => {
                    // Language: cycle locale, then re-register achievements
                    // so their names/descriptions pick up the new language
                    // (id-keyed insert — unlock state is untouched).
                    ctx.strings.cycle_locale();
                    crate::achievements::register_all(ctx.achievements, ctx.strings);
                }
                _ => ctx.exit_requested = true,
            }
        }
    }

    pub(crate) fn update_achievements_input(&mut self, ctx: &mut GameContext) {
        let input = MenuInput::read(ctx.input);
        // Whole-window dismiss: clicks on headers/margins count too, not
        // just the row bands (the page is one big info sheet).
        let click_dismiss = achievements_panel("", ctx.window_size).clicked_inside(ctx.input);
        if input.back || input.confirm || click_dismiss {
            self.state = GameState::TitleScreen { selection: 2 };
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
