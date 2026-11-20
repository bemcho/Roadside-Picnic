//! Textured trees interspersed with the chunk-batched procedural conifers.

use std::f32::consts::TAU;

use bevy::gltf::GltfMaterial;
use bevy::light::NotShadowCaster;
use bevy::pbr::gltf::standard_material_from_gltf_material;
use bevy::prelude::*;

use crate::assets::{self, PreloadedAssets};
use crate::player::PlayerCamera;

use super::collision::{Collider, Pillar};
use super::detail_range;
use super::terrain::{BASE, TILE_SIZE, Terrain, lattice_hash, lerp, smoothstep};

const SEED: u32 = 0xD4EA_9021;
const SHADOW_ENTER: f32 = 48.0;
const SHADOW_EXIT: f32 = 55.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Species {
    Small,
    Broadleaf,
    Tall,
    Birch,
    Dead,
}

impl Species {
    pub(super) const ALL: [Self; 5] = [
        Self::Small,
        Self::Broadleaf,
        Self::Tall,
        Self::Birch,
        Self::Dead,
    ];

    fn model(self) -> &'static TreeModel {
        &MODELS[self as usize]
    }
}

struct TreeModel {
    path: &'static str,
    scale: (f32, f32),
    footprint: f32,
    wind_bend: f32,
    draw_distance: f32,
    trunks: &'static [Pillar],
}

const MODELS: [TreeModel; Species::ALL.len()] = [
    TreeModel {
        path: assets::TREE_SMALL,
        scale: (4.8, 7.5),
        footprint: 0.34,
        wind_bend: 0.034,
        draw_distance: 125.0,
        trunks: &[Pillar {
            x: 0.004,
            z: 0.001,
            radius: 0.017,
            bottom: -0.035,
            top: 0.35,
        }],
    },
    TreeModel {
        path: assets::TREE_BIG,
        scale: (13.5, 19.5),
        footprint: 0.22,
        wind_bend: 0.017,
        draw_distance: 195.0,
        trunks: &[Pillar {
            x: 0.019,
            z: 0.022,
            radius: 0.083,
            bottom: -0.035,
            top: 0.20,
        }],
    },
    TreeModel {
        path: assets::TREE_VERY_TALL,
        scale: (25.5, 33.5),
        footprint: 0.05,
        wind_bend: 0.021,
        draw_distance: 335.0,
        trunks: &[Pillar {
            x: 0.008,
            z: -0.004,
            radius: 0.024,
            bottom: -0.035,
            top: 0.15,
        }],
    },
    TreeModel {
        path: assets::TREE_BIRCH,
        scale: (10.0, 15.0),
        footprint: 0.047,
        wind_bend: 0.028,
        draw_distance: 175.0,
        trunks: &[Pillar {
            x: 0.005,
            z: 0.0,
            radius: 0.032,
            bottom: -0.035,
            top: 0.38,
        }],
    },
    TreeModel {
        path: assets::TREE_DEAD,
        scale: (9.5, 14.5),
        footprint: 0.22,
        wind_bend: 0.0,
        draw_distance: 155.0,
        trunks: &[Pillar {
            x: -0.014,
            z: 0.045,
            radius: 0.070,
            bottom: -0.035,
            top: 0.40,
        }],
    },
];

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct ForestTree {
    pub species: Species,
    pub pose: Transform,
}

impl ForestTree {
    pub(super) fn replace_pine(terrain: &Terrain, pine: Transform, seed: u32) -> Option<Self> {
        let roll = |n| lattice_hash(seed as i32, 0, SEED + n);
        if roll(0) >= 0.34 {
            return None;
        }
        let choice = roll(1);
        let species = if choice < 0.32 {
            Species::Small
        } else if choice < 0.52 {
            Species::Broadleaf
        } else if choice < 0.70 {
            Species::Tall
        } else if choice < 0.88 {
            Species::Birch
        } else {
            Species::Dead
        };
        let spec = species.model();
        let scale = lerp(spec.scale.0, spec.scale.1, roll(2));
        Self::grounded(
            terrain,
            species,
            pine.translation.xz(),
            pine.rotation,
            scale,
            false,
        )
    }

    fn grounded(
        terrain: &Terrain,
        species: Species,
        at: Vec2,
        rotation: Quat,
        scale: f32,
        allow_base_pad: bool,
    ) -> Option<Self> {
        let radius = species.model().footprint * scale;
        let edge = (0..8).map(|i| at + Vec2::from_angle(i as f32 * TAU / 8.0) * radius);
        let center = terrain.mesh_height_at(at);
        let mut low = center;
        let mut high = center;
        for root in std::iter::once(at).chain(edge) {
            if terrain.road_mask(root.x, root.y) > 0.02
                || (!allow_base_pad && terrain.base_mask(root.x, root.y) > 0.05)
                || terrain.water_depth_at(root.x, root.y) > 0.0
            {
                return None;
            }
            let y = terrain.mesh_height_at(root);
            low = low.min(y);
            high = high.max(y);
        }
        if high - low > scale * 0.09 {
            return None;
        }
        Some(Self {
            species,
            pose: Transform::from_xyz(at.x, low - 0.05, at.y)
                .with_rotation(rotation)
                .with_scale(Vec3::splat(scale)),
        })
    }

    pub(super) fn collider(self) -> Collider {
        Collider(self.species.model().trunks)
    }
}

pub(super) fn hangar_trees(terrain: &Terrain, tile: IVec2) -> Vec<ForestTree> {
    [
        (Species::Small, -14.7, 23.0, 6.4, 0.6),
        (Species::Broadleaf, -27.0, 16.0, 16.0, 1.4),
        (Species::Tall, 26.0, 17.0, 29.0, 2.0),
        (Species::Birch, 24.0, -11.0, 13.0, 0.4),
        (Species::Dead, -23.0, -18.0, 12.0, -0.8),
    ]
    .into_iter()
    .filter_map(|(species, x, z, scale, yaw)| {
        let at = Vec2::from(BASE) + Vec2::new(x, z);
        if (at / TILE_SIZE).floor().as_ivec2() != tile {
            return None;
        }
        ForestTree::grounded(
            terrain,
            species,
            at,
            Quat::from_rotation_y(yaw),
            scale,
            true,
        )
    })
    .collect()
}

#[derive(Component)]
pub(super) struct CanopyVisual {
    rest: Quat,
    phase: f32,
    bend: f32,
}

impl CanopyVisual {
    fn new(tree: ForestTree) -> Self {
        let cell = tree.pose.translation.xz().floor().as_ivec2();
        let roll = |n| lattice_hash(cell.x, cell.y, SEED + n);
        Self {
            rest: tree.pose.rotation,
            phase: roll(11) * TAU,
            bend: if roll(10) < 0.65 {
                tree.species.model().wind_bend * lerp(0.55, 1.0, roll(12))
            } else {
                0.0
            },
        }
    }

    fn displaced(&self, seconds: f32, distance: f32) -> Quat {
        let bend = self.bend * (1.0 - smoothstep(60.0, 85.0, distance));
        if bend == 0.0 {
            return self.rest;
        }
        let gust = (seconds * 0.53 + self.phase * 0.35).sin();
        let sway = (seconds * 1.12 + self.phase).sin();
        let side = (seconds * 0.76 + self.phase).cos();
        Quat::from_euler(
            EulerRot::XYZ,
            bend * (0.32 + gust * 0.28 + sway * 0.39),
            0.0,
            bend * side * 0.27,
        ) * self.rest
    }
}

#[derive(Resource, Default)]
pub(super) struct CanopyModels([(Handle<Mesh>, Handle<StandardMaterial>); MODELS.len()]);

pub(super) fn preload_canopies(server: Res<AssetServer>, mut preloaded: ResMut<PreloadedAssets>) {
    for spec in &MODELS {
        let (mesh, paint) = assets::load_single_primitive(&server, spec.path);
        preloaded.0.extend([mesh.untyped(), paint.untyped()]);
    }
}

pub(super) fn prepare_canopy_materials(
    mut commands: Commands,
    server: Res<AssetServer>,
    paints: Res<Assets<GltfMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(CanopyModels(MODELS.map(|spec| {
        let (mesh, paint) = assets::load_single_primitive(&server, spec.path);
        let paint = paints
            .get(&paint)
            .expect("tree materials finish preloading before the Zone opens");
        (
            mesh,
            materials.add(standard_material_from_gltf_material(paint)),
        )
    })));
}

pub(super) fn spawn_canopies(
    parent: &mut ChildSpawnerCommands,
    models: &CanopyModels,
    sites: &[ForestTree],
) {
    for &tree in sites {
        let (mesh, paint) = &models.0[tree.species as usize];
        let distance = tree.species.model().draw_distance;
        parent.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(paint.clone()),
            tree.pose,
            detail_range(distance, 25.0),
            CanopyVisual::new(tree),
            NotShadowCaster,
        ));
    }
}

pub(super) fn update_canopy_motion(
    mut commands: Commands,
    time: Res<Time>,
    mut seconds: Local<f32>,
    camera: Single<&Transform, With<PlayerCamera>>,
    mut trees: Query<
        (Entity, &CanopyVisual, &mut Transform, Has<NotShadowCaster>),
        Without<PlayerCamera>,
    >,
) {
    *seconds += time.delta_secs();
    for (entity, visual, mut pose, no_shadow) in &mut trees {
        let distance_sq = pose.translation.distance_squared(camera.translation);
        if no_shadow && distance_sq < SHADOW_ENTER * SHADOW_ENTER {
            commands.entity(entity).remove::<NotShadowCaster>();
        } else if !no_shadow && distance_sq > SHADOW_EXIT * SHADOW_EXIT {
            commands.entity(entity).insert(NotShadowCaster);
        }
        if visual.bend != 0.0 {
            let rotation = visual.displaced(*seconds, distance_sq.sqrt());
            if pose.rotation != rotation {
                pose.rotation = rotation;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hangar_has_a_grounded_tree_of_each_species_outside_the_driveway() {
        let terrain = Terrain::new();
        let trees: Vec<_> = [-2, -1]
            .into_iter()
            .flat_map(|z| hangar_trees(&terrain, IVec2::new(0, z)))
            .collect();
        for species in Species::ALL {
            let Some(tree) = trees.iter().find(|tree| tree.species == species) else {
                panic!("missing {species:?} around the hangar");
            };
            let at = tree.pose.translation;
            let offset = at.xz() - Vec2::from(BASE);
            assert!(offset.length() < 36.0);
            assert!(offset.x.abs() > 19.0 || offset.y > 21.0);
            assert!(at.y <= terrain.mesh_height_at(at.xz()));
            assert!(terrain.road_mask(at.x, at.z) <= 0.02);
        }
    }

    #[test]
    fn wind_moves_nearby_trees_in_place_and_distant_trees_stop_casting_shadows() {
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs_f32(0.5));
        app.insert_resource(time)
            .add_systems(Update, update_canopy_motion);
        let camera = app
            .world_mut()
            .spawn((PlayerCamera, Transform::IDENTITY))
            .id();
        let root = Vec3::new(2.0, 0.0, 1.0);
        let tree = app
            .world_mut()
            .spawn((
                CanopyVisual {
                    rest: Quat::IDENTITY,
                    phase: 1.0,
                    bend: 0.02,
                },
                Transform::from_translation(root),
                NotShadowCaster,
            ))
            .id();
        app.update();
        let near = app.world().entity(tree);
        let rotation = near.get::<Transform>().expect("visual transform").rotation;
        assert_ne!(rotation, Quat::IDENTITY);
        assert!(!near.contains::<NotShadowCaster>());
        app.update();
        assert_ne!(
            app.world()
                .get::<Transform>(tree)
                .expect("visual transform")
                .rotation,
            rotation
        );
        app.world_mut()
            .get_mut::<Transform>(camera)
            .expect("camera transform")
            .translation
            .x = 100.0;
        app.update();
        let far = app.world().entity(tree);
        assert_eq!(
            *far.get::<Transform>().expect("visual transform"),
            Transform::from_translation(root)
        );
        assert!(far.contains::<NotShadowCaster>());
    }
}
