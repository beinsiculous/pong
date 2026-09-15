//! Visual effect presets — particle configs.
//!
//! Centralizes the look of each event (paddle hit, goal explosion) so tuning
//! happens in one place. The backdrop grid is not here: it is a `GridBackdrop` entity
//! the engine simulates and draws (see `spawning::spawn_backdrop`), and gameplay events
//! reach it by queueing an impulse rather than by building vertices.

use engine_core::prelude::*;

/// Particles thrown off a paddle hit. Cone aligned with the bounce normal so
/// the spray reads as kinetic.
pub(crate) fn paddle_hit_burst(color: Vec4, normal: Vec2, theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let count = (24.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.25, 0.55)
        .with_speed(140.0, 320.0)
        .with_direction(normal, std::f32::consts::FRAC_PI_3) // ~60° half-cone
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(6.0, 0.5)
        .with_drag(2.5)
        .with_emissive(2.0)
        .with_texture(tex)
}

/// Larger, omnidirectional explosion for a scored goal.
pub(crate) fn goal_explosion(color: Vec4, theme: &ChaosTheme, tex: u32) -> ParticleConfig {
    let count = (80.0 * theme.particle_count_mult).round() as usize;
    ParticleConfig::burst(count)
        .with_lifetime(0.4, 1.0)
        .with_speed(120.0, 520.0)
        .with_direction(Vec2::Y, std::f32::consts::PI) // full circle
        .with_color(color, Vec4::new(color.x, color.y, color.z, 0.0))
        .with_scale(10.0, 0.5)
        .with_drag(1.6)
        .with_emissive(3.0)
        .with_texture(tex)
}
