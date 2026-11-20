//! Asset path constants.
//!
//! All asset handles are loaded through `AssetServer` using these paths.
//! Never scatter inline string literals for asset paths across systems.
//!
//! The `#[expect(dead_code)]` markers are intentional: these constants are
//! the project's forward-looking contract for asset locations. When a module
//! starts using one, the compiler will demand the attribute be removed.

use bevy::gltf::GltfMaterial;
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::shader::Shader;

pub const SPLASH_HANGAR: &str = "textures/splash/hangar.png";
pub const STALKER_MODEL: &str = "models/stalker.glb";
pub const TERRAIN_SHADER: &str = "shaders/terrain.wesl";
pub const WATER_SHADER: &str = "shaders/water.wesl";
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
pub const DRY_GRASS: &str = "models/plants/drygrass.glb";
pub const GLOWWEED: &str = "models/plants/glowweed.glb";
pub const HOGWEED: &str = "models/plants/hogweed.glb";
pub const SHRUB: &str = "models/plants/shrub.glb";
pub const THORNBUSH: &str = "models/plants/thornbush.glb";
pub const TREE_SMALL: &str = "models/trees/tree_small.glb";
pub const TREE_BIG: &str = "models/trees/tree_big.glb";
pub const TREE_VERY_TALL: &str = "models/trees/tree_veryhigh.glb";
pub const BASE_HANGAR: &str = "models/base/hangar.glb";
pub const BASE_VEHICLE: &str = "models/base/armored_vehicle.glb";
pub const HANGAR_LAB: &str = "models/base/props/lab.glb";
pub const HANGAR_MAP_BOARD: &str = "models/base/props/mapboard.glb";
pub const HANGAR_REPAIR_STAND: &str = "models/base/props/repairstand.glb";
pub const HANGAR_TABLE: &str = "models/base/props/table.glb";
pub const HANGAR_TOILET: &str = "models/base/props/toilet.glb";
pub const HANGAR_TOOLS: &str = "models/base/props/tools.glb";
pub const HANGAR_WORKSHOP: &str = "models/base/props/workshop.glb";

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
        BASE_HANGAR,
        BASE_VEHICLE,
        FAUNA_DEAD_FOREST,
        FAUNA_OVERGROWTH,
        FAUNA_MUD_BOG,
        FAUNA_SLIME_DITCH,
        HANGAR_LAB,
        HANGAR_MAP_BOARD,
        HANGAR_REPAIR_STAND,
        HANGAR_TABLE,
        HANGAR_TOILET,
        HANGAR_TOOLS,
        HANGAR_WORKSHOP,
    ];
    let albedo = [TERRAIN_GRASS_ALBEDO, TERRAIN_EARTH_ALBEDO];
    let data_maps = [
        TERRAIN_GRASS_NORMAL,
        TERRAIN_GRASS_SURFACE,
        TERRAIN_EARTH_NORMAL,
        TERRAIN_EARTH_SURFACE,
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
        server.load::<Shader>(WATER_SHADER).untyped(),
    ]);
}

/// Loads the shared mesh and material of a single-primitive GLB.
pub(crate) fn load_single_primitive(
    server: &AssetServer,
    path: &'static str,
) -> (Handle<Mesh>, Handle<GltfMaterial>) {
    (
        server.load(
            GltfAssetLabel::Primitive {
                mesh: 0,
                primitive: 0,
            }
            .from_asset(path),
        ),
        server.load(
            GltfAssetLabel::Material {
                index: 0,
                is_scale_inverted: false,
            }
            .from_asset(path),
        ),
    )
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
