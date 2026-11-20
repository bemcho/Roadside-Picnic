//! Grounded, biome-specific plants streamed with the surrounding scenery.

use std::f32::consts::TAU;

use bevy::camera::visibility::VisibilityRange;
use bevy::gltf::GltfMaterial;
use bevy::light::NotShadowCaster;
use bevy::pbr::gltf::standard_material_from_gltf_material;
use bevy::prelude::*;

use crate::assets::{self, PreloadedAssets};

use super::collision::{Collider, Pillar};
use super::hydrology::Wetlands;
use super::scenery::forest_density;
use super::terrain::{HALF_SIZE, TILE_SIZE, Terrain, lattice_hash, lerp, smoothstep, value_noise};

const CELL: f32 = 8.0;
const SEED: u32 = 0x1035_F108;
const SHRUB: &[Pillar] = &[Pillar {
    top: 0.63,
    bottom: 0.0,
    ..Pillar::centered(0.34, 0.0)
}];
const THORNS: &[Pillar] = &[Pillar {
    top: 0.73,
    bottom: 0.0,
    ..Pillar::centered(0.23, 0.0)
}];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    DryGrass,
    Glowweed,
    Hogweed,
    Shrub,
    Thornbush,
}

impl Kind {
    const ALL: [Self; 5] = [
        Self::DryGrass,
        Self::Glowweed,
        Self::Hogweed,
        Self::Shrub,
        Self::Thornbush,
    ];

    fn path(self) -> &'static str {
        match self {
            Self::DryGrass => assets::DRY_GRASS,
            Self::Glowweed => assets::GLOWWEED,
            Self::Hogweed => assets::HOGWEED,
            Self::Shrub => assets::SHRUB,
            Self::Thornbush => assets::THORNBUSH,
        }
    }

    fn size_and_roots(self) -> (f32, f32, f32) {
        match self {
            Self::DryGrass => (0.50, 1.08, 0.18),
            Self::Glowweed => (0.64, 1.40, 0.37),
            Self::Hogweed => (1.1, 1.92, 0.17),
            Self::Shrub => (1.0, 1.95, 0.40),
            Self::Thornbush => (0.90, 1.58, 0.27),
        }
    }

    fn collider(self) -> Option<Collider> {
        match self {
            Self::Shrub => Some(Collider(SHRUB)),
            Self::Thornbush => Some(Collider(THORNS)),
            _ => None,
        }
    }

    fn draw_distance(self) -> f32 {
        match self {
            Self::DryGrass => 55.0,
            Self::Glowweed => 105.0,
            Self::Hogweed => 80.0,
            Self::Shrub => 90.0,
            Self::Thornbush => 98.0,
        }
    }
}

#[derive(Resource, Default)]
pub(super) struct PlantModels([(Handle<Mesh>, Handle<StandardMaterial>); Kind::ALL.len()]);

pub(super) fn preload_plants(server: Res<AssetServer>, mut preloaded: ResMut<PreloadedAssets>) {
    for kind in Kind::ALL {
        let (mesh, paint) = assets::load_single_primitive(&server, kind.path());
        preloaded.0.extend([mesh.untyped(), paint.untyped()]);
    }
}

pub(super) fn prepare_plant_materials(
    mut commands: Commands,
    server: Res<AssetServer>,
    paints: Res<Assets<GltfMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(PlantModels(Kind::ALL.map(|kind| {
        let (mesh, paint) = assets::load_single_primitive(&server, kind.path());
        let paint = paints
            .get(&paint)
            .expect("plant materials finish preloading before the Zone opens");
        (
            mesh,
            materials.add(standard_material_from_gltf_material(paint)),
        )
    })));
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct PlantSite {
    kind: Kind,
    pose: Transform,
}

pub(super) fn plant_sites(terrain: &Terrain, tile: IVec2) -> Vec<PlantSite> {
    let cells = (TILE_SIZE / CELL) as i32;
    let start = tile * cells;
    let mut sites = Vec::new();
    for z in start.y..start.y + cells {
        for x in start.x..start.x + cells {
            let roll = |n| lattice_hash(x, z, SEED + n);
            let at = (Vec2::new(x as f32, z as f32) + Vec2::new(roll(0), roll(1))) * CELL;
            if at.abs().max_element() > HALF_SIZE - 3.0
                || terrain.base_mask(at.x, at.y) > 0.05
                || terrain.road_mask(at.x, at.y) > 0.025
            {
                continue;
            }
            let y = terrain.height_at(at.x, at.y);
            let patch = smoothstep(0.17, 0.73, value_noise(at.x / 34.0, at.y / 34.0, SEED + 9));
            let density = 0.42 * patch * smoothstep(97.0, 44.0, y);
            if roll(2) >= density {
                continue;
            }
            let wet = Wetlands::at(at.x, at.y);
            let depth = wet.depth(y);
            if depth > 0.20 || terrain.normal_at(at.x, at.y).y < 0.87 {
                continue;
            }
            let damp = wet.wetness();
            let dry = if depth > 0.025 { 0.0 } else { 1.0 - damp };
            let forest = forest_density(at.x, at.y);
            let glow = smoothstep(0.28, 0.78, damp);
            let weights = [
                (0.38 - forest * 0.22) * dry,
                0.34 * glow,
                0.07 * (1.0 - forest) * dry + 0.19 * damp,
                (0.08 + forest * 0.32) * dry,
                (0.08 + 0.14 * smoothstep(4.0, 28.0, y)) * dry,
            ];
            let Some(kind) = pick_plant(roll(3), weights) else {
                continue;
            };
            let (small, big, roots) = kind.size_and_roots();
            let scale = lerp(small, big, roll(4));
            let angle = roll(5) * TAU;
            let base = root_height(terrain, at, y, angle, scale * roots);
            if y - base > 0.22 * scale {
                continue;
            }
            sites.push(PlantSite {
                kind,
                pose: Transform::from_xyz(at.x, base - 0.06, at.y)
                    .with_rotation(Quat::from_rotation_y(angle))
                    .with_scale(Vec3::splat(scale)),
            });
        }
    }
    sites
}

fn pick_plant(mut roll: f32, weights: [f32; Kind::ALL.len()]) -> Option<Kind> {
    for (kind, weight) in Kind::ALL.into_iter().zip(weights) {
        if roll < weight {
            return Some(kind);
        }
        roll -= weight;
    }
    None
}

fn root_height(terrain: &Terrain, at: Vec2, height: f32, angle: f32, radius: f32) -> f32 {
    (0..6).fold(height, |low, i| {
        let p = at + Vec2::from_angle(angle + TAU / 6.0 * i as f32) * radius;
        low.min(terrain.height_at(p.x, p.y))
    })
}

pub(super) fn spawn_plants(
    parent: &mut ChildSpawnerCommands,
    models: &PlantModels,
    sites: &[PlantSite],
) {
    for &PlantSite { kind, pose } in sites {
        let (mesh, paint) = &models.0[kind as usize];
        let distance = kind.draw_distance();
        let mut plant = parent.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(paint.clone()),
            pose,
            VisibilityRange {
                start_margin: 0.0..0.0,
                end_margin: distance..distance + 10.0,
                use_aabb: false,
            },
        ));
        if let Some(collider) = kind.collider() {
            plant.insert(collider);
        } else if kind != Kind::Hogweed {
            plant.insert(NotShadowCaster);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::terrain::TILE_RADIUS;
    use crate::world::{Body, push_out};

    #[test]
    fn the_pack_is_distributed_in_grounded_patches_clear_of_the_base_and_road() {
        let terrain = Terrain::new();
        let wet_tile = IVec2::new(-2, 1);
        assert_eq!(
            plant_sites(&terrain, wet_tile),
            plant_sites(&terrain, wet_tile)
        );
        let mut counts = [0; Kind::ALL.len()];
        for z in -TILE_RADIUS..TILE_RADIUS {
            for x in -TILE_RADIUS..TILE_RADIUS {
                for site in plant_sites(&terrain, IVec2::new(x, z)) {
                    counts[site.kind as usize] += 1;
                    let p = site.pose.translation;
                    let wet = Wetlands::at(p.x, p.z);
                    let y = terrain.height_at(p.x, p.z);
                    assert!(terrain.base_mask(p.x, p.z) <= 0.05);
                    assert!(terrain.road_mask(p.x, p.z) <= 0.025);
                    assert!(wet.depth(y) <= 0.20);
                    assert!(p.y < y && y - p.y < 0.5);
                    if site.kind == Kind::Glowweed {
                        assert!(wet.wetness() > 0.28);
                    }
                }
            }
        }
        assert!(
            counts.into_iter().all(|count| count >= 12),
            "plant counts: {counts:?}"
        );
    }

    #[test]
    fn only_woody_bushes_block_travel() {
        let pose = GlobalTransform::IDENTITY;
        for kind in Kind::ALL {
            let collider = kind.collider();
            let pushed = push_out(
                Vec2::ZERO,
                0.0,
                Body {
                    radius: 0.35,
                    height: 1.8,
                },
                collider.as_ref().map(|shape| (shape, &pose)),
            );
            assert_eq!(
                pushed != Vec2::ZERO,
                matches!(kind, Kind::Shrub | Kind::Thornbush)
            );
        }
    }
}
