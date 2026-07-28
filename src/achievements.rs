//! Pong-specific achievement definitions and unlock logic.
//!
//! Registered once in `init()`; `unlock_win_achievements()` is called from the
//! game-over transition in `check_win_condition` whenever the player (left
//! paddle / player 1) wins.

use engine_core::prelude::*;

use crate::types::{Difficulty, GameMode, PongGame};

/// IDs — kept as `&'static str` so the compiler catches typos at call sites.
pub(crate) const BEAT_CPU_EASY:   &str = "beat_cpu_easy";
pub(crate) const BEAT_CPU_MEDIUM: &str = "beat_cpu_medium";
pub(crate) const BEAT_CPU_HARD:   &str = "beat_cpu_hard";

pub(crate) const WIN_NORMAL:      &str = "win_normal";
pub(crate) const WIN_INSANE:      &str = "win_insane";
pub(crate) const WIN_RIDICULOUS:  &str = "win_ridiculous";
pub(crate) const WIN_INSICULOUS:  &str = "win_insiculous";

pub(crate) const TWO_PLAYER:      &str = "two_player";

pub(crate) const SHUTOUT_NORMAL:     &str = "shutout_normal";
pub(crate) const SHUTOUT_INSANE:     &str = "shutout_insane";
pub(crate) const SHUTOUT_RIDICULOUS: &str = "shutout_ridiculous";
pub(crate) const SHUTOUT_INSICULOUS: &str = "shutout_insiculous";

/// Every achievement id, in registration order.
pub(crate) const ALL_IDS: [&str; 12] = [
    BEAT_CPU_EASY, BEAT_CPU_MEDIUM, BEAT_CPU_HARD,
    WIN_NORMAL, WIN_INSANE, WIN_RIDICULOUS, WIN_INSICULOUS,
    TWO_PLAYER,
    SHUTOUT_NORMAL, SHUTOUT_INSANE, SHUTOUT_RIDICULOUS, SHUTOUT_INSICULOUS,
];

/// Grouped display order for the achievements page. First tuple element is
/// the section-header locale key, second is the list of ids under it.
pub(crate) const DISPLAY_SECTIONS: &[(&str, &[&str])] = &[
    ("ach.section.cpu",
        &[BEAT_CPU_EASY, BEAT_CPU_MEDIUM, BEAT_CPU_HARD]),
    ("ach.section.chaos",
        &[WIN_NORMAL, WIN_INSANE, WIN_RIDICULOUS, WIN_INSICULOUS]),
    ("ach.section.shutouts",
        &[SHUTOUT_NORMAL, SHUTOUT_INSANE, SHUTOUT_RIDICULOUS, SHUTOUT_INSICULOUS]),
    ("ach.section.multi",
        &[TWO_PLAYER]),
];

/// Register every Pong achievement with names/descriptions from the locale
/// tables (`ach.<id>.name` / `ach.<id>.desc`). Called from `Game::init` AND
/// again after a locale switch — `register` is an id-keyed insert, so
/// re-registering refreshes the display strings without touching unlock
/// state. (Beating a higher CPU difficulty cascades to unlock easier ones —
/// handled at unlock time, not registration.)
pub(crate) fn register_all(mgr: &mut AchievementManager, strings: &Strings) {
    for id in ALL_IDS {
        let name_key = format!("ach.{id}.name");
        let desc_key = format!("ach.{id}.desc");
        mgr.register(Achievement::new(
            id,
            strings.tr(&name_key).to_string(),
            strings.tr(&desc_key).to_string(),
        ));
    }
}

impl PongGame {
    /// Called from `check_win_condition` when a match ends (either side won).
    /// The left paddle is always the local player (single-player) or player 1
    /// (two-player), so `left_wins` tells us whether the local player won.
    pub(crate) fn unlock_win_achievements(&self, ctx: &mut GameContext, left_wins: bool) {
        match self.settings.mode {
            GameMode::TwoPlayer => {
                // "Friendly Rivalry" fires regardless of who won — it's for
                // *playing* a 2P match to completion.
                ctx.achievements.unlock(TWO_PLAYER);
            }
            GameMode::SinglePlayer if left_wins => {
                // CPU-win cascade: winning at a harder difficulty also grants
                // the easier ones (implies you could've won those too).
                let cpu_ids: &[&str] = match self.settings.difficulty {
                    Difficulty::Easy   => &[BEAT_CPU_EASY],
                    Difficulty::Medium => &[BEAT_CPU_EASY, BEAT_CPU_MEDIUM],
                    Difficulty::Hard   => &[BEAT_CPU_EASY, BEAT_CPU_MEDIUM, BEAT_CPU_HARD],
                };
                for id in cpu_ids {
                    ctx.achievements.unlock(id);
                }

                // Chaos-mode win. Pong mutates `self.settings.chaos` from its
                // own menu, so it's the source of truth (not `ctx.chaos_mode`).
                ctx.achievements.unlock(chaos_win_id(self.settings.chaos));

                // Shutout — mode-specific, difficulty ignored.
                if self.score.right == 0 {
                    ctx.achievements.unlock(chaos_shutout_id(self.settings.chaos));
                }
            }
            _ => {} // Single-player loss — no achievements.
        }
    }
}

fn chaos_win_id(mode: ChaosMode) -> &'static str {
    match mode {
        ChaosMode::Normal     => WIN_NORMAL,
        ChaosMode::Insane     => WIN_INSANE,
        ChaosMode::Ridiculous => WIN_RIDICULOUS,
        ChaosMode::Insiculous => WIN_INSICULOUS,
    }
}

fn chaos_shutout_id(mode: ChaosMode) -> &'static str {
    match mode {
        ChaosMode::Normal     => SHUTOUT_NORMAL,
        ChaosMode::Insane     => SHUTOUT_INSANE,
        ChaosMode::Ridiculous => SHUTOUT_RIDICULOUS,
        ChaosMode::Insiculous => SHUTOUT_INSICULOUS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game's real locale tables, loaded from assets/locales.
    fn real_strings() -> Strings {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/locales");
        Strings::load_dir(&dir)
    }

    #[test]
    fn register_all_adds_twelve() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr, &Strings::empty());
        assert_eq!(mgr.total(), 12);
    }

    #[test]
    fn register_all_uses_locale_names_and_rereg_keeps_unlocks() {
        let strings = real_strings();
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr, &strings);
        assert_eq!(mgr.get(BEAT_CPU_EASY).unwrap().name, "Training Wheels");

        mgr.unlock(BEAT_CPU_EASY);
        assert!(mgr.is_unlocked(BEAT_CPU_EASY));

        // Switch locale, re-register: names refresh, unlock state survives.
        let mut strings = strings;
        strings.set_locale("pirate");
        register_all(&mut mgr, &strings);
        assert_eq!(mgr.get(BEAT_CPU_EASY).unwrap().name, "Barnacle Scraper");
        assert!(mgr.is_unlocked(BEAT_CPU_EASY), "re-registering must not reset unlocks");
    }

    #[test]
    fn locale_files_have_matching_keys() {
        let strings = real_strings();
        let en = strings.locale_keys("en").expect("en.ron loads");
        let pirate = strings.locale_keys("pirate").expect("pirate.ron loads");
        assert!(!en.is_empty(), "en locale must define keys");
        assert_eq!(en, pirate, "en.ron and pirate.ron must define the same key set");
    }

    #[test]
    fn every_achievement_id_has_name_and_desc_keys() {
        let strings = real_strings();
        let en = strings.locale_keys("en").expect("en.ron loads");
        for id in ALL_IDS {
            let name_key = format!("ach.{id}.name");
            let desc_key = format!("ach.{id}.desc");
            assert!(en.contains(&name_key.as_str()), "{name_key} missing from en.ron");
            assert!(en.contains(&desc_key.as_str()), "{desc_key} missing from en.ron");
        }
    }

    #[test]
    fn chaos_win_id_maps_each_mode() {
        assert_eq!(chaos_win_id(ChaosMode::Normal),     WIN_NORMAL);
        assert_eq!(chaos_win_id(ChaosMode::Insane),     WIN_INSANE);
        assert_eq!(chaos_win_id(ChaosMode::Ridiculous), WIN_RIDICULOUS);
        assert_eq!(chaos_win_id(ChaosMode::Insiculous), WIN_INSICULOUS);
    }

    #[test]
    fn chaos_shutout_id_maps_each_mode() {
        assert_eq!(chaos_shutout_id(ChaosMode::Normal),     SHUTOUT_NORMAL);
        assert_eq!(chaos_shutout_id(ChaosMode::Insane),     SHUTOUT_INSANE);
        assert_eq!(chaos_shutout_id(ChaosMode::Ridiculous), SHUTOUT_RIDICULOUS);
        assert_eq!(chaos_shutout_id(ChaosMode::Insiculous), SHUTOUT_INSICULOUS);
    }

    #[test]
    fn display_sections_cover_every_registered_achievement() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr, &Strings::empty());

        let shown: std::collections::HashSet<&str> = DISPLAY_SECTIONS
            .iter()
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect();

        for ach in mgr.all() {
            assert!(
                shown.contains(ach.id.as_str()),
                "{} registered but not in DISPLAY_SECTIONS",
                ach.id
            );
        }
        assert_eq!(shown.len(), mgr.total(), "DISPLAY_SECTIONS has duplicates or extras");
    }

    #[test]
    fn every_id_is_registered() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr, &Strings::empty());
        for id in [
            BEAT_CPU_EASY, BEAT_CPU_MEDIUM, BEAT_CPU_HARD,
            WIN_NORMAL, WIN_INSANE, WIN_RIDICULOUS, WIN_INSICULOUS,
            TWO_PLAYER,
            SHUTOUT_NORMAL, SHUTOUT_INSANE, SHUTOUT_RIDICULOUS, SHUTOUT_INSICULOUS,
        ] {
            assert!(mgr.get(id).is_some(), "{} not registered", id);
        }
    }
}
