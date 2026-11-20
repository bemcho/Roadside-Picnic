//! World environment: terrain, lighting, atmosphere, ambience.

mod base;
mod collision;
mod hydrology;
mod landscape;
mod scenery;
mod terrain;
mod terrain_material;
mod trees;
mod water;

pub(crate) use collision::STEP_HEIGHT;
pub use collision::{Body, Collider, Pillar, push_out};
pub use terrain::{HALF_SIZE as TERRAIN_HALF_SIZE, SPAWN, Terrain};

use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::{Atmosphere, SunDisk, light_consts::lux};
use bevy::prelude::*;

use crate::states::AppState;

use landscape::{Landscape, LandscapeSource, prepare_landscape};
use scenery::{Scenery, stream_scenery};
use terrain_material::{TerrainMaterial, load_terrain_material, mipmap_tiling_textures};
use water::WaterMaterial;

/// Direction toward the sun: late afternoon, ahead of the stalker's start.
const SUN_DIRECTION: Vec3 = Vec3::new(-0.35, 0.19, -1.0);

/// Atmosphere density relative to Earth; thicker air reads as Zone haze.
const HAZE: f32 = 2.5;

/// Spawns the base world (terrain, sun, ambience) when entering the Zone,
/// and tears it down when leaving.
pub struct WorldPlugin;

/// Root marker for everything spawned by this plugin's OnEnter.
#[derive(Component)]
struct WorldRoot;

/// Directional light standing in for the sun.
#[derive(Component)]
struct Sun;

#[derive(Resource)]
struct LandscapeMaterials {
    terrain: Handle<TerrainMaterial>,
    water: Handle<WaterMaterial>,
    foliage: Handle<StandardMaterial>,
}

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MaterialPlugin::<TerrainMaterial>::default(),
            MaterialPlugin::<WaterMaterial>::default(),
        ))
        .init_asset::<Landscape>()
        .init_resource::<Terrain>()
        .add_systems(Startup, (prepare_landscape, load_landscape_materials))
        .add_systems(
            OnEnter(AppState::InGame),
            (spawn_world, spawn_sky, base::spawn_base),
        )
        .add_systems(Update, mipmap_tiling_textures)
        .add_systems(PreUpdate, stream_scenery.run_if(in_state(AppState::InGame)))
        .add_systems(OnExit(AppState::InGame), despawn_world);
    }
}

fn load_landscape_materials(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    mut standard: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.insert_resource(LandscapeMaterials {
        terrain: materials.add(load_terrain_material(&asset_server)),
        water: water_materials.add(water::material()),
        foliage: standard.add(scenery::foliage_material(&mut images)),
    });
}

fn spawn_world(
    mut commands: Commands,
    source: Res<LandscapeSource>,
    mut landscapes: ResMut<Assets<Landscape>>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<LandscapeMaterials>,
) {
    let mut landscape = landscapes
        .get_mut(&source.0)
        .expect("landscape loaded before entering the Zone");
    landscape.upload(&mut meshes);
    let root = commands
        .spawn((WorldRoot, Transform::default(), Visibility::default()))
        .id();
    commands.insert_resource(landscape.scenery(root));
    commands.entity(root).with_children(|world| {
        landscape.spawn(
            world,
            &materials.terrain,
            &materials.water,
            &materials.foliage,
        );
    });
}

fn spawn_sky(mut commands: Commands, mut mediums: ResMut<Assets<ScatteringMedium>>) {
    // Earth-like sky, thickened so the Zone sits in a dusty haze.
    let medium = mediums.add(ScatteringMedium::earth(256, 256).with_density_multiplier(HAZE));
    let atmosphere = Atmosphere::earth(medium);
    let planet_center = Transform::from_xyz(0.0, -atmosphere.inner_radius, 0.0);

    commands
        .spawn((WorldRoot, Transform::IDENTITY, Visibility::default()))
        .with_children(|world| {
            world.spawn((atmosphere, planet_center));

            // Sun: low over the hills ahead of the stalker. Raw (unscattered)
            // sunlight is the input the atmosphere expects.
            world.spawn((
                Sun,
                DirectionalLight {
                    illuminance: lux::RAW_SUNLIGHT,
                    shadow_maps_enabled: true,
                    ..default()
                },
                SunDisk::EARTH,
                Transform::from_translation(SUN_DIRECTION).looking_at(Vec3::ZERO, Vec3::Y),
            ));
        });
}

fn despawn_world(mut commands: Commands, roots: Query<Entity, With<WorldRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
    commands.remove_resource::<Scenery>();
}
