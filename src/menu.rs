//! Menu screens: navigation and selection. Match lifecycle (starting the
//! game, theming) lives in `gameplay::flow`.

use engine_core::prelude::*;
use crate::types::*;

impl PongGame {
    pub(crate) fn update_title_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let selection = input.navigate(selection, 5);
        self.state = GameState::TitleScreen { selection };

        if input.confirm {
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
        if input.back || input.confirm {
            self.state = GameState::TitleScreen { selection: 2 };
        }
    }

    pub(crate) fn update_difficulty_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let selection = input.navigate(selection, 3);
        self.state = GameState::DifficultySelect { selection };

        if input.back {
            self.state = GameState::TitleScreen { selection: 0 };
        } else if input.confirm {
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
        let count = ChaosMode::ALL.len() as u8;
        let selection = input.navigate(selection, count);
        self.state = GameState::ChaosSelect { selection };

        if input.back {
            self.state = GameState::TitleScreen { selection: 0 };
        } else if input.confirm {
            self.settings.chaos = ChaosMode::ALL[selection as usize];
            // Mirror the runtime selection into the engine context so any
            // code reading ctx.chaos_mode agrees with self.settings.chaos.
            ctx.chaos_mode = self.settings.chaos;
            self.start_game(ctx.world);
        }
    }
}
