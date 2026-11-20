//! Player-facing display strings.
//!
//! Kept in one place so they can move into a localization table later.
//! Systems must use these instead of inline string literals.

/// Title shown in the HUD corner.
pub const HUD_TITLE: &str = "THE ZONE";

/// Hint shown in the HUD corner.
pub const HUD_CONTROLS_HINT: &str = "WASD — move | ESC — pause | Q — quit";

/// Title shown in the pause menu.
pub const PAUSE_TITLE: &str = "PAUSED";

/// Hint shown in the pause menu.
pub const PAUSE_HINT: &str = "ESC — resume";

/// Formats the per-run Zone statistics line for the HUD.
pub fn hud_zone_stats(artifacts: u32, anomalies: u32) -> String {
    format!("Artifacts: {artifacts} | Anomalies triggered: {anomalies}")
}
