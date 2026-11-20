//! Player-facing display strings.
//!
//! Kept in one place so they can move into a localization table later.
//! Systems must use these instead of inline string literals.

/// Title shown in the HUD corner.
pub const HUD_TITLE: &str = "THE ZONE";

/// Hint shown in the HUD corner.
pub const HUD_CONTROLS_HINT: &str = "WASD: move | M: mute | ESC: pause | Q: quit";
pub const HUD_CONTROLS_MUTED: &str = "WASD: move | M: unmute | ESC: pause | Q: quit";
pub const HUD_DRIVER_CONTROLS: &str =
    "W/S: accelerate/reverse | A/D: steer | SPACE: brake | M: mute";
pub const HUD_DRIVER_MUTED: &str =
    "W/S: accelerate/reverse | A/D: steer | SPACE: brake | M: unmute";
pub const HUD_ENTER_VEHICLE: &str = "Press E to enter vehicle";
pub const HUD_EXIT_VEHICLE: &str = "Press E to exit vehicle";

/// Title shown in the pause menu.
pub const PAUSE_TITLE: &str = "PAUSED";

/// Hint shown in the pause menu.
pub const PAUSE_HINT: &str = "ESC — resume";

/// Formats the per-run Zone statistics line for the HUD.
pub fn hud_zone_stats(artifacts: u32, anomalies: u32) -> String {
    format!("Artifacts: {artifacts} | Anomalies triggered: {anomalies}")
}
