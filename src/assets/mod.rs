//! Asset path constants.
//!
//! All asset handles are loaded through `AssetServer` using these paths.
//! Never scatter inline string literals for asset paths across systems.
//!
//! The `#[expect(dead_code)]` markers are intentional: these constants are
//! the project's forward-looking contract for asset locations. When a module
//! starts using one, the compiler will demand the attribute be removed.

pub const TERRAIN_SHADER: &str = "shaders/terrain.wesl";
pub const TERRAIN_GRASS_ALBEDO: &str = "textures/terrain/grass_albedo.jpg";
pub const TERRAIN_GRASS_NORMAL: &str = "textures/terrain/grass_normal.jpg";
pub const TERRAIN_GRASS_ARM: &str = "textures/terrain/grass_arm.jpg";
pub const TERRAIN_EARTH_ALBEDO: &str = "textures/terrain/earth_albedo.jpg";
pub const TERRAIN_EARTH_NORMAL: &str = "textures/terrain/earth_normal.jpg";
pub const TERRAIN_EARTH_ARM: &str = "textures/terrain/earth_arm.jpg";
pub const FAUNA_DEAD_FOREST: &str = "models/fauna/dead_forest.glb";
pub const FAUNA_OVERGROWTH: &str = "models/fauna/overgrowth.glb";
pub const FAUNA_MUD_BOG: &str = "models/fauna/mud_bog.glb";
pub const FAUNA_SLIME_DITCH: &str = "models/fauna/slime_ditch.glb";
pub const BASE_YARD: &str = "models/base/warehouse.glb";
pub const BASE_VEHICLE: &str = "models/base/armored_vehicle.glb";
pub const BASE_CONCRETE_ALBEDO: &str = "textures/base/concrete_albedo.jpg";
pub const BASE_CONCRETE_NORMAL: &str = "textures/base/concrete_normal.jpg";
pub const BASE_CONCRETE_ARM: &str = "textures/base/concrete_arm.jpg";
pub const BASE_CORRUGATED_ALBEDO: &str = "textures/base/corrugated_albedo.jpg";
pub const BASE_CORRUGATED_NORMAL: &str = "textures/base/corrugated_normal.jpg";
pub const BASE_CORRUGATED_ARM: &str = "textures/base/corrugated_arm.jpg";

/// Root folder for textures and materials.
#[expect(dead_code)]
pub const TEXTURES: &str = "textures";

/// Root folder for 3D models (the stalker lives here as `models/stalker.glb`).
pub const MODELS: &str = "models";

/// Root folder for audio files.
#[expect(dead_code)]
pub const AUDIO: &str = "audio";

/// Root folder for fonts.
#[expect(dead_code)]
pub const FONTS: &str = "fonts";
