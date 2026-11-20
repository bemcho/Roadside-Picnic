//! Asset path constants.
//!
//! All asset handles are loaded through `AssetServer` using these paths.
//! Never scatter inline string literals for asset paths across systems.
//!
//! The `#[expect(dead_code)]` markers are intentional: these constants are
//! the project's forward-looking contract for asset locations. When a module
//! starts using one, the compiler will demand the attribute be removed.

use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::shader::Shader;

pub const SPLASH_HANGAR: &str = "textures/splash/hangar.png";
pub const STALKER_MODEL: &str = "models/stalker.glb";
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

/// Root folder for audio files.
#[expect(dead_code)]
pub const AUDIO: &str = "audio";

/// Root folder for fonts.
#[expect(dead_code)]
pub const FONTS: &str = "fonts";

#[derive(Resource, Default)]
pub(crate) struct PreloadedAssets(pub Vec<UntypedHandle>);

pub(crate) fn preload_game_assets(server: Res<AssetServer>, mut assets: ResMut<PreloadedAssets>) {
    let scenes = [
        STALKER_MODEL,
        BASE_YARD,
        BASE_VEHICLE,
        FAUNA_DEAD_FOREST,
        FAUNA_OVERGROWTH,
        FAUNA_MUD_BOG,
        FAUNA_SLIME_DITCH,
    ];
    let albedo = [
        TERRAIN_GRASS_ALBEDO,
        TERRAIN_EARTH_ALBEDO,
        BASE_CONCRETE_ALBEDO,
        BASE_WALL_ALBEDO,
        BASE_STEEL_ALBEDO,
        BASE_YELLOW_ALBEDO,
        BASE_HAZARD_ALBEDO,
        BASE_GRATE_ALBEDO,
    ];
    let data_maps = [
        TERRAIN_GRASS_NORMAL,
        TERRAIN_GRASS_SURFACE,
        TERRAIN_EARTH_NORMAL,
        TERRAIN_EARTH_SURFACE,
        BASE_CONCRETE_NORMAL,
        BASE_CONCRETE_ARM,
        BASE_WALL_NORMAL,
        BASE_WALL_ARM,
        BASE_STEEL_NORMAL,
        BASE_STEEL_ARM,
    ];
    assets.0.extend(scenes.map(|path| {
        server
            .load::<WorldAsset>(GltfAssetLabel::Scene(0).from_asset(path))
            .untyped()
    }));
    assets
        .0
        .extend(albedo.map(|path| load_repeating(&server, path, true).untyped()));
    assets
        .0
        .extend(data_maps.map(|path| load_repeating(&server, path, false).untyped()));
    assets.0.extend([
        server.load::<Image>(SPLASH_HANGAR).untyped(),
        server.load::<Shader>(TERRAIN_SHADER).untyped(),
    ]);
}

/// Loads a tiling texture: repeating, anisotropically filtered.
pub(crate) fn load_repeating(
    server: &AssetServer,
    path: &'static str,
    is_srgb: bool,
) -> Handle<Image> {
    server
        .load_builder()
        .with_settings(move |settings: &mut ImageLoaderSettings| {
            settings.is_srgb = is_srgb;
            settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                anisotropy_clamp: 8,
                ..ImageSamplerDescriptor::linear()
            });
        })
        .load(path)
}
