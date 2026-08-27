//! All on-screen text: menu screens, the achievements page, and the in-match
//! HUD (score, banners, serve/game-over prompts). Menu screens draw inside
//! the engine's `MenuPanel` window chrome, styled from the chaos theme.
//!
//! Every player-facing string goes through `ctx.strings.tr(...)` — the title
//! menu's Language item cycles locales (English/Pirate; Pirate also swaps
//! the font via its locale file's `font` field).

use engine_core::prelude::*;
use crate::achievements::DISPLAY_SECTIONS;
use crate::menu::{achievements_panel, chaos_panel, difficulty_panel, title_panel, TitleItem, TITLE_ITEMS};
use crate::types::*;

/// Pixel height of the full achievements list (headers + rows + gaps).
pub(crate) fn achievements_content_height() -> f32 {
    DISPLAY_SECTIONS
        .iter()
        .map(|(_, ids)| 22.0 + ids.len() as f32 * 36.0 + 6.0)
        .sum()
}

/// How far the achievements page can scroll: content height minus the
/// window's item viewport (panel minus title block, unlocked line, hint).
pub(crate) fn achievements_max_scroll(window_size: Vec2) -> f32 {
    let rect = achievements_panel("", window_size).panel_rect();
    let viewport = (rect.height - 130.0).max(0.0);
    (achievements_content_height() - viewport).max(0.0)
}

impl PongGame {
    fn menu_style(&self) -> MenuStyle {
        MenuStyle::from_theme(&self.current_theme())
    }

    pub(crate) fn draw_ui(&self, ctx: &mut GameContext) {
        match &self.state {
            GameState::TitleScreen { selection } => self.draw_title(ctx, *selection),
            GameState::DifficultySelect { selection } => self.draw_difficulty(ctx, *selection),
            GameState::ChaosSelect { selection } => self.draw_chaos(ctx, *selection),
            GameState::Achievements => self.draw_achievements(ctx),
            _ => self.draw_gameplay_hud(ctx),
        }
    }

    fn draw_title(&self, ctx: &mut GameContext, selection: u8) {
        let style = self.menu_style();
        let strings = &ctx.strings;
        let title = strings.tr("title.window").to_string();
        // Same row list the input half dispatches on — keep them in sync
        // by construction (the wasm build has no Achievements row).
        let items: Vec<String> = TITLE_ITEMS
            .iter()
            .map(|item| match item {
                TitleItem::SinglePlayer => strings.tr("title.single").to_string(),
                TitleItem::TwoPlayer => strings.tr("title.two").to_string(),
                TitleItem::Achievements => strings.tr("title.achievements").to_string(),
                TitleItem::Language => format!(
                    "{}: {}",
                    strings.tr("title.language"),
                    strings.current_display_name()
                ),
                TitleItem::Exit => strings.tr("title.exit").to_string(),
            })
            .collect();
        let hint = strings.tr("title.hint").to_string();

        let panel = title_panel(&title, ctx.window_size);
        let mut y = panel.begin(ctx.ui, &style);
        for (i, item) in items.iter().enumerate() {
            y = panel.item(ctx.ui, y, item, i as u8 == selection, &style);
        }
        panel.hint(ctx.ui, &hint, &style);
    }

    fn draw_achievements(&self, ctx: &mut GameContext) {
        let style = self.menu_style();
        let total = ctx.achievements.total();
        let unlocked = ctx.achievements.unlocked_count();

        // Tall window covering most of the screen; the section list draws
        // left-aligned inside its bounds.
        let window_title = ctx.strings.tr("ach.window").to_string();
        let panel = achievements_panel(&window_title, ctx.window_size);
        let first_y = panel.begin(ctx.ui, &style);
        let rect = panel.panel_rect();

        let unlocked_word = ctx.strings.tr("ach.unlocked").to_string();
        ctx.ui.label_centered(
            &format!("{unlocked} / {total} {unlocked_word}"),
            Vec2::new(ctx.window_size.x / 2.0, first_y - 8.0),
        );

        let left = rect.x + 28.0;
        // Scroll window: rows outside [content_top, content_bottom] are
        // culled — immediate-mode text would otherwise overpaint the title
        // and hint (the panel does not clip).
        let content_top = first_y + 12.0;
        let content_bottom = rect.y + rect.height - 52.0;
        let mut y = first_y + 18.0 - self.achievements_scroll;

        let locked_color = Color::new(0.45, 0.45, 0.5, 1.0);
        let unlocked_color = Color::new(1.0, 0.85, 0.25, 1.0);
        let desc_color = Color::new(0.75, 0.75, 0.8, 1.0);
        let header_color = Color::new(0.6, 0.75, 1.0, 1.0);

        for (section_key, ids) in DISPLAY_SECTIONS {
            if y >= content_top && y <= content_bottom {
                let section = ctx.strings.tr(section_key).to_string();
                ctx.ui.label_styled(&section, Vec2::new(left, y), header_color, 16.0);
            }
            y += 22.0;
            for id in *ids {
                let is_unlocked = ctx.achievements.is_unlocked(id);
                // Registry always has entries for these ids (registered in init).
                let Some(ach) = ctx.achievements.get(id) else { continue };

                let (marker, name_color) = if is_unlocked {
                    ("[X]", unlocked_color)
                } else {
                    ("[ ]", locked_color)
                };

                let (name, desc) = if !is_unlocked && ach.hidden {
                    (
                        ctx.strings.tr("ach.hidden_name").to_string(),
                        ctx.strings.tr("ach.hidden_desc").to_string(),
                    )
                } else {
                    (ach.name.clone(), ach.description.clone())
                };

                if y >= content_top && y + 16.0 <= content_bottom {
                    ctx.ui.label_styled(
                        &format!("{marker} {name}"),
                        Vec2::new(left + 8.0, y),
                        name_color,
                        14.0,
                    );
                    ctx.ui.label_styled(&desc, Vec2::new(left + 52.0, y + 16.0), desc_color, 12.0);
                }
                y += 36.0;
            }
            y += 6.0;
        }

        // Overflow markers: show which directions still have content.
        let marker_x = rect.x + rect.width - 34.0;
        if self.achievements_scroll > 0.0 {
            ctx.ui.label_styled("^", Vec2::new(marker_x, content_top + 4.0), desc_color, 14.0);
        }
        if self.achievements_scroll < achievements_max_scroll(ctx.window_size) {
            ctx.ui.label_styled("v", Vec2::new(marker_x, content_bottom + 12.0), desc_color, 14.0);
        }

        let hint = ctx.strings.tr("ach.hint").to_string();
        panel.hint(ctx.ui, &hint, &style);
    }

    fn draw_difficulty(&self, ctx: &mut GameContext, selection: u8) {
        let style = self.menu_style();
        let window_title = ctx.strings.tr("diff.window").to_string();
        let panel = difficulty_panel(&window_title, ctx.window_size);
        let mut y = panel.begin(ctx.ui, &style);
        let items = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];
        for (i, diff) in items.iter().enumerate() {
            let label = ctx.strings.tr(diff.label_key()).to_string();
            y = panel.item(ctx.ui, y, &label, i as u8 == selection, &style);
        }
        let hint = ctx.strings.tr("diff.hint").to_string();
        panel.hint(ctx.ui, &hint, &style);
    }

    fn draw_chaos(&self, ctx: &mut GameContext, selection: u8) {
        let style = self.menu_style();
        let window_title = ctx.strings.tr("chaos.window").to_string();
        let panel = chaos_panel(&window_title, ctx.window_size);
        let mut y = panel.begin(ctx.ui, &style);
        for (i, mode) in ChaosMode::ALL.iter().enumerate() {
            let label = ctx.strings.tr(chaos_label_key(*mode)).to_string();
            y = panel.item(ctx.ui, y, &label, i as u8 == selection, &style);
        }

        let hint_key = match ChaosMode::ALL[selection as usize] {
            ChaosMode::Normal => "chaos.normal.desc",
            ChaosMode::Insane => "chaos.insane.desc",
            ChaosMode::Ridiculous => "chaos.ridiculous.desc",
            ChaosMode::Insiculous => "chaos.insiculous.desc",
        };
        let hint = ctx.strings.tr(hint_key).to_string();
        panel.hint(ctx.ui, &hint, &style);
    }

    fn draw_gameplay_hud(&self, ctx: &mut GameContext) {
        let cx = ctx.window_size.x / 2.0;
        let cy = ctx.window_size.y / 2.0;

        let score_text = match self.settings.mode {
            GameMode::SinglePlayer => format!(
                "{} {}  :  {} {}",
                ctx.strings.tr("hud.you"), self.score.left,
                self.score.right, ctx.strings.tr("hud.cpu"),
            ),
            GameMode::TwoPlayer => format!(
                "{} {}  :  {} {}",
                ctx.strings.tr("hud.p1"), self.score.left,
                self.score.right, ctx.strings.tr("hud.p2"),
            ),
        };
        // y is the text BASELINE: the top wall spans screen y 0..20 and the
        // bottom wall (window_h-20)..window_h, so keep glyphs clear of both.
        ctx.ui.label_centered(&score_text, Vec2::new(cx, 42.0));

        let theme = self.current_theme();
        if let Some(banner) = theme.banner_text {
            let color = Color::new(theme.banner_color.x, theme.banner_color.y, theme.banner_color.z, theme.banner_color.w);
            ctx.ui.label_centered_styled(banner, Vec2::new(cx, ctx.window_size.y - 32.0), color, 16.0);
        }

        if self.power_ups.speed_boost.active() {
            let boost_text = format!(
                "{} {:.1}s",
                ctx.strings.tr("hud.speed_boost"),
                self.power_ups.speed_boost.remaining(),
            );
            ctx.ui.label_centered(&boost_text, Vec2::new(cx, 48.0));
        }

        match &self.state {
            GameState::Serving => {
                let serve = ctx.strings.tr("serve.prompt").to_string();
                let pause = ctx.strings.tr("serve.pause").to_string();
                ctx.ui.label_centered(&serve, Vec2::new(cx, cy - 50.0));
                ctx.ui.label_centered(&pause, Vec2::new(cx, cy - 24.0));
            }
            GameState::GameOver { left_wins } => {
                let msg_key = match (self.settings.mode, *left_wins) {
                    (GameMode::SinglePlayer, true) => "over.you_win",
                    (GameMode::SinglePlayer, false) => "over.cpu_wins",
                    (GameMode::TwoPlayer, true) => "over.p1_wins",
                    (GameMode::TwoPlayer, false) => "over.p2_wins",
                };
                let msg = ctx.strings.tr(msg_key).to_string();
                let again = ctx.strings.tr("over.again").to_string();
                let hint = ctx.strings.tr("over.hint").to_string();
                let style = self.menu_style();
                let panel = MenuPanel::new(&msg, Vec2::new(cx, cy), 340.0, 1);
                let y = panel.begin(ctx.ui, &style);
                panel.line(ctx.ui, y, &again, &style);
                panel.hint(ctx.ui, &hint, &style);
            }
            _ => {}
        }

        if self.pause.is_active() {
            let style = self.menu_style();
            let labels = PauseMenuLabels {
                title: ctx.strings.tr("pause.title"),
                items: [
                    ctx.strings.tr("pause.resume"),
                    ctx.strings.tr("pause.restart"),
                    ctx.strings.tr("pause.quit_title"),
                    ctx.strings.tr("pause.exit_game"),
                ],
                hint: ctx.strings.tr("pause.hint"),
            };
            self.pause.draw_labeled(ctx.ui, ctx.window_size, &style, &labels);
        }
    }
}
