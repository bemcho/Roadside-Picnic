//! Player-facing display strings.
//!
//! Kept in one place so they can move into a localization table later.
//! Systems must use these instead of inline string literals.

pub const SPLASH_TITLE: &str = "ROADSIDE PICNIC";
pub const SPLASH_LOADING: &str = "Loading the Zone...";
pub const SPLASH_READY: &str = "Press any key to continue";
pub const SPLASH_FAILED: &str = "Could not load the Zone. Close the game and try again.";

/// Title shown in the HUD corner.
pub const HUD_TITLE: &str = "THE ZONE";
pub const HUD_FPS_PENDING: &str = "FPS: --";

/// Hint shown in the HUD corner.
pub const HUD_CONTROLS_HINT: &str = "WASD: move | M: mute | ESC: pause | Q: quit";
pub const HUD_CONTROLS_MUTED: &str = "WASD: move | M: unmute | ESC: pause | Q: quit";
pub const HUD_DRIVER_CONTROLS: &str =
    "W/S: accelerate/reverse | A/D: steer | SPACE: brake | M: mute";
pub const HUD_DRIVER_MUTED: &str =
    "W/S: accelerate/reverse | A/D: steer | SPACE: brake | M: unmute";
pub const HUD_ENTER_VEHICLE: &str = "Press E to enter vehicle";
pub const HUD_EXIT_VEHICLE: &str = "Press E to exit vehicle";

pub struct ArtifactCopy {
    pub title: &'static str,
    pub description: &'static str,
}

pub const ARTIFACT_READOUT_LABEL: &str = "RECOVERED ARTIFACT";

pub const ARTIFACT_COPY: [ArtifactCopy; 14] = [
    ArtifactCopy {
        title: "Death Lamp",
        description: "A cylinder that emits invisible radiation lethal to living things within its range.",
    },
    ArtifactCopy {
        title: "Golden Sphere / Golden Ball",
        description: "A heavy, warm sphere said to reveal a person's deepest true desire and grant it, with a human sacrifice required to bypass its guardian anomaly.",
    },
    ArtifactCopy {
        title: "Itcher",
        description: "Proximity can cause severe distress, panic, aggression, and uncontrollable barking or howling in animals and humans.",
    },
    ArtifactCopy {
        title: "Lobster Eyes",
        description: "Glassy, iridescent spheres whose optical properties remain unexplained.",
    },
    ArtifactCopy {
        title: "Spinning Ring / Perpetual Ring",
        description: "A white metal ring that spins endlessly on a flat surface without visible friction or energy loss.",
    },
    ArtifactCopy {
        title: "Sponge",
        description: "A porous object believed to absorb and neutralize certain Zone toxins or radiation.",
    },
    ArtifactCopy {
        title: "White Ring / Rattling Napkin",
        description: "A strange, non-reactive curiosity; no reliable practical use has been discovered.",
    },
    ArtifactCopy {
        title: "Wriggling Magnet",
        description: "A magnetic object that vibrates and oscillates continuously despite having no visible power source.",
    },
    ArtifactCopy {
        title: "Black Spark / Black Spray",
        description: "A black bead or droplet that absorbs and delays light, making movement appear visually out of step.",
    },
    ArtifactCopy {
        title: "Bracelet",
        description: "A metallic ring that emits radiation or stimulates biological processes, affecting health and metabolism.",
    },
    ArtifactCopy {
        title: "Empty / Spacer",
        description: "Two copper discs held about eighteen inches apart; the empty gap resists forces trying to close or separate them.",
    },
    ArtifactCopy {
        title: "Full Empty",
        description: "An Empty containing luminous blue fluid within the gap. It is rarer, heavier, and less understood than a standard Empty.",
    },
    ArtifactCopy {
        title: "Pins & Needles",
        description: "Small crystalline or metallic objects that emit faint colored flashes or magnetic pulses; their purpose is unknown.",
    },
    ArtifactCopy {
        title: "So-So / Etak",
        description: "Small black cylinders or discs called batteries. They generate endless electricity and can self-replicate under certain conditions.",
    },
];

/// Title shown in the pause menu.
pub const PAUSE_TITLE: &str = "PAUSED";

/// Hint shown in the pause menu.
pub const PAUSE_HINT: &str = "ESC — resume";

/// Formats the per-run Zone statistics line for the HUD.
pub fn hud_zone_stats(artifacts: u32, anomalies: u32) -> String {
    format!("Artifacts: {artifacts} | Anomalies triggered: {anomalies}")
}

pub fn hud_fps(fps: f64) -> String {
    format!("FPS: {fps:.0}")
}
