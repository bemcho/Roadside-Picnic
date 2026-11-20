//! Player: the stalker, their camera, and movement controls.

use std::f32::consts::PI;

use bevy::prelude::*;

use crate::assets::MODELS;
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

fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>, terrain: Res<Terrain>) {
    let ground_y = terrain.height_at(SPAWN.0, SPAWN.1);

    // The stalker: glTF model from assets/models/stalker.glb, facing away
    // from the camera at spawn.
    commands.spawn((
        Player { move_speed: 5.0 },
        WorldAssetRoot(asset_server.load(format!("{MODELS}/stalker.glb#Scene0"))),
        Transform::from_xyz(SPAWN.0, ground_y, SPAWN.1).with_rotation(Quat::from_rotation_y(PI)),
    ));

    // Camera perched above and behind the stalker, with the Zone's haze.
    commands.spawn((
        PlayerCamera,
        Camera3d::default(),
        Transform::from_xyz(SPAWN.0, ground_y + 4.0, SPAWN.1 + 9.0)
            .looking_at(Vec3::new(SPAWN.0, ground_y + 1.2, SPAWN.1), Vec3::Y),
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

/// WASD movement along the ground plane. The stalker turns to face the
/// direction of travel.
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

    // The model faces +Z at rotation 0; yaw so it faces the travel direction.
    if direction != Vec3::ZERO {
        let target = Quat::from_rotation_y(direction.x.atan2(direction.z));
        let blend = 1.0 - (-10.0 * time.delta_secs()).exp();
        transform.rotation = transform.rotation.slerp(target, blend);
    }

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

    let target = player_transform.translation + Vec3::new(0.0, 4.0, 9.0);
    camera_transform.translation = target;
    camera_transform.look_at(player_transform.translation + Vec3::Y * 1.2, Vec3::Y);
}
