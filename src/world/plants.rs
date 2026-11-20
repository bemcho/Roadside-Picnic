//! Grounded, biome-specific plants streamed with the surrounding scenery.

use std::f32::consts::TAU;

use bevy::camera::visibility::VisibilityRange;
use bevy::gltf::GltfMaterial;
use bevy::light::NotShadowCaster;
use bevy::pbr::gltf::standard_material_from_gltf_material;
use bevy::prelude::*;

use crate::assets::{self, PreloadedAssets};

use super::hydrology::Wetlands;
use super::scenery::forest_density;
use super::terrain::{
    BASE, HALF_SIZE, TILE_SIZE, Terrain, lattice_hash, lerp, smoothstep, value_noise,
};

const CELL: f32 = 8.0;
const SEED: u32 = 0x1035_F108;

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

    fn draw_distance(self) -> f32 {
        match self {
            Self::DryGrass => 55.0,
            Self::Glowweed => 105.0,
            Self::Hogweed => 65.0,
            Self::Shrub => 58.0,
            Self::Thornbush => 64.0,
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
    casts_shadow: bool,
}

impl PlantSite {
    fn grounded(
        terrain: &Terrain,
        kind: Kind,
        at: Vec2,
        y: f32,
        size: f32,
        angle: f32,
        casts_shadow: bool,
    ) -> Option<Self> {
        let (small, big, roots) = kind.size_and_roots();
        let scale = lerp(small, big, size);
        let base = root_height(terrain, at, y, angle, scale * roots);
        if y - base > 0.22 * scale {
            return None;
        }
        Some(Self {
            kind,
            pose: Transform::from_xyz(at.x, base - 0.06, at.y)
                .with_rotation(Quat::from_rotation_y(angle))
                .with_scale(Vec3::splat(scale)),
            casts_shadow,
        })
    }
}

pub(super) fn plant_sites(terrain: &Terrain, tile: IVec2) -> Vec<PlantSite> {
    let mut sites = Vec::new();
    for layer in 0..5 {
        grow_plant_layer(terrain, tile, layer, &mut sites);
    }
    grow_hangar_plants(terrain, tile, &mut sites);
    sites
}

fn grow_plant_layer(terrain: &Terrain, tile: IVec2, layer: u32, sites: &mut Vec<PlantSite>) {
    let cells = (TILE_SIZE / CELL) as i32;
    let start = tile * cells;
    for z in start.y..start.y + cells {
        for x in start.x..start.x + cells {
            let roll = |n| lattice_hash(x, z, SEED + 17 * layer + n);
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
            sites.extend(PlantSite::grounded(
                terrain,
                kind,
                at,
                y,
                roll(4),
                roll(5) * TAU,
                layer == 0 && kind == Kind::Hogweed,
            ));
        }
    }
}

fn grow_hangar_plants(terrain: &Terrain, tile: IVec2, sites: &mut Vec<PlantSite>) {
    let centers = [
        Vec2::new(-19.5, 23.0),
        Vec2::new(19.5, 23.0),
        Vec2::new(-21.0, -4.0),
        Vec2::new(21.0, -4.0),
        Vec2::new(-10.5, -23.0),
        Vec2::new(10.5, -23.0),
    ];
    for (bed, center) in centers.into_iter().enumerate() {
        for (i, kind) in Kind::ALL.into_iter().cycle().take(10).enumerate() {
            let roll = |n| lattice_hash(bed as i32, i as i32, SEED + n);
            let angle = (i as f32 * 0.1 + roll(11) * 0.06) * TAU;
            let reach = lerp(1.0, 2.7, roll(12));
            let at = Vec2::from(BASE) + center + Vec2::from_angle(angle) * reach;
            if (at / TILE_SIZE).floor().as_ivec2() != tile
                || terrain.road_mask(at.x, at.y) > 0.025
                || terrain.water_depth_at(at.x, at.y) > 0.025
            {
                continue;
            }
            sites.extend(PlantSite::grounded(
                terrain,
                kind,
                at,
                terrain.height_at(at.x, at.y),
                roll(13),
                angle,
                false,
            ));
        }
    }
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
    for &PlantSite {
        kind,
        pose,
        casts_shadow,
    } in sites
    {
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
        if !casts_shadow {
            plant.insert(NotShadowCaster);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Collider;
    use crate::world::terrain::TILE_RADIUS;

    #[test]
    fn the_wild_plant_pack_is_fivefold_and_still_avoids_the_base_and_road() {
        let terrain = Terrain::new();
        let wet_tile = IVec2::new(-2, 1);
        assert_eq!(
            plant_sites(&terrain, wet_tile),
            plant_sites(&terrain, wet_tile)
        );
        let mut counts = [0; Kind::ALL.len()];
        let mut original_counts = [0; Kind::ALL.len()];
        let mut original = Vec::new();
        for z in -TILE_RADIUS..TILE_RADIUS {
            for x in -TILE_RADIUS..TILE_RADIUS {
                let tile = IVec2::new(x, z);
                grow_plant_layer(&terrain, tile, 0, &mut original);
                for site in original.drain(..) {
                    original_counts[site.kind as usize] += 1;
                }
                for site in plant_sites(&terrain, tile) {
                    let p = site.pose.translation;
                    if terrain.base_mask(p.x, p.z) > 0.05 {
                        continue;
                    }
                    counts[site.kind as usize] += 1;
                    let wet = Wetlands::at(p.x, p.z);
                    let y = terrain.height_at(p.x, p.z);
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
        for kind in Kind::ALL {
            let before = original_counts[kind as usize];
            let after = counts[kind as usize];
            assert!(
                (4.6..=5.4).contains(&(after as f32 / before as f32)),
                "{kind:?}: {before} -> {after}"
            );
        }
    }

    #[test]
    fn all_five_plant_types_are_grounded_around_the_hangar_without_blocking_its_lane() {
        let terrain = Terrain::new();
        let mut count = [0; Kind::ALL.len()];
        for z in [-2, -1] {
            for site in plant_sites(&terrain, IVec2::new(0, z)) {
                let at = site.pose.translation;
                if terrain.base_mask(at.x, at.z) <= 0.05 {
                    continue;
                }
                let offset = at.xz() - Vec2::from(BASE);
                assert!(offset.x.abs() > 17.0 || offset.y.abs() > 20.0);
                assert!(terrain.road_mask(at.x, at.z) <= 0.025);
                assert!(at.y <= terrain.mesh_height_at(at.xz()));
                count[site.kind as usize] += 1;
            }
        }
        assert!(count.into_iter().all(|n| n >= 8), "{count:?}");
    }

    #[test]
    fn spawned_bushes_are_visual_only() {
        let mut world = World::new();
        let root = world.spawn(Transform::IDENTITY).id();
        let sites = [Kind::Shrub, Kind::Thornbush].map(|kind| PlantSite {
            kind,
            pose: Transform::IDENTITY,
            casts_shadow: false,
        });
        world.commands().entity(root).with_children(|parent| {
            spawn_plants(parent, &PlantModels::default(), &sites);
        });
        world.flush();
        let collision = world
            .query_filtered::<Has<Collider>, With<Mesh3d>>()
            .iter(&world)
            .collect::<Vec<_>>();
        assert_eq!(collision, [false, false]);
    }
}
