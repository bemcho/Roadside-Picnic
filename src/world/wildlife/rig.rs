use std::f32::consts::{PI, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::DynamicSkinnedMeshBounds;
use bevy::light::NotShadowCaster;
use bevy::mesh::{
    PrimitiveTopology, VertexAttributeValues,
    skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
};
use bevy::prelude::*;

use super::movement::{Wildlife, smooth};
use super::{Kind, Terrain, detail_range};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::wildlife::Route;

    fn route() -> Route {
        Route {
            center: Vec2::new(0.0, -70.0),
            radius: Vec2::splat(1.0),
            phase: 0.0,
            speed: 0.2,
        }
    }

    #[test]
    fn grounded_steps_plant_one_foot_and_lift_the_other() {
        let terrain = Terrain::new();
        let mut motion = Wildlife::new(Kind::Heron, route(), None);
        motion.pace = 1.0;
        motion.gait = 0.22;
        let at = motion.initial_pose(&terrain);
        let feet: Vec<_> = bones(Kind::Heron)
            .into_iter()
            .filter_map(|bone| {
                let Bone::Foot { tip, .. } = bone else {
                    return None;
                };
                let foot = bone_pose(bone, &motion, &at, Transform::IDENTITY, &terrain);
                let posed = at.transform_point(foot.translation);
                let clearance = posed.y - terrain.mesh_height_at(posed.xz()) - tip.y * at.scale.y;
                Some((clearance, foot.translation.z - tip.z))
            })
            .collect();
        assert_eq!(feet.len(), 2);
        assert!((feet[0].0 - 0.012).abs() < 0.001, "stance foot floats");
        assert!(feet[1].0 > 0.07, "swing foot drags");
        assert!((feet[0].1 - feet[1].1).abs() > 0.015);
    }

    #[test]
    fn mesh_wings_move_in_sync_without_flapping_the_whole_model() {
        let terrain = Terrain::new();
        for kind in [Kind::Hawk, Kind::Skyglider] {
            let mut motion = Wildlife::new(kind, route(), None);
            motion.air = 1.0;
            let at = motion.initial_pose(&terrain);
            let angles = |motion: &Wildlife| {
                bones(kind)
                    .into_iter()
                    .filter_map(|bone| {
                        let Bone::Wing { side, .. } = bone else {
                            return None;
                        };
                        let rotation =
                            bone_pose(bone, motion, &at, Transform::IDENTITY, &terrain).rotation;
                        Some(rotation.to_euler(EulerRot::XYZ).2 * side)
                    })
                    .collect::<Vec<_>>()
            };
            let before = angles(&motion);
            motion.age += 0.10;
            let after = angles(&motion);
            assert_eq!(before.len(), 2);
            let changes = [after[0] - before[0], after[1] - before[1]];
            assert!(changes[0].abs() > 0.10 && (changes[0] - changes[1]).abs() < 0.001);
        }
    }
}

#[derive(Clone, Copy)]
enum Bone {
    Body,
    Foot {
        tip: Vec3,
        hip: Vec3,
        phase: f32,
        radius: f32,
    },
    Head(Vec3),
    Wing {
        shoulder: Vec3,
        side: f32,
        open: f32,
    },
}

impl Bone {
    fn pivot(self) -> Vec3 {
        match self {
            Self::Body => Vec3::ZERO,
            Self::Foot { tip, .. } => tip,
            Self::Head(neck) => neck,
            Self::Wing { shoulder, .. } => shoulder,
        }
    }

    fn influence(self, point: Vec3) -> f32 {
        match self {
            Self::Body => 0.0,
            Self::Foot {
                tip, hip, radius, ..
            } => {
                let leg = hip - tip;
                let u = (point.y - tip.y) / leg.y;
                let at = tip + leg * u.clamp(0.0, 1.0);
                let reach = 1.0 - smooth((point.xz().distance(at.xz()) / radius - 0.52) / 0.48);
                reach * (1.0 - smooth((u - 0.24) / 0.76))
            }
            Self::Head(neck) => {
                smooth((point.z - neck.z) / 0.14) * smooth((point.y - neck.y + 0.08) / 0.13)
            }
            Self::Wing { shoulder, side, .. } => {
                let wide = side * (point.x - shoulder.x);
                smooth(wide / 0.18) * (1.0 - smooth((point.z - 0.24 - wide) / 0.13))
            }
        }
    }
}

fn foot(tip: [f32; 3], hip: [f32; 3], phase: f32, radius: f32) -> Bone {
    Bone::Foot {
        tip: Vec3::from_array(tip),
        hip: Vec3::from_array(hip),
        phase,
        radius,
    }
}

fn bones(kind: Kind) -> Vec<Bone> {
    let mut bones = vec![Bone::Body];
    match kind {
        Kind::Frostwolf => {
            for side in [-1.0, 1.0] {
                let phase = if side < 0.0 { 0.0 } else { 0.5 };
                bones.push(foot(
                    [side * 0.088, 0.014, 0.418],
                    [side * 0.078, 0.225, 0.273],
                    phase,
                    0.105,
                ));
                bones.push(foot(
                    [side * 0.104, 0.017, -0.126],
                    [side * 0.086, 0.216, 0.017],
                    0.75 - phase,
                    0.105,
                ));
            }
            bones.push(Bone::Head(Vec3::new(0.0, 0.17, 0.176)));
        }
        Kind::Hexadeer => {
            // The sculpture has eight distinct hoof islands, not a symmetric stock quadruped.
            bones.extend([
                foot([-0.182, 0.024, 0.397], [-0.100, 0.51, 0.199], 0.00, 0.093),
                foot([0.064, 0.016, 0.335], [0.077, 0.51, 0.220], 0.53, 0.081),
                foot([-0.108, 0.022, 0.105], [-0.020, 0.49, 0.097], 0.74, 0.081),
                foot([0.184, 0.031, 0.152], [0.133, 0.49, 0.148], 0.24, 0.090),
                foot([-0.073, 0.018, -0.054], [-0.047, 0.48, -0.035], 0.45, 0.078),
                foot([0.145, 0.017, 0.017], [0.113, 0.47, -0.095], 0.91, 0.075),
                foot([-0.022, 0.015, -0.231], [-0.062, 0.49, -0.241], 0.60, 0.084),
                foot([0.041, 0.024, -0.374], [0.064, 0.49, -0.292], 0.10, 0.087),
                Bone::Head(Vec3::new(0.0, 0.64, 0.16)),
            ]);
        }
        Kind::Chameleon => bones.push(Bone::Head(Vec3::new(0.0, 0.60, 0.19))),
        Kind::Hawk | Kind::Skyglider => {
            let (left, right, y, z) = if kind == Kind::Hawk {
                (-0.075, 0.215, 0.51, 0.065)
            } else {
                (-0.258, 0.035, 0.31, -0.04)
            };
            bones.extend([
                Bone::Wing {
                    shoulder: Vec3::new(left, y, z),
                    side: -1.0,
                    open: 0.12,
                },
                Bone::Wing {
                    shoulder: Vec3::new(right, y, z),
                    side: 1.0,
                    open: if kind == Kind::Hawk { 0.86 } else { 0.0 },
                },
            ]);
        }
        Kind::Heron | Kind::Owl | Kind::Raven | Kind::Sparrow => {
            let (x, y, z, hip_y, hip_z, radius) = match kind {
                Kind::Heron => (0.127, 0.012, -0.061, 0.41, -0.087, 0.101),
                Kind::Owl => (0.123, 0.018, 0.067, 0.27, 0.030, 0.139),
                Kind::Raven => (0.11, 0.018, 0.093, 0.34, 0.023, 0.107),
                _ => (0.143, 0.025, 0.247, 0.33, 0.098, 0.126),
            };
            bones.extend([-1.0, 1.0].map(|side| {
                foot(
                    [side * x, y, z],
                    [side * x * 0.72, hip_y, hip_z],
                    0.25 + 0.25 * side,
                    radius,
                )
            }));
        }
    }
    bones
}

pub(super) struct Rig {
    mesh: Handle<Mesh>,
    inverse: Handle<SkinnedMeshInverseBindposes>,
    bones: Vec<Bone>,
}

impl Rig {
    pub(super) fn build(
        kind: Kind,
        source: Mesh,
        meshes: &mut Assets<Mesh>,
        binds: &mut Assets<SkinnedMeshInverseBindposes>,
    ) -> Self {
        let (_, _, yaw) = kind.model();
        let mut mesh = source.transformed_by(Transform::from_rotation(Quat::from_rotation_y(yaw)));
        let bones = bones(kind);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            unreachable!("wildlife GLBs have float POSITION attributes");
        };
        let (joints, weights): (Vec<_>, Vec<_>) = positions
            .iter()
            .map(|&at| {
                let mut active = [(0.0_f32, 0_u16); 2];
                for (i, &bone) in bones.iter().enumerate().skip(1) {
                    let w = bone.influence(Vec3::from_array(at));
                    if w > active[0].0 {
                        active[1] = active[0];
                        active[0] = (w, i as u16);
                    } else if w > active[1].0 {
                        active[1] = (w, i as u16);
                    }
                }
                // Competing limbs should not drag each other's toes, while the hips fade into the body.
                let (strength, a) = active[0];
                let (secondary, b) = active[1];
                let bleed = 0.35 * secondary * (1.0 - strength);
                ([0, a, b, 0], [1.0 - strength - bleed, strength, bleed, 0.0])
            })
            .unzip();
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(joints),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, weights);
        mesh = mesh
            .with_generated_skinned_mesh_bounds()
            .expect("wildlife has skin weights");
        let matrices: Vec<_> = bones
            .iter()
            .map(|bone| Mat4::from_translation(-bone.pivot()))
            .collect();
        Self {
            mesh: meshes.add(mesh),
            inverse: binds.add(matrices),
            bones,
        }
    }

    pub(super) fn spawn(
        &self,
        creature: &mut EntityCommands,
        motion: &Wildlife,
        material: &Handle<StandardMaterial>,
        distance: f32,
    ) -> Articulation {
        let mut joints = Vec::with_capacity(self.bones.len());
        // Joints and mesh share the same model-space carrier, so terrain motion precedes skinning.
        let mut root_entity = Entity::PLACEHOLDER;
        creature.with_children(|parts| {
            let mut root = parts.spawn((VisualPart, carrier(motion), Visibility::default()));
            root_entity = root.id();
            root.with_children(|skeleton| {
                for &bone in &self.bones {
                    let entity = skeleton
                        .spawn((VisualPart, Transform::from_translation(bone.pivot())))
                        .id();
                    joints.push((bone, entity));
                }
            });
            root.insert((
                Mesh3d(self.mesh.clone()),
                MeshMaterial3d(material.clone()),
                SkinnedMesh {
                    inverse_bindposes: self.inverse.clone(),
                    joints: joints.iter().map(|(_, entity)| *entity).collect(),
                },
                DynamicSkinnedMeshBounds,
                detail_range(distance, 20.0),
                NotShadowCaster,
            ));
        });
        Articulation {
            carrier: root_entity,
            joints,
            feathers: None,
            distance,
        }
    }
}

#[derive(Component)]
pub(in crate::world) struct VisualPart;

#[derive(Component)]
pub(in crate::world) struct Articulation {
    carrier: Entity,
    joints: Vec<(Bone, Entity)>,
    feathers: Option<[Entity; 2]>,
    distance: f32,
}

impl Articulation {
    pub(super) fn spawn_feathers(
        &mut self,
        creature: &mut EntityCommands,
        wings: &[Handle<Mesh>; 2],
        paint: &Handle<StandardMaterial>,
    ) {
        let mut entities = [Entity::PLACEHOLDER; 2];
        creature.with_children(|parts| {
            for (index, mesh) in wings.iter().enumerate() {
                entities[index] = parts
                    .spawn((
                        VisualPart,
                        Transform::IDENTITY,
                        Visibility::Hidden,
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(paint.clone()),
                        detail_range(self.distance, 20.0),
                        NotShadowCaster,
                    ))
                    .id();
            }
        });
        self.feathers = Some(entities);
    }
}

pub(super) fn wing_mesh(side: f32) -> Mesh {
    let shape = [
        (0.0, 0.0, 0.15),
        (0.23, 0.022, 0.22),
        (0.48, 0.025, 0.18),
        (0.67, -0.01, 0.065),
        (0.91, -0.055, -0.15),
        (0.73, -0.044, -0.14),
        (0.82, -0.058, -0.235),
        (0.63, -0.04, -0.204),
        (0.69, -0.05, -0.276),
        (0.46, -0.022, -0.239),
        (0.51, -0.031, -0.296),
        (0.18, -0.012, -0.217),
        (0.0, 0.0, -0.16),
    ]
    .map(|(x, y, z)| Vec3::new(x * side, y, z));
    let center = Vec3::new(side * 0.29, 0.024, -0.025);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    for (i, a) in shape.iter().enumerate() {
        let b = shape[(i + 1) % shape.len()];
        let (a, b) = if side > 0.0 { (*a, b) } else { (b, *a) };
        positions.extend([center.to_array(), a.to_array(), b.to_array()]);
        normals.extend([((a - center).cross(b - center).normalize()).to_array(); 3]);
        let outer = if i % 2 == 0 { 0.72 } else { 0.90 };
        colors.extend([
            [0.96, 0.96, 0.96, 1.0],
            [outer, outer, outer, 1.0],
            [0.8, 0.8, 0.8, 1.0],
        ]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
}

pub(super) fn stride(kind: Kind, run: f32) -> f32 {
    match kind {
        Kind::Frostwolf => 0.16 + 0.23 * run,
        Kind::Hexadeer => 0.19 + 0.28 * run,
        Kind::Heron => 0.24,
        Kind::Owl => 0.14,
        _ => 0.19,
    }
}

fn step_cycle(t: f32) -> (f32, f32) {
    let t = t.rem_euclid(1.0);
    if t < 0.59 {
        (0.5 - t / 0.59, 0.0)
    } else {
        let swing = (t - 0.59) / 0.41;
        (-0.5 + smooth(swing), (swing * PI).sin())
    }
}

fn carrier(motion: &Wildlife) -> Transform {
    let (pitch, offset) = match motion.kind {
        Kind::Hawk => (0.54, 0.28),
        Kind::Raven | Kind::Sparrow => (0.81, 0.33),
        Kind::Skyglider => (0.05, 0.24),
        _ => (0.0, 0.0),
    };
    let flap = (motion.age * 8.0 + motion.phase()).sin();
    let pitch = (pitch + flap * 0.04) * motion.air - motion.lunge * 0.10
        + (motion.gait * TAU).sin() * motion.run * 0.035;
    let rotation = Quat::from_rotation_x(pitch);
    let lift = motion.lunge * 0.085;
    Transform::from_translation(rotation * (Vec3::NEG_Y * offset * motion.air) + Vec3::Y * lift)
        .with_rotation(rotation)
}

fn wingbeat(motion: &Wildlife) -> f32 {
    let (rate, glide) = match motion.kind {
        Kind::Skyglider => (2.1, 0.68),
        Kind::Hawk => (2.7, 0.52),
        Kind::Raven => (4.8, 0.28),
        _ => (8.2, 0.15),
    };
    let t = motion.age + motion.phase();
    let flutter = (t * rate * TAU).sin() - (t * rate * TAU * 2.0).sin() * 0.14;
    let glide_phase = 0.5 + 0.5 * (t * 1.2).sin();
    0.09 + flutter * (0.56 * (1.0 - glide * (1.0 - glide_phase)))
}

fn bone_pose(
    bone: Bone,
    motion: &Wildlife,
    at: &Transform,
    body: Transform,
    terrain: &Terrain,
) -> Transform {
    match bone {
        Bone::Body => Transform::from_xyz(
            0.0,
            (motion.gait * 2.0 * TAU).cos() * 0.009 * motion.pace * (1.0 - motion.air),
            0.0,
        ),
        Bone::Foot { tip, hip, phase, .. } => {
            let (stride_phase, lift) = step_cycle(motion.gait + phase);
            let pounce = if motion.kind == Kind::Frostwolf && tip.z > 0.2 { motion.lunge } else { 0.0 };
            let forward = stride_phase * stride(motion.kind, motion.run) * motion.pace + 0.08 * pounce;
            let foot_at = at.transform_point(body.transform_point(tip + Vec3::Z * forward));
            let ground = if motion.air < 0.6 {
                (terrain.mesh_height_at(foot_at.xz()) + 0.012 + tip.y * at.scale.y - foot_at.y)
                    / at.scale.y
            } else {
                0.0
            };
            let mut posed = tip + Vec3::new(
                0.0,
                ground.clamp(-0.15, 0.15) + lift * (0.075 + motion.run * 0.065) * motion.pace + 0.10 * pounce,
                forward,
            );
            // Hold the foot's height but limit stride reach so the skin cannot stretch a planted leg.
            let rest = tip - hip;
            let reach = (1.12 * rest.length_squared() - (posed.x - hip.x).powi(2) - (posed.y - hip.y).powi(2))
                .max(0.0).sqrt();
            posed.z = posed.z.clamp(hip.z - reach, hip.z + reach);
            posed = posed.lerp(tip + Vec3::new(0.0, 0.19, -0.15), motion.air);
            let leg = posed - hip;
            let bend = (rest.y * leg.z - rest.z * leg.y).atan2(rest.y * leg.y + rest.z * leg.z);
            Transform::from_translation(posed)
                .with_rotation(Quat::from_rotation_x(bend * 0.35 + lift * 0.11 * motion.pace))
        }
        Bone::Head(neck) => {
            let scan = (motion.age * 1.2 + motion.phase()).sin()
                * if motion.kind == Kind::Chameleon {
                    0.25
                } else {
                    0.04
                };
            let bite = motion.lunge * 0.29;
            Transform::from_translation(neck + Vec3::new(0.0, -bite * 0.10, bite * 0.23))
                .with_rotation(Quat::from_euler(
                    EulerRot::YXZ,
                    scan,
                    bite - motion.run * 0.09,
                    0.0,
                ))
        }
        Bone::Wing {
            shoulder,
            side,
            open,
        } => Transform::from_translation(shoulder)
            .with_rotation(Quat::from_rotation_z(open + side * wingbeat(motion))),
    }
}

pub(in crate::world) fn pose_wildlife(
    terrain: Res<Terrain>,
    camera: Query<&Transform, (With<Camera3d>, Without<VisualPart>)>,
    wildlife: Query<(&Wildlife, &Transform, &Articulation), Without<VisualPart>>,
    mut parts: Query<(&mut Transform, Option<&mut Visibility>), With<VisualPart>>,
) {
    let camera = camera.single().ok();
    for (motion, at, rig) in &wildlife {
        if camera.is_some_and(|camera| {
            at.translation.distance_squared(camera.translation) > (rig.distance + 27.0).powi(2)
        }) {
            continue;
        }
        let body = carrier(motion);
        if let Ok((mut transform, _)) = parts.get_mut(rig.carrier) {
            *transform = body;
        }
        for &(bone, entity) in &rig.joints {
            if let Ok((mut transform, _)) = parts.get_mut(entity) {
                *transform = bone_pose(bone, motion, at, body, &terrain);
            }
        }

        if let Some(wings) = rig.feathers {
            for (side, entity) in [-1.0, 1.0].into_iter().zip(wings) {
                if let Ok((mut transform, Some(mut visibility))) = parts.get_mut(entity) {
                    let open = smooth((motion.air - 0.05) / 0.54);
                    *visibility = if open > 0.03 {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                    let span = if motion.kind == Kind::Sparrow {
                        0.76
                    } else {
                        1.0
                    };
                    *transform = Transform::from_translation(body.transform_point(Vec3::new(
                        side * 0.117,
                        0.43,
                        0.096,
                    )))
                    .with_rotation(
                        Quat::from_rotation_z(side * wingbeat(motion))
                            * Quat::from_rotation_y(side * (0.1 + (1.0 - open) * 0.74)),
                    )
                    .with_scale(Vec3::new(
                        (open * span).max(0.01),
                        span,
                        span,
                    ));
                }
            }
        }
    }
}
