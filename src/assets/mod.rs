//! Asset path constants.
//!
//! All asset handles are loaded through `AssetServer` using these paths.
//! Never scatter inline string literals for asset paths across systems.
//!
//! The `#[expect(dead_code)]` markers are intentional: these constants are
//! the project's forward-looking contract for asset locations. When a module
//! starts using one, the compiler will demand the attribute be removed.

pub const TERRAIN_SHADER: &str = "shaders/terrain.wesl";
pub const TERRAIN_GRASS_ALBEDO: &str = "textures/terrain/grass_albedo.png";
pub const TERRAIN_GRASS_NORMAL: &str = "textures/terrain/grass_normal.png";
pub const TERRAIN_GRASS_SURFACE: &str = "textures/terrain/grass_surface.png";
pub const TERRAIN_EARTH_ALBEDO: &str = "textures/terrain/earth_albedo.png";
pub const TERRAIN_EARTH_NORMAL: &str = "textures/terrain/earth_normal.png";
pub const TERRAIN_EARTH_SURFACE: &str = "textures/terrain/earth_surface.png";
pub const FAUNA_DEAD_FOREST: &str = "models/fauna/dead_forest.glb";
pub const FAUNA_OVERGROWTH: &str = "models/fauna/overgrowth.glb";
pub const FAUNA_MUD_BOG: &str = "models/fauna/mud_bog.glb";
pub const FAUNA_SLIME_DITCH: &str = "models/fauna/slime_ditch.glb";
pub const BASE_YARD: &str = "models/base/warehouse.glb";
pub const BASE_VEHICLE: &str = "models/base/armored_vehicle.glb";
pub const BASE_CONCRETE_ALBEDO: &str = "textures/base/concrete_albedo.png";
pub const BASE_CONCRETE_NORMAL: &str = "textures/base/concrete_normal.png";
pub const BASE_CONCRETE_ARM: &str = "textures/base/concrete_arm.png";
pub const BASE_WALL_ALBEDO: &str = "textures/base/wall_albedo.png";
pub const BASE_WALL_NORMAL: &str = "textures/base/wall_normal.png";
pub const BASE_WALL_ARM: &str = "textures/base/wall_arm.png";
pub const BASE_STEEL_ALBEDO: &str = "textures/base/steel_albedo.png";
pub const BASE_STEEL_NORMAL: &str = "textures/base/steel_normal.png";
pub const BASE_STEEL_ARM: &str = "textures/base/steel_arm.png";
pub const BASE_YELLOW_ALBEDO: &str = "textures/base/yellow_albedo.png";
pub const BASE_HAZARD_ALBEDO: &str = "textures/base/hazard_albedo.png";
pub const BASE_GRATE_ALBEDO: &str = "textures/base/grate_albedo.png";

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
