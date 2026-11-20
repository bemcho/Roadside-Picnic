//! Stands of dead trees scattered across the Zone: groves and clearings on a
//! jittered grid, placed deterministically from the terrain's woodland
//! suitability. Each model is a whole stand of trunks and fallen logs.

use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::assets;

use super::collision::{Collider, Pillar};
use super::terrain::{HALF_SIZE, SPAWN, Terrain, lattice_hash, value_noise};

/// Spacing of the placement grid; each cell holds at most one stand.
const CELL: f32 = 22.0;

/// Fixed seed so the forest is the same on every run.
const SEED: u32 = 0x7EE5_1979;

/// Keeps the stalker's starting point clear.
const SPAWN_CLEARING: f32 = 16.0;

/// Stands stay this far inside the terrain edge.
const EDGE_MARGIN: f32 = 8.0;

/// Range of stand scale: meters per model unit (a stand is ~0.95 units wide).
const SCALE_RANGE: (f32, f32) = (11.0, 15.0);

/// Radius of a stand's footprint, in model units.
const FOOTPRINT: f32 = 0.48;

/// Largest ground rise across a footprint, in meters; steeper ground would
/// bury half a stand.
const MAX_FOOTPRINT_RISE: f32 = 1.6;

/// How far below the lowest ground under the stand the trunks start, so no
/// trunk ever floats.
const ROOT_SINK: f32 = 0.15;

/// Pillars at or below this height (model units) touch the ground; the stand
/// is sunk so none of them floats.
const GROUND_CONTACT: f32 = 0.01;

/// Dead-tree stand models and their collision pillars, measured by
/// tools/trim_tree_base.py.
const MODELS: [(&str, &[Pillar]); 2] = [
    (assets::TREE_TALL, include!("tree_pillars/tall_tree.rs")),
    (
        assets::TREE_ROADSIDE,
        include!("tree_pillars/roadside_tree.rs"),
    ),
];

/// Where one tree stands and how it is posed.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TreeSite {
    pub model: usize,
    pub transform: Transform,
}

/// Scatters trees: one candidate per grid cell, jittered, kept where both
/// the land suits trees and a low-frequency grove field is dense.
pub(super) fn tree_sites(terrain: &Terrain) -> Vec<TreeSite> {
    let cells = (2.0 * HALF_SIZE / CELL) as i32;
    let limit = HALF_SIZE - EDGE_MARGIN;
    let mut sites = Vec::new();

    for iz in 0..cells {
        for ix in 0..cells {
            let roll = |n: u32| lattice_hash(ix, iz, SEED + n);
            let x = -HALF_SIZE + (ix as f32 + roll(0)) * CELL;
            let z = -HALF_SIZE + (iz as f32 + roll(1)) * CELL;
            if x.abs() > limit || z.abs() > limit {
                continue;
            }
            if Vec2::new(x - SPAWN.0, z - SPAWN.1).length() < SPAWN_CLEARING {
                continue;
            }
            let grove = value_noise(x / 45.0, z / 45.0, SEED + 2);
            let density = terrain.woodland_at(x, z) * smoothstep(0.3, 0.7, grove);
            if roll(2) >= density {
                continue;
            }

            let model = (roll(3) * MODELS.len() as f32) as usize % MODELS.len();
            let scale = SCALE_RANGE.0 + (SCALE_RANGE.1 - SCALE_RANGE.0) * roll(4);
            let rotation = Quat::from_rotation_y(roll(5) * TAU);
            let contacts = ground_contacts(MODELS[model].1)
                .map(|p| Vec2::new(x, z) + (rotation * Vec3::new(p.x, 0.0, p.z) * scale).xz());
            let Some(base) = footprint_base(terrain, Vec2::new(x, z), FOOTPRINT * scale, contacts)
            else {
                continue;
            };
            let transform = Transform::from_xyz(x, base - ROOT_SINK, z)
                .with_rotation(rotation)
                .with_scale(Vec3::splat(scale));
            sites.push(TreeSite { model, transform });
        }
    }
    sites
}

/// The pillars of a stand that rest on the ground.
fn ground_contacts(pillars: &[Pillar]) -> impl Iterator<Item = &Pillar> {
    pillars.iter().filter(|p| p.bottom <= GROUND_CONTACT)
}

/// Lowest ground under a stand: at each ground contact and around its
/// footprint of `radius`. `None` if the ground rises too much across it.
fn footprint_base(
    terrain: &Terrain,
    center: Vec2,
    radius: f32,
    contacts: impl Iterator<Item = Vec2>,
) -> Option<f32> {
    let ring = (0..16).map(|i| {
        let angle = (i % 8) as f32 * TAU / 8.0;
        let reach = if i < 8 { 0.5 } else { 1.0 } * radius;
        center + Vec2::from_angle(angle) * reach
    });
    let (low, high) = std::iter::once(center)
        .chain(ring)
        .chain(contacts)
        .map(|p| terrain.height_at(p.x, p.y))
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), h| {
            (lo.min(h), hi.max(h))
        });
    (high - low <= MAX_FOOTPRINT_RISE).then_some(low)
}

/// Spawns every stand as a child of `parent`, sharing one scene per model,
/// each solid through its model's pillars.
pub(super) fn spawn_trees(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    sites: &[TreeSite],
) {
    let scenes = MODELS.map(|(path, _)| asset_server.load(format!("{path}#Scene0")));
    for site in sites {
        let (_, pillars) = MODELS[site.model];
        parent.spawn((
            WorldAssetRoot(scenes[site.model].clone()),
            site.transform,
            Collider(pillars),
        ));
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forest_is_deterministic_and_sized_for_the_zone() {
        let terrain = Terrain::new();
        let sites = tree_sites(&terrain);
        assert_eq!(sites, tree_sites(&terrain));
        assert!((25..=90).contains(&sites.len()), "{} stands", sites.len());
        assert!(sites.iter().any(|s| s.model == 0) && sites.iter().any(|s| s.model == 1));
    }

    #[test]
    fn stands_avoid_spawn_road_and_quarry_and_never_float() {
        let terrain = Terrain::new();
        for site in tree_sites(&terrain) {
            let p = site.transform.translation;
            assert!(p.xz().length() >= SPAWN_CLEARING, "stand at spawn {p}");
            assert!(
                terrain.woodland_at(p.x, p.z) > 0.0,
                "stand on barren ground {p}"
            );
            for pillar in ground_contacts(MODELS[site.model].1) {
                let foot =
                    site.transform
                        .transform_point(Vec3::new(pillar.x, pillar.bottom, pillar.z));
                let ground = terrain.height_at(foot.x, foot.z);
                assert!(foot.y < ground, "floating stand at {foot}, ground {ground}");
            }
        }
        // The road's midpoint and the quarry floor are barren.
        assert_eq!(terrain.woodland_at(18.0, 10.0), 0.0);
        assert_eq!(terrain.woodland_at(70.0, 40.0), 0.0);
    }
}
