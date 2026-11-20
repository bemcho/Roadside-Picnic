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
pub const TREE_TALL: &str = "models/trees/tall_tree.glb";
pub const TREE_ROADSIDE: &str = "models/trees/roadside_tree.glb";

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
