//! Player: the stalker, their camera, and movement controls.

mod animation;
mod camera;
pub(crate) mod vehicle;

use std::f32::consts::PI;

use bevy::camera::Exposure;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::AtmosphereEnvironmentMapLight;
use bevy::pbr::AtmosphereSettings;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

use crate::assets::MODELS;
use crate::states::{AppState, InGamePhase};
use crate::world::{Body, Collider, SPAWN, TERRAIN_HALF_SIZE, Terrain, push_out};

use animation::{
    Gait, StalkerModel, animate_stalker, attach_stalker_animator, load_stalker_animations,
    pause_stalker_animation, pose_stalker_body, resume_stalker_animation,
};
use camera::{LOOK_HEIGHT, OrbitCamera, follow_player, grab_cursor_while_orbiting, orbit_camera};
use vehicle::{Driver, Driving, Vehicle, drive_vehicle, interact_vehicle, pose_wheels};

pub struct PlayerPlugin;

/// Scales the 0.977-unit model (origin at its soles) to a 1.8 m stalker.
const MODEL_SCALE: f32 = 1.84;

/// Downward acceleration while airborne, in m/s².
const GRAVITY: f32 = 18.0;

/// How far inside the terrain edge the stalker must stay.
const EDGE_MARGIN: f32 = 2.0;

/// The stalker's body for collision, standing and lying prone.
const STANDING: Body = Body {
    radius: 0.35,
    height: 1.8,
};
const PRONE: Body = Body {
    radius: 0.35,
    height: 0.5,
};

/// The stalker. Movement speeds are data, not magic numbers in systems.
#[derive(Component)]
#[require(Locomotion, Visibility)]
pub struct Player {
    pub walk_speed: f32,
    pub run_speed: f32,
    pub crawl_speed: f32,
    /// Take-off speed of a jump, in m/s.
    pub jump_speed: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            walk_speed: 1.8,
            run_speed: 4.5,
            crawl_speed: 0.7,
            jump_speed: 5.0,
        }
    }
}

/// The stalker's vertical motion and current gait.
#[derive(Component, Default)]
pub struct Locomotion {
    pub vertical_speed: f32,
    pub grounded: bool,
    pub gait: Gait,
}

/// Sends the stalker back to base, as after dying in the Zone.
#[derive(Message)]
pub struct Respawn;

/// Tags the camera that follows the stalker.
#[derive(Component)]
#[require(OrbitCamera)]
pub struct PlayerCamera;

/// Everything spawned by [`spawn_player`] and torn down by [`despawn_player`].
type PlayerEntities<'w, 's> = Query<'w, 's, Entity, Or<(With<Player>, With<PlayerCamera>)>>;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Respawn>()
            .add_systems(Startup, load_stalker_animations)
            .add_systems(OnEnter(AppState::InGame), spawn_player)
            .add_systems(OnExit(AppState::InGame), despawn_player)
            .add_systems(
                Update,
                (
                    orbit_camera,
                    respawn_player,
                    interact_vehicle,
                    move_player,
                    drive_vehicle,
                    pose_wheels,
                    follow_player,
                    animate_stalker,
                    pose_stalker_body,
                )
                    .chain()
                    .run_if(in_state(InGamePhase::Running)),
            )
            .add_systems(OnEnter(InGamePhase::Paused), pause_stalker_animation)
            .add_systems(OnExit(InGamePhase::Paused), resume_stalker_animation)
            .add_systems(Update, grab_cursor_while_orbiting);
    }
}

/// Camera yaw at the start of a run: behind the stalker, looking north out
/// of the hangar door into the Zone.
const SPAWN_YAW: f32 = PI;

/// Where the stalker stands at the start of a run and after respawning:
/// inside the base by its door, facing out (model +Z) and away from the camera.
fn spawn_transform(terrain: &Terrain) -> Transform {
    let spawn = Vec3::new(SPAWN.0, terrain.height_at(SPAWN.0, SPAWN.1), SPAWN.1);
    Transform::from_translation(spawn)
}

fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>, terrain: Res<Terrain>) {
    // The stalker: rigged glTF model from assets/models/stalker.glb. Its
    // origin is at the soles of its feet.
    let start = spawn_transform(&terrain);
    let spawn = start.translation;
    let player = commands
        .spawn((
            Player::default(),
            Locomotion {
                grounded: true,
                ..default()
            },
            start,
        ))
        .id();
    commands
        .spawn((
            StalkerModel::new(player),
            WorldAssetRoot(asset_server.load(format!("{MODELS}/stalker.glb#Scene0"))),
            Transform::from_scale(Vec3::splat(MODEL_SCALE)),
            ChildOf(player),
        ))
        .observe(attach_stalker_animator);

    // Camera orbiting behind the stalker. The sky and haze come from the
    // world's atmosphere, which needs HDR at sunlight exposure.
    let orbit = OrbitCamera {
        yaw: SPAWN_YAW,
        ..default()
    };
    let focus = spawn + Vec3::Y * LOOK_HEIGHT;
    commands.spawn((
        PlayerCamera,
        Camera3d::default(),
        Transform::from_translation(focus + orbit.offset()).looking_at(focus, Vec3::Y),
        orbit,
        AtmosphereSettings::default(),
        AtmosphereEnvironmentMapLight::default(),
        Exposure { ev100: 13.0 },
        Tonemapping::AcesFitted,
        Bloom::NATURAL,
    ));
}

fn despawn_player(mut commands: Commands, player_entities: PlayerEntities) {
    for entity in &player_entities {
        commands.entity(entity).despawn();
    }
}

/// Puts the stalker back at the base, standing, with the camera behind.
fn respawn_player(
    mut commands: Commands,
    mut respawns: MessageReader<Respawn>,
    terrain: Res<Terrain>,
    mut players: Driver,
    mut vehicles: Query<&mut Vehicle>,
    mut cameras: Query<&mut OrbitCamera>,
) {
    if respawns.read().count() == 0 {
        return;
    }
    for (entity, mut transform, mut locomotion, mut visibility, driving) in &mut players {
        if let Some(driving) = driving {
            if let Ok(mut vehicle) = vehicles.get_mut(driving.0) {
                vehicle.park();
            }
            commands.entity(entity).remove::<Driving>();
            *visibility = Visibility::Inherited;
        }
        *transform = spawn_transform(&terrain);
        *locomotion = Locomotion {
            grounded: true,
            ..default()
        };
    }
    for mut orbit in &mut cameras {
        orbit.yaw = SPAWN_YAW;
    }
}

/// WASD movement relative to where the camera looks, Shift to run, Space to
/// jump, Ctrl to crawl. The stalker turns to face the direction of travel,
/// bumps into solid models, and stays on the terrain surface unless airborne.
fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    terrain: Res<Terrain>,
    colliders: Query<(&Collider, &GlobalTransform)>,
    cameras: Query<&OrbitCamera>,
    mut players: Query<(&Player, &mut Locomotion, &mut Transform), Without<Driving>>,
) {
    let Ok((player, mut locomotion, mut transform)) = players.single_mut() else {
        return;
    };
    let heading = cameras
        .single()
        .map_or(Quat::IDENTITY, OrbitCamera::heading);

    let mut input = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        input -= Vec3::Z;
    }
    if keys.pressed(KeyCode::KeyS) {
        input += Vec3::Z;
    }
    if keys.pressed(KeyCode::KeyA) {
        input -= Vec3::X;
    }
    if keys.pressed(KeyCode::KeyD) {
        input += Vec3::X;
    }

    let direction = heading * input.normalize_or_zero();

    // The model faces +Z at rotation 0; yaw so it faces the travel direction.
    if direction != Vec3::ZERO {
        let target = Quat::from_rotation_y(direction.x.atan2(direction.z));
        let blend = 1.0 - (-10.0 * time.delta_secs()).exp();
        transform.rotation = transform.rotation.slerp(target, blend);
    }

    let crawling =
        locomotion.grounded && keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let running = !crawling && keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let speed = if crawling {
        player.crawl_speed
    } else if running {
        player.run_speed
    } else {
        player.walk_speed
    };
    let step = direction * speed * time.delta_secs();
    let target = transform.translation + step;
    let body = if crawling { PRONE } else { STANDING };
    let clear = push_out(target.xz(), target.y, body, colliders);
    let ground = ground_position(&terrain, Vec3::new(clear.x, target.y, clear.y));

    if locomotion.grounded && !crawling && keys.just_pressed(KeyCode::Space) {
        locomotion.vertical_speed = player.jump_speed;
    }
    let (y, vertical_speed, grounded) = fall(
        transform.translation.y,
        locomotion.vertical_speed,
        locomotion.grounded,
        ground.y,
        time.delta_secs(),
    );
    transform.translation = Vec3::new(ground.x, y, ground.z);
    locomotion.vertical_speed = vertical_speed;
    locomotion.grounded = grounded;
    let moving = direction != Vec3::ZERO;
    locomotion.gait = match (grounded, crawling, moving, running) {
        (false, ..) => Gait::Jump,
        (true, true, false, _) => Gait::Prone,
        (true, true, true, _) => Gait::Crawl,
        (true, false, false, _) => Gait::Idle,
        (true, false, true, false) => Gait::Walk,
        (true, false, true, true) => Gait::Run,
    };
}

/// Advances height `y` with vertical speed `vy` over `dt` under gravity,
/// landing on `ground`. Returns the new height, speed, and whether grounded.
///
/// A grounded stalker without upward speed sticks to the ground, so walking
/// downhill never counts as falling.
fn fall(y: f32, vy: f32, grounded: bool, ground: f32, dt: f32) -> (f32, f32, bool) {
    if grounded && vy <= 0.0 {
        return (ground, 0.0, true);
    }
    let vy = vy - GRAVITY * dt;
    let y = y + vy * dt;
    if y <= ground {
        (ground, 0.0, true)
    } else {
        (y, vy, false)
    }
}

/// Keeps a point on the terrain: inside its edges and on its surface.
fn ground_position(terrain: &Terrain, position: Vec3) -> Vec3 {
    let limit = TERRAIN_HALF_SIZE - EDGE_MARGIN;
    let x = position.x.clamp(-limit, limit);
    let z = position.z.clamp(-limit, limit);
    Vec3::new(x, terrain.height_at(x, z), z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stalker_cannot_leave_the_terrain() {
        let terrain = Terrain::new();
        let grounded = ground_position(&terrain, Vec3::new(500.0, 0.0, -500.0));
        let limit = TERRAIN_HALF_SIZE - EDGE_MARGIN;
        assert_eq!((grounded.x, grounded.z), (limit, -limit));
    }

    #[test]
    fn walking_over_hills_keeps_stalker_and_camera_above_ground() {
        use std::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                50,
            )))
            .insert_resource(Terrain::new())
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, (move_player, follow_player).chain());

        // Camera swung round so W walks toward +X, over the hills and down
        // into the quarry.
        app.world_mut().spawn((
            PlayerCamera,
            Transform::default(),
            OrbitCamera {
                yaw: -std::f32::consts::FRAC_PI_2,
                ..default()
            },
        ));
        let player = app
            .world_mut()
            .spawn((
                Player::default(),
                Locomotion {
                    grounded: true,
                    ..default()
                },
                Transform::from_xyz(0.0, Terrain::new().height_at(0.0, 40.0), 40.0),
            ))
            .id();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);

        let terrain = Terrain::new();
        for _ in 0..400 {
            app.update();
            let feet = app
                .world()
                .get::<Transform>(player)
                .expect("player")
                .translation;
            assert!((feet.y - terrain.height_at(feet.x, feet.z)).abs() < 1e-4);
            let mut cameras = app
                .world_mut()
                .query_filtered::<&Transform, With<PlayerCamera>>();
            let eye = cameras.single(app.world()).expect("camera").translation;
            assert!(
                eye.y > terrain.height_at(eye.x, eye.z),
                "camera underground at {eye}"
            );
        }
        let feet = app
            .world()
            .get::<Transform>(player)
            .expect("player")
            .translation;
        assert!(feet.x > 60.0, "stalker should have run east, at {feet}");
        let locomotion = app.world().get::<Locomotion>(player).expect("locomotion");
        assert_eq!(locomotion.gait, Gait::Run);
    }

    #[test]
    fn jump_rises_then_lands_on_the_ground() {
        let dt = 1.0 / 60.0;
        let (mut y, mut vy, mut grounded) = (0.0, Player::default().jump_speed, false);
        let mut peak: f32 = 0.0;
        let mut frames = 0;
        while !grounded {
            (y, vy, grounded) = fall(y, vy, grounded, 0.0, dt);
            peak = peak.max(y);
            frames += 1;
            assert!(frames < 120, "never landed");
        }
        assert_eq!(y, 0.0);
        assert!((0.5..1.0).contains(&peak), "jump peak {peak} m");
    }

    #[test]
    fn walking_downhill_stays_grounded() {
        let (y, vy, grounded) = fall(10.0, 0.0, true, 9.5, 1.0 / 60.0);
        assert_eq!((y, vy, grounded), (9.5, 0.0, true));
    }

    /// An app that runs [`move_player`] at 20 Hz for a stalker at (0, 0)
    /// facing -Z, holding `keys`.
    fn moving_app(keys: &[KeyCode]) -> (App, Entity) {
        use std::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                50,
            )))
            .insert_resource(Terrain::new())
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, move_player);
        app.world_mut()
            .spawn((Transform::default(), OrbitCamera::default()));
        let player = app
            .world_mut()
            .spawn((
                Player::default(),
                Locomotion {
                    grounded: true,
                    ..default()
                },
                Transform::from_xyz(0.0, Terrain::new().height_at(0.0, 0.0), 0.0),
            ))
            .id();
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        for &key in keys {
            input.press(key);
        }
        (app, player)
    }

    fn feet(app: &App, player: Entity) -> Vec3 {
        app.world()
            .get::<Transform>(player)
            .expect("player")
            .translation
    }

    #[test]
    fn stalker_bumps_into_solid_models() {
        use crate::world::Pillar;

        const ROCK: &[Pillar] = &[Pillar::centered(0.4, 1.2)];
        let (mut app, player) = moving_app(&[KeyCode::KeyW]);
        let ground = Terrain::new().height_at(0.0, -5.0);
        let rock = Transform::from_xyz(0.0, ground + 0.6, -5.0);
        app.world_mut()
            .spawn((Collider(ROCK), rock, GlobalTransform::from(rock)));
        for _ in 0..100 {
            app.update();
            let distance = feet(&app, player).xz().distance(Vec2::new(0.0, -5.0));
            assert!(distance >= 0.75 - 1e-4, "inside the rock, {distance} m off");
        }
        assert!(feet(&app, player).z > -5.0, "walked through the rock");
    }

    #[test]
    fn ctrl_crawls_slowly_and_cannot_jump() {
        let (mut app, player) = moving_app(&[KeyCode::ControlLeft, KeyCode::KeyW, KeyCode::Space]);
        for _ in 0..40 {
            app.update();
        }
        let locomotion = app.world().get::<Locomotion>(player).expect("locomotion");
        assert_eq!(locomotion.gait, Gait::Crawl);
        assert!(locomotion.grounded);
        let travelled = feet(&app, player).xz().length();
        let crawl_speed = Player::default().crawl_speed;
        // The first update has no elapsed time.
        assert!(
            (travelled - crawl_speed * 39.0 * 0.05).abs() < 1e-3,
            "{travelled} m"
        );
    }

    #[test]
    fn respawn_returns_the_stalker_to_base() {
        let (mut app, player) = moving_app(&[]);
        app.add_message::<Respawn>()
            .add_systems(Update, respawn_player.before(move_player));
        app.world_mut()
            .get_mut::<Locomotion>(player)
            .expect("locomotion")
            .gait = Gait::Crawl;
        app.world_mut().write_message(Respawn);
        app.update();
        let at = feet(&app, player);
        let terrain = Terrain::new();
        assert_eq!(at, spawn_transform(&terrain).translation);
        let locomotion = app.world().get::<Locomotion>(player).expect("locomotion");
        assert_eq!((locomotion.gait, locomotion.grounded), (Gait::Idle, true));
    }
}
