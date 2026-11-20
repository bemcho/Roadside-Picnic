//! The stalkers' hangar, parked vehicle, and interior lighting.

use std::sync::LazyLock;

use bevy::light::{EnvironmentMapLight, LightProbe};
use bevy::prelude::*;

use crate::assets::BASE_HANGAR;
use crate::player::vehicle::{parked_vehicle, spawn_vehicle};

use super::WorldRoot;
use super::collision::{Collider, Pillar};
use super::terrain::{BASE, Terrain};

const MODEL_SCALE: f32 = 38.0;
const FLOOR_DATUM: f32 = 0.0217;
const DOOR_Z: f32 = 0.375;
const DOOR_HALF_WIDTH: f32 = 0.294;

/// Inner and outer steel faces measured from the GLB in model units.
static WALLS: LazyLock<Vec<Pillar>> = LazyLock::new(|| {
    let mut walls = Vec::new();
    let mut run = |start: Vec2, end: Vec2| {
        let length = start.distance(end) * MODEL_SCALE;
        let steps = (length / 0.38).ceil() as u32;
        for i in 0..=steps {
            let p = start.lerp(end, i as f32 / steps as f32) * MODEL_SCALE;
            walls.push(Pillar {
                x: p.x,
                z: p.y,
                radius: 0.32,
                bottom: -0.5,
                top: 11.0,
            });
        }
    };
    const INNER: [(f32, f32); 7] = [
        (0.302, -0.476),
        (0.323, -0.30),
        (0.350, -0.15),
        (0.375, 0.0),
        (0.421, 0.15),
        (0.434, 0.30),
        (0.425, DOOR_Z),
    ];
    const OUTER: [(f32, f32); 5] = [
        (0.402, -0.479),
        (0.418, -0.30),
        (0.425, 0.0),
        (0.444, 0.30),
        (0.445, DOOR_Z),
    ];
    for sign in [-1.0, 1.0] {
        for face in [&INNER[..], &OUTER[..]] {
            for &[a, b] in face.array_windows() {
                run(Vec2::new(sign * a.0, a.1), Vec2::new(sign * b.0, b.1));
            }
        }
        run(
            Vec2::new(sign * DOOR_HALF_WIDTH, DOOR_Z),
            Vec2::new(sign * 0.445, DOOR_Z),
        );
    }
    run(Vec2::new(-0.405, -0.476), Vec2::new(0.405, -0.476));
    walls
});

pub(super) fn spawn_base(
    mut commands: Commands,
    server: Res<AssetServer>,
    terrain: Res<Terrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let pad = Vec3::new(BASE.0, terrain.height_at(BASE.0, BASE.1), BASE.1);
    let hall = commands
        .spawn((
            WorldRoot,
            Transform::from_translation(pad),
            Visibility::default(),
            Collider(WALLS.as_slice()),
        ))
        .id();
    commands.spawn((
        WorldAssetRoot(server.load(GltfAssetLabel::Scene(0).from_asset(BASE_HANGAR))),
        Transform::from_xyz(0.0, -FLOOR_DATUM * MODEL_SCALE, 0.0)
            .with_scale(Vec3::splat(MODEL_SCALE)),
        ChildOf(hall),
    ));
    let vehicle = spawn_vehicle(
        &mut commands,
        &server,
        parked_vehicle(pad + Vec3::Y * 0.035),
        &mut meshes,
        &mut materials,
    );
    commands.entity(vehicle).insert(WorldRoot);
    spawn_hall_probe(&mut commands, hall, &mut images);
    spawn_lamps(&mut commands, hall, &mut meshes, &mut materials);
}

fn spawn_hall_probe(commands: &mut Commands, hall: Entity, images: &mut Assets<Image>) {
    commands.spawn((
        LightProbe {
            falloff: Vec3::new(0.04, 0.14, 0.08),
        },
        EnvironmentMapLight::hemispherical_gradient(
            images,
            Color::linear_rgb(0.79, 0.87, 1.0),
            Color::linear_rgb(0.62, 0.68, 0.76),
            Color::linear_rgb(0.25, 0.26, 0.30),
        )
        .with_intensity(1_750.0),
        Transform::from_xyz(0.0, 8.0, -2.0).with_scale(Vec3::new(29.0, 17.0, 33.0)),
        ChildOf(hall),
    ));
}

fn spawn_lamps(
    commands: &mut Commands,
    hall: Entity,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let steel = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.21, 0.22),
        metallic: 0.58,
        perceptual_roughness: 0.67,
        ..default()
    });
    let lens = materials.add(StandardMaterial {
        emissive: LinearRgba::rgb(1.0, 0.88, 0.72) * 42_000.0,
        ..default()
    });
    let housing = meshes.add(Cuboid::new(0.98, 0.14, 0.37));
    let panel = meshes.add(Cuboid::new(0.83, 0.024, 0.27));
    for (x, z, roof) in [
        (0.0, 9.0, 20.60),
        (-8.6, 6.0, 18.00),
        (8.6, 6.0, 18.00),
        (-8.2, -9.6, 14.13),
        (8.2, -9.6, 14.13),
        (0.0, -13.0, 15.88),
    ] {
        let y = roof * 0.68;
        let lamp = Vec3::new(x, y, z);
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.026, roof - y))),
            MeshMaterial3d(steel.clone()),
            Transform::from_xyz(x, (y + roof) * 0.5, z),
            ChildOf(hall),
        ));
        commands.spawn((
            Mesh3d(housing.clone()),
            MeshMaterial3d(steel.clone()),
            Transform::from_translation(lamp),
            ChildOf(hall),
        ));
        commands.spawn((
            Mesh3d(panel.clone()),
            MeshMaterial3d(lens.clone()),
            Transform::from_translation(lamp - Vec3::Y * 0.072),
            ChildOf(hall),
        ));
        commands.spawn((
            SpotLight {
                color: if z < 0.0 {
                    Color::srgb(1.0, 0.84, 0.67)
                } else {
                    Color::srgb(1.0, 0.95, 0.86)
                },
                intensity: 2.45e5 * y * y,
                range: y + 9.0,
                radius: 0.34,
                outer_angle: 0.58,
                inner_angle: 0.25,
                shadow_maps_enabled: z == 9.0,
                ..default()
            },
            Transform::from_translation(lamp - Vec3::Y * 0.12).looking_to(Vec3::NEG_Y, Vec3::Z),
            ChildOf(hall),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::vehicle::{blocked, vehicle_distance};
    use crate::world::{Body, SPAWN, push_out};

    #[test]
    fn imported_walls_block_both_approaches_and_leave_the_door_open() {
        let walls = Collider(WALLS.as_slice());
        let at = GlobalTransform::IDENTITY;
        for height in [0.5, 1.8] {
            let body = Body {
                radius: 0.35,
                height,
            };
            let push = |p: Vec2| push_out(p * MODEL_SCALE, 0.0, body, [(&walls, &at)]);
            for p in [
                Vec2::new(0.375, 0.0),
                Vec2::new(-0.425, 0.0),
                Vec2::new(0.0, -0.476),
                Vec2::new(0.32, DOOR_Z),
            ] {
                assert_ne!(push(p), p * MODEL_SCALE, "wall clear at {p}");
            }
            for p in [Vec2::new(0.0, DOOR_Z), Vec2::new(0.25, DOOR_Z)] {
                assert_eq!(push(p), p * MODEL_SCALE, "door blocked at {p}");
            }
        }
    }

    #[test]
    fn player_spawn_and_vehicle_route_clear_the_imported_model() {
        let walls = Collider(WALLS.as_slice());
        let hall = GlobalTransform::IDENTITY;
        let mut vehicle = parked_vehicle(Vec3::ZERO);
        let spawn = Vec2::from(SPAWN) - Vec2::from(BASE);
        assert!(DOOR_Z * MODEL_SCALE - spawn.y > 5.0);
        let start = Vec3::new(spawn.x, 0.0, spawn.y);
        assert!(vehicle_distance(&vehicle, start) > 0.35);
        assert_eq!(
            push_out(
                spawn,
                0.0,
                Body {
                    radius: 0.35,
                    height: 1.8
                },
                [(&walls, &hall)]
            ),
            spawn
        );
        let forward = vehicle.rotation * Vec3::Z;
        while vehicle.translation.z <= MODEL_SCALE * 0.4932 + 3.0 {
            assert!(
                !blocked(&vehicle, [(&walls, &hall)].into_iter()),
                "blocked at {}",
                vehicle.translation
            );
            vehicle.translation += forward * 0.2;
        }
    }
}
