//! Textured trees interspersed with the chunk-batched procedural conifers.

use std::f32::consts::TAU;

use bevy::camera::visibility::VisibilityRange;
use bevy::gltf::GltfMaterial;
use bevy::pbr::gltf::standard_material_from_gltf_material;
use bevy::prelude::*;

use crate::assets::{self, PreloadedAssets};

use super::collision::{Collider, Pillar};
use super::terrain::{Terrain, lattice_hash, lerp};

const SEED: u32 = 0xD4EA_9021;

struct TreeModel {
    path: &'static str,
    scale: (f32, f32),
    footprint: f32,
    draw_distance: f32,
    trunks: &'static [Pillar],
}

const MODELS: [TreeModel; 3] = [
    TreeModel {
        path: assets::TREE_SMALL,
        scale: (4.8, 7.5),
        footprint: 0.34,
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
        draw_distance: 225.0,
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
        draw_distance: 420.0,
        trunks: &[Pillar {
            x: 0.008,
            z: -0.004,
            radius: 0.024,
            bottom: -0.035,
            top: 0.15,
        }],
    },
];

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct ForestTree {
    model: usize,
    pub pose: Transform,
}

impl ForestTree {
    pub(super) fn replace_pine(terrain: &Terrain, pine: Transform, seed: u32) -> Option<Self> {
        let roll = |n| lattice_hash(seed as i32, 0, SEED + n);
        if roll(0) >= 0.34 {
            return None;
        }
        let choice = roll(1);
        let model = if choice < 0.51 {
            0
        } else if choice < 0.77 {
            1
        } else {
            2
        };
        let spec = &MODELS[model];
        let scale = lerp(spec.scale.0, spec.scale.1, roll(2));
        let at = pine.translation.xz();
        let edge =
            (0..8).map(|i| at + Vec2::from_angle(i as f32 * TAU / 8.0) * spec.footprint * scale);
        let center = terrain.mesh_height_at(at);
        let mut low = center;
        let mut high = center;
        for root in edge {
            if terrain.road_mask(root.x, root.y) > 0.02
                || terrain.base_mask(root.x, root.y) > 0.05
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
            model,
            pose: Transform::from_xyz(at.x, low - 0.05, at.y)
                .with_rotation(pine.rotation)
                .with_scale(Vec3::splat(scale)),
        })
    }

    pub(super) fn collider(self) -> Collider {
        Collider(MODELS[self.model].trunks)
    }
}

#[derive(Resource)]
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
    for &ForestTree { model, pose } in sites {
        let (mesh, paint) = &models.0[model];
        let distance = MODELS[model].draw_distance;
        parent.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(paint.clone()),
            pose,
            VisibilityRange {
                start_margin: 0.0..0.0,
                end_margin: distance..distance + 25.0,
                use_aabb: true,
            },
        ));
    }
}
