//! World environment: terrain, lighting, atmosphere, ambience.

mod ambience;
mod base;
mod collision;
mod terrain;
mod terrain_material;
mod trees;

pub(crate) use collision::STEP_HEIGHT;
pub use collision::{Body, Collider, Pillar, push_out};
pub use terrain::{HALF_SIZE as TERRAIN_HALF_SIZE, SPAWN, Terrain};

use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::{Atmosphere, SunDisk, light_consts::lux};
use bevy::prelude::*;

use crate::states::AppState;

use terrain_material::{TerrainMaterial, load_terrain_material, mipmap_tiling_textures};

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

/// Where the trees stand; computed once, spawned on each entry to the Zone.
#[derive(Resource)]
struct TreeSites(Vec<trees::TreeSite>);

/// The terrain the stalker walks on.
#[derive(Component)]
struct Ground;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        let terrain = Terrain::new();
        let tree_sites = trees::tree_sites(&terrain);
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .insert_resource(TreeSites(tree_sites))
            .insert_resource(terrain)
            .add_systems(
                OnEnter(AppState::InGame),
                (spawn_world, spawn_forest, base::spawn_base),
            )
            .add_systems(Update, mipmap_tiling_textures)
            .add_systems(OnExit(AppState::InGame), despawn_world);
    }
}

fn spawn_world(
    mut commands: Commands,
    terrain: Res<Terrain>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut mediums: ResMut<Assets<ScatteringMedium>>,
) {
    let root = commands
        .spawn((WorldRoot, Transform::default(), Visibility::default()))
        .id();

    // Ambient audio loop, parented to the world root so it dies with it.
    let ambience_entity = ambience::spawn_ambience(&mut commands, &mut sources);
    commands.entity(root).add_child(ambience_entity);

    // Earth-like sky, thickened so the Zone sits in a dusty haze.
    let medium = mediums.add(ScatteringMedium::earth(256, 256).with_density_multiplier(HAZE));
    let atmosphere = Atmosphere::earth(medium);
    let planet_center = Transform::from_xyz(0.0, -atmosphere.inner_radius, 0.0);

    commands.entity(root).with_children(|world| {
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

        world.spawn((
            Ground,
            Mesh3d(meshes.add(terrain.build_mesh())),
            MeshMaterial3d(materials.add(load_terrain_material(&asset_server))),
        ));
    });
}

fn spawn_forest(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    tree_sites: Res<TreeSites>,
) {
    commands
        .spawn((WorldRoot, Transform::default(), Visibility::default()))
        .with_children(|forest| trees::spawn_trees(forest, &asset_server, &tree_sites.0));
}

fn despawn_world(mut commands: Commands, roots: Query<Entity, With<WorldRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}
