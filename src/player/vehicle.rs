//! Driving, collision, and wheel posing for the armored vehicle.

use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

use crate::assets;
use crate::world::{Collider, Pillar, STEP_HEIGHT, TERRAIN_HALF_SIZE, Terrain, push_out};

use super::{Locomotion, Player, STANDING, camera::OrbitCamera};

const MODEL_SCALE: f32 = 5.2;
const WHEEL_RADIUS: f32 = 0.1008 * MODEL_SCALE;
const HALF_EXTENTS: Vec2 = Vec2::new(0.283 * MODEL_SCALE, 0.5 * MODEL_SCALE);
const HULL_HEIGHT: f32 = 0.47 * MODEL_SCALE;
const GROUND_CLEARANCE: f32 = 0.02;
const ENTRY_REACH: f32 = 1.65;
const FORWARD: Vec3 = Vec3::Z;
#[allow(clippy::approx_constant, reason = "Measured model coordinates")]
const COLLISION: &[Pillar] = include!("../world/pillars/armored_vehicle.rs");

#[derive(Component, Default)]
pub(crate) struct Vehicle {
    speed: f32,
    roll: f32,
    steer: f32,
}

#[derive(Component)]
pub(crate) struct Driving(pub(super) Entity);

pub(super) type Driver<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Transform,
        &'static mut Locomotion,
        &'static mut Visibility,
        Option<&'static Driving>,
    ),
    With<Player>,
>;

#[derive(Component)]
pub(super) struct Wheel {
    vehicle: Entity,
    front: bool,
}

pub(crate) fn parked_vehicle(floor: Vec3) -> Transform {
    Transform::from_translation(floor + Vec3::new(8.5, 0.0, 8.5))
        .with_rotation(Quat::from_rotation_y(-0.59))
}

impl Vehicle {
    pub(super) fn park(&mut self) {
        self.speed = 0.0;
        self.steer = 0.0;
    }
}

pub(crate) fn spawn_vehicle(
    commands: &mut Commands,
    assets: &AssetServer,
    transform: Transform,
) -> Entity {
    commands
        .spawn((
            Vehicle::default(),
            Collider(COLLISION),
            transform.with_scale(Vec3::splat(MODEL_SCALE)),
            WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(assets::BASE_VEHICLE))),
        ))
        .observe(attach_wheels)
        .id()
}

fn attach_wheels(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    for entity in children.iter_descendants(ready.entity) {
        let Ok(name) = names.get(entity) else {
            continue;
        };
        let front = match name.as_str() {
            "WheelFrontLeft" | "WheelFrontRight" => true,
            "WheelRearLeft" | "WheelRearRight" => false,
            _ => continue,
        };
        commands.entity(entity).insert(Wheel {
            vehicle: ready.entity,
            front,
        });
    }
}

pub(crate) fn vehicle_distance(at: &Transform, point: Vec3) -> f32 {
    let offset = at.rotation.inverse() * (point - at.translation);
    if offset.y.abs() >= 1.2 {
        return f32::INFINITY;
    }
    (offset.xz().abs() - HALF_EXTENTS).max(Vec2::ZERO).length()
}

pub(crate) fn can_enter_vehicle(locomotion: &Locomotion, at: &Transform, point: Vec3) -> bool {
    locomotion.grounded && vehicle_distance(at, point) <= ENTRY_REACH
}

pub(super) fn interact_vehicle(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    terrain: Res<Terrain>,
    colliders: Query<(Entity, &Collider, &GlobalTransform)>,
    mut vehicles: Query<(Entity, &mut Vehicle, &Transform), Without<Player>>,
    mut players: Driver,
    mut cameras: Query<&mut OrbitCamera>,
) {
    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }
    let Ok((player, mut feet, mut locomotion, mut visibility, driving)) = players.single_mut()
    else {
        return;
    };
    if let Some(driving) = driving {
        let Ok((entity, mut vehicle, at)) = vehicles.get_mut(driving.0) else {
            return;
        };
        if let Some(exit) = exit_position(at, &terrain, |point| {
            push_out(
                point.xz(),
                point.y,
                STANDING,
                other_colliders(&colliders, entity),
            ) == point.xz()
        }) {
            vehicle.park();
            feet.translation = exit;
            *locomotion = grounded_idle();
            *visibility = Visibility::Inherited;
            commands.entity(player).remove::<Driving>();
        }
    } else if let Some((entity, _, at)) = vehicles
        .iter()
        .filter(|(_, _, at)| can_enter_vehicle(&locomotion, at, feet.translation))
        .min_by(|(_, _, a), (_, _, b)| {
            vehicle_distance(a, feet.translation).total_cmp(&vehicle_distance(b, feet.translation))
        })
    {
        *locomotion = grounded_idle();
        *visibility = Visibility::Hidden;
        commands.entity(player).insert(Driving(entity));
        if let Ok(mut orbit) = cameras.single_mut() {
            let forward = at.rotation * FORWARD;
            orbit.yaw = (-forward.x).atan2(-forward.z);
        }
    }
}

fn grounded_idle() -> Locomotion {
    Locomotion {
        grounded: true,
        ..default()
    }
}

fn exit_position(at: &Transform, terrain: &Terrain, clear: impl Fn(Vec3) -> bool) -> Option<Vec3> {
    let (side, end) = (HALF_EXTENTS.x, HALF_EXTENTS.y);
    [
        Vec3::X * side,
        Vec3::NEG_X * side,
        -FORWARD * end,
        FORWARD * end,
    ]
    .into_iter()
    .find_map(|edge| {
        let sample = |reach: f32| {
            let mut point = at.translation + at.rotation * (edge + edge.normalize() * reach);
            point.y = terrain.height_at(point.x, point.z);
            point
        };
        let exit = sample(0.9);
        (exit.xz().abs().max_element() < TERRAIN_HALF_SIZE - super::EDGE_MARGIN
            && (exit.y - at.translation.y).abs() <= 1.2
            && clear(sample(STANDING.radius))
            && clear(exit))
        .then_some(exit)
    })
}

pub(super) fn drive_vehicle(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    terrain: Res<Terrain>,
    colliders: Query<(Entity, &Collider, &GlobalTransform)>,
    mut vehicles: Query<(&mut Vehicle, &mut Transform), Without<Player>>,
    mut players: Query<(&Driving, &mut Transform), With<Player>>,
    mut cameras: Query<&mut OrbitCamera>,
) {
    let Ok((driving, mut player)) = players.single_mut() else {
        return;
    };
    let Ok((mut vehicle, mut at)) = vehicles.get_mut(driving.0) else {
        return;
    };
    let brake = keys.pressed(KeyCode::Space);
    let throttle = axis(&keys, KeyCode::KeyW, KeyCode::KeyS);
    let steering = axis(&keys, KeyCode::KeyA, KeyCode::KeyD);
    let target = if brake {
        0.0
    } else {
        throttle * if throttle >= 0.0 { 10.0 } else { 4.0 }
    };
    let dt = time.delta_secs();
    let steps = (dt / 0.04).ceil().max(1.0) as u32;
    let dt = dt / steps as f32;
    let mut turned = 0.0;
    for _ in 0..steps {
        let acceleration = if brake {
            18.0
        } else if throttle == 0.0 {
            3.5
        } else {
            6.0
        };
        vehicle.speed += (target - vehicle.speed).clamp(-acceleration * dt, acceleration * dt);
        vehicle.steer += (steering * 0.48 - vehicle.steer).clamp(-3.0 * dt, 3.0 * dt);
        let distance = vehicle.speed * dt;
        if distance == 0.0 {
            continue;
        }
        let mut next = *at;
        let turn = vehicle.steer * distance / 2.8;
        next.rotate_y(turn);
        next.translation += next.rotation * FORWARD * distance;
        next.translation.y =
            terrain.height_at(next.translation.x, next.translation.z) + GROUND_CLEARANCE;
        if next.translation.xz().abs().max_element() > TERRAIN_HALF_SIZE - HALF_EXTENTS.length()
            || blocked(&next, other_colliders(&colliders, driving.0))
        {
            vehicle.speed = 0.0;
            break;
        }
        vehicle.roll = (vehicle.roll + distance / WHEEL_RADIUS).rem_euclid(TAU);
        turned += turn;
        *at = next;
    }
    player.translation = at.translation;
    if let Ok(mut orbit) = cameras.single_mut() {
        orbit.yaw += turned;
    }
}

fn axis(keys: &ButtonInput<KeyCode>, positive: KeyCode, negative: KeyCode) -> f32 {
    f32::from(keys.pressed(positive)) - f32::from(keys.pressed(negative))
}

fn other_colliders<'a>(
    colliders: &'a Query<(Entity, &Collider, &GlobalTransform)>,
    vehicle: Entity,
) -> impl Iterator<Item = (&'a Collider, &'a GlobalTransform)> {
    colliders
        .iter()
        .filter(move |(entity, _, _)| *entity != vehicle)
        .map(|(_, collider, transform)| (collider, transform))
}

pub(crate) fn blocked<'a>(
    at: &Transform,
    colliders: impl Iterator<Item = (&'a Collider, &'a GlobalTransform)>,
) -> bool {
    let world_to_vehicle = at.rotation.inverse();
    colliders.into_iter().any(|(collider, transform)| {
        let (scale, _, origin) = transform.to_scale_rotation_translation();
        collider.0.iter().any(|pillar| {
            let top = origin.y + pillar.top * scale.y;
            let bottom = origin.y + pillar.bottom * scale.y;
            if top <= at.translation.y + STEP_HEIGHT || bottom >= at.translation.y + HULL_HEIGHT {
                return false;
            }
            let center = transform.transform_point(Vec3::new(pillar.x, 0.0, pillar.z));
            let local = (world_to_vehicle * (center - at.translation)).xz();
            (local.abs() - HALF_EXTENTS)
                .max(Vec2::ZERO)
                .length_squared()
                < (pillar.radius * scale.x).powi(2)
        })
    })
}

pub(super) fn pose_wheels(vehicles: Query<&Vehicle>, mut wheels: Query<(&Wheel, &mut Transform)>) {
    for (wheel, mut transform) in &mut wheels {
        let Ok(vehicle) = vehicles.get(wheel.vehicle) else {
            continue;
        };
        let steer = if wheel.front { vehicle.steer } else { 0.0 };
        transform.rotation = Quat::from_rotation_y(steer) * Quat::from_rotation_x(vehicle.roll);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    #[test]
    fn enters_drives_and_exits_with_one_press_each() {
        let flat = Vec2::new(40.0, -128.0);
        let terrain = Terrain::new();
        let floor = terrain.height_at(flat.x, flat.y);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                50,
            )))
            .insert_resource(terrain)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(
                Update,
                (interact_vehicle, drive_vehicle, pose_wheels).chain(),
            );
        let original = Transform::from_xyz(flat.x, floor + GROUND_CLEARANCE, flat.y);
        let vehicle = app.world_mut().spawn((Vehicle::default(), original)).id();
        let wheel = app
            .world_mut()
            .spawn((
                Transform::default(),
                Wheel {
                    vehicle,
                    front: true,
                },
            ))
            .id();
        let player = app
            .world_mut()
            .spawn((
                Player::default(),
                grounded_idle(),
                Transform::from_xyz(flat.x + 10.0, floor, flat.y),
                Visibility::Inherited,
            ))
            .id();

        press(&mut app, KeyCode::KeyE);
        app.update();
        assert!(app.world().get::<Driving>(player).is_none());
        release(&mut app, KeyCode::KeyE);
        app.world_mut()
            .get_mut::<Transform>(player)
            .expect("player")
            .translation
            .x = flat.x + HALF_EXTENTS.x + 0.5;

        press(&mut app, KeyCode::KeyE);
        app.update();
        assert_eq!(
            app.world().get::<Driving>(player).map(|d| d.0),
            Some(vehicle)
        );
        assert_eq!(
            app.world().get::<Visibility>(player),
            Some(&Visibility::Hidden)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear_just_pressed(KeyCode::KeyE);
        press(&mut app, KeyCode::KeyW);
        press(&mut app, KeyCode::KeyA);
        for _ in 0..15 {
            app.update();
        }
        let driven = *app.world().get::<Transform>(vehicle).expect("vehicle");
        assert!(driven.translation.z > original.translation.z + 1.0);
        assert!(driven.translation.x > original.translation.x);
        assert_ne!(
            app.world().get::<Transform>(wheel).expect("wheel").rotation,
            Quat::IDENTITY
        );
        assert_eq!(
            app.world()
                .get::<Transform>(player)
                .expect("player")
                .translation,
            driven.translation
        );
        release(&mut app, KeyCode::KeyW);
        release(&mut app, KeyCode::KeyA);
        release(&mut app, KeyCode::KeyE);
        press(&mut app, KeyCode::KeyE);
        app.update();
        assert!(app.world().get::<Driving>(player).is_none());
        assert_eq!(
            app.world().get::<Visibility>(player),
            Some(&Visibility::Inherited)
        );
        let feet = app
            .world()
            .get::<Transform>(player)
            .expect("player")
            .translation;
        assert!(vehicle_distance(&driven, feet) >= STANDING.radius);
        assert_eq!(app.world().get::<Transform>(vehicle), Some(&driven));
    }

    #[test]
    fn vehicle_stops_at_obstacles_but_clears_their_high_and_low_edges() {
        const SHAPE: &[Pillar] = &[Pillar {
            top: 1.0,
            ..Pillar::centered(0.35, 0.0)
        }];
        let obstacle = Collider(SHAPE);
        let at = Transform::default().with_rotation(Quat::from_rotation_y(0.5));
        let forward = at.rotation * FORWARD;
        let placed = |y, z| GlobalTransform::from_translation(forward * z + Vec3::Y * y);
        assert!(blocked(
            &at,
            [(&obstacle, &placed(0.0, HALF_EXTENTS.y + 0.2))].into_iter()
        ));
        for (height, distance) in [(-0.7, 0.0), (HULL_HEIGHT + 0.1, 0.0), (0.0, 3.2)] {
            assert!(!blocked(
                &at,
                [(&obstacle, &placed(height, distance))].into_iter()
            ));
        }
    }

    #[test]
    fn a_blocked_driver_side_exits_on_the_other_side() {
        let terrain = Terrain::new();
        let y = terrain.height_at(40.0, -128.0);
        let at = Transform::from_xyz(40.0, y, -128.0);
        let exit = exit_position(&at, &terrain, |point| point.x < at.translation.x);
        assert!(exit.is_some_and(|point| point.x < at.translation.x - HALF_EXTENTS.x));
        assert!(exit_position(&at, &terrain, |_| false).is_none());
    }

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
    }

    fn release(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(key);
    }
}
