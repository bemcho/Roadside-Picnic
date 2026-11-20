//! Player: the stalker, their camera, and movement controls.

use bevy::prelude::*;

use crate::states::{AppState, InGamePhase};
use crate::world::{SPAWN, Terrain};

pub struct PlayerPlugin;

/// The stalker. Movement speed is data, not magic numbers in systems.
#[derive(Component)]
pub struct Player {
    pub move_speed: f32,
}

/// Tags the camera that follows the stalker.
#[derive(Component)]
pub struct PlayerCamera;

/// Everything spawned by [`spawn_player`] and torn down by [`despawn_player`].
type PlayerEntities<'w, 's> = Query<'w, 's, Entity, Or<(With<Player>, With<PlayerCamera>)>>;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), spawn_player)
            .add_systems(OnExit(AppState::InGame), despawn_player)
            .add_systems(
                Update,
                (move_player, follow_player).run_if(in_state(InGamePhase::Running)),
            );
    }
}

fn spawn_player(
    mut commands: Commands,
    terrain: Res<Terrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ground_y = terrain.height_at(SPAWN.0, SPAWN.1);

    // The stalker: a plain capsule for now, to be replaced with a real model.
    commands.spawn((
        Player { move_speed: 5.0 },
        Mesh3d(meshes.add(Capsule3d::new(0.4, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.7, 0.65, 0.4),
            ..default()
        })),
        Transform::from_xyz(SPAWN.0, ground_y + 1.0, SPAWN.1),
    ));

    // Camera perched above and behind the stalker, with the Zone's haze.
    commands.spawn((
        PlayerCamera,
        Camera3d::default(),
        Transform::from_xyz(SPAWN.0, ground_y + 6.0, SPAWN.1 + 8.0)
            .looking_at(Vec3::new(SPAWN.0, ground_y + 1.0, SPAWN.1), Vec3::Y),
        DistanceFog {
            color: Color::srgb(0.62, 0.64, 0.58),
            falloff: FogFalloff::Linear {
                start: 90.0,
                end: 290.0,
            },
            ..default()
        },
    ));
}

fn despawn_player(mut commands: Commands, player_entities: PlayerEntities) {
    for entity in &player_entities {
        commands.entity(entity).despawn();
    }
}

/// WASD movement along the ground plane.
fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut players: Query<(&Player, &mut Transform)>,
) {
    let Ok((player, mut transform)) = players.single_mut() else {
        return;
    };

    let mut direction = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        direction -= Vec3::Z;
    }
    if keys.pressed(KeyCode::KeyS) {
        direction += Vec3::Z;
    }
    if keys.pressed(KeyCode::KeyA) {
        direction -= Vec3::X;
    }
    if keys.pressed(KeyCode::KeyD) {
        direction += Vec3::X;
    }

    let direction = direction.normalize_or_zero();
    transform.translation += direction * player.move_speed * time.delta_secs();
}

/// Keeps the camera above and behind the stalker.
fn follow_player(
    players: Query<&Transform, (With<Player>, Without<PlayerCamera>)>,
    mut cameras: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
) {
    let Ok(player_transform) = players.single() else {
        return;
    };
    let Ok(mut camera_transform) = cameras.single_mut() else {
        return;
    };

    let target = player_transform.translation + Vec3::new(0.0, 5.0, 8.0);
    camera_transform.translation = target;
    camera_transform.look_at(player_transform.translation, Vec3::Y);
}
