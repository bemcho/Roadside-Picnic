//! World environment: terrain, lighting, atmosphere, ambience.

mod ambience;
mod terrain;
mod terrain_material;

pub use terrain::{SPAWN, Terrain};

use bevy::prelude::*;

use crate::states::AppState;

use terrain_material::{TerrainMaterial, load_terrain_material, prepare_ground_textures};

/// Spawns the base world (terrain, sun, ambience) when entering the Zone,
/// and tears it down when leaving.
pub struct WorldPlugin;

/// Root marker for everything spawned by this plugin's OnEnter.
#[derive(Component)]
struct WorldRoot;

/// Directional light standing in for the sun.
#[derive(Component)]
struct Sun;

/// The terrain the stalker walks on.
#[derive(Component)]
struct Ground;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .insert_resource(Terrain::new())
            .insert_resource(GlobalAmbientLight {
                color: Color::srgb(0.62, 0.66, 0.58),
                brightness: 120.0,
                ..default()
            })
            .add_systems(OnEnter(AppState::InGame), spawn_world)
            .add_systems(Update, prepare_ground_textures)
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
) {
    let root = commands
        .spawn((WorldRoot, Transform::default(), Visibility::default()))
        .id();

    // Ambient audio loop, parented to the world root so it dies with it.
    let ambience_entity = ambience::spawn_ambience(&mut commands, &mut sources);
    commands.entity(root).add_child(ambience_entity);

    commands.entity(root).with_children(|world| {
        // Sun: low and hazy, as it always is over the Zone.
        world.spawn((
            Sun,
            DirectionalLight {
                illuminance: 9_000.0,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.6, -0.8, 0.0)),
        ));

        world.spawn((
            Ground,
            Mesh3d(meshes.add(terrain.build_mesh())),
            MeshMaterial3d(materials.add(load_terrain_material(&asset_server))),
        ));
    });
}

fn despawn_world(mut commands: Commands, roots: Query<Entity, With<WorldRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}
