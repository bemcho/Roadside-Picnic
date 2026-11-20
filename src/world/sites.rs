//! Grounded, roadside landmarks distributed across the Zone.

use bevy::gltf::GltfMaterial;
use bevy::pbr::gltf::standard_material_from_gltf_material;
use bevy::prelude::*;

use crate::assets::{self, PreloadedAssets};

use super::collision::{Collider, Pillar};
use super::roads;
use super::terrain::{TILE_SIZE, Terrain, smoothstep};
use super::{WorldRoot, detail_range};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Factory,
    Cemetery,
    Garage,
    PlagueQuarter,
    Railway,
    Schoolhouse,
    Silos,
    Substation,
}

macro_rules! model {
    ($name:literal, $file:literal, $scale:literal, $datum:literal, $front:literal) => {
        Model {
            name: $name,
            path: concat!("models/sites/site-", $file, ".glb"),
            scale: $scale,
            datum: $datum,
            front: $front,
        }
    };
}

struct Model {
    name: &'static str,
    path: &'static str,
    scale: f32,
    datum: f32,
    front: f32,
}

const MODELS: [Model; 8] = [
    model!("Brick factory", "brick-factory", 24.0, 0.008, 0.39),
    model!("Cemetery", "cemetery", 3.2, 0.008, 0.20),
    model!("Garage", "garage", 11.0, 0.155, 0.49),
    model!("Plague quarter", "plague-quarter", 15.0, 0.055, 0.50),
    model!(
        "Railway embankment",
        "railway-embankment",
        29.0,
        0.016,
        0.50
    ),
    model!("Schoolhouse", "schoolhouse", 11.5, 0.103, 0.402),
    model!("Silos", "silos", 18.0, 0.017, 0.453),
    model!("Substation", "substation", 8.0, 0.016, 0.401),
];

#[allow(clippy::approx_constant, reason = "Measured model coordinates")]
const PILLARS: [&[Pillar]; MODELS.len()] = [
    include!("pillars/site_brick_factory.rs"),
    include!("pillars/site_cemetery.rs"),
    include!("pillars/site_garage.rs"),
    include!("pillars/site_plague_quarter.rs"),
    include!("pillars/site_railway_embankment.rs"),
    include!("pillars/site_schoolhouse.rs"),
    include!("pillars/site_silos.rs"),
    include!("pillars/site_substation.rs"),
];

#[derive(Clone, Copy)]
pub(super) struct Landmark {
    kind: Kind,
    pub center: Vec2,
    pub junction: Vec2,
}

impl Landmark {
    const fn new(kind: Kind, x: f32, z: f32, junction: Vec2) -> Self {
        Self {
            kind,
            center: Vec2::new(x, z),
            junction,
        }
    }

    fn model(self) -> &'static Model {
        &MODELS[self.kind as usize]
    }

    fn toward_road(self) -> Vec2 {
        (self.junction - self.center).normalize()
    }

    pub fn entrance(self) -> Vec2 {
        let m = self.model();
        self.center + self.toward_road() * (m.scale * m.front + 7.0)
    }

    fn pad(self) -> f32 {
        self.model().scale * 0.72 + 9.0
    }

    fn pose(self, terrain: &Terrain) -> Transform {
        let facing = self.toward_road();
        let scale = self.model().scale;
        Transform::from_xyz(
            self.center.x,
            terrain.mesh_height_at(self.center) - self.model().datum * scale,
            self.center.y,
        )
        .with_rotation(Quat::from_rotation_y(-facing.y.atan2(facing.x)))
        .with_scale(Vec3::splat(scale))
    }
}

pub(super) const LANDMARKS: [Landmark; 16] = [
    Landmark::new(Kind::Garage, -197.0, -88.0, roads::WEST_GARAGE),
    Landmark::new(Kind::Garage, 440.0, -516.0, roads::SOUTH_GARAGE),
    Landmark::new(Kind::Silos, 270.0, 114.0, roads::EAST_SILOS),
    Landmark::new(Kind::Silos, 896.0, 1030.0, roads::NORTH_SILOS),
    Landmark::new(Kind::Factory, -1070.0, 190.0, roads::WEST_FACTORY),
    Landmark::new(Kind::Factory, 1190.0, 235.0, roads::EAST_FACTORY),
    Landmark::new(Kind::Cemetery, -1320.0, 905.0, roads::WEST_CEMETERY),
    Landmark::new(Kind::Cemetery, 1280.0, -1250.0, roads::SOUTH_CEMETERY),
    Landmark::new(Kind::PlagueQuarter, -320.0, -575.0, roads::SOUTH_PLAGUE),
    Landmark::new(Kind::PlagueQuarter, -860.0, 1275.0, roads::NORTH_PLAGUE),
    Landmark::new(Kind::Railway, -1260.0, -785.0, roads::WEST_RAIL),
    Landmark::new(Kind::Railway, 700.0, 842.0, roads::NORTH_RAIL),
    Landmark::new(Kind::Schoolhouse, -225.0, 510.0, roads::NORTH_SCHOOL),
    Landmark::new(Kind::Schoolhouse, 1195.0, -719.0, roads::SOUTH_SCHOOL),
    Landmark::new(Kind::Substation, 1177.0, 512.0, roads::NORTH_SUBSTATION),
    Landmark::new(Kind::Substation, 250.0, -1320.0, roads::SOUTH_SUBSTATION),
];

pub(super) fn foundation(at: Vec2) -> Option<(Vec2, f32)> {
    LANDMARKS.into_iter().find_map(|site| {
        let distance = at.distance_squared(site.center);
        let pad = site.pad();
        let outer = pad + 26.0;
        (distance < outer * outer).then(|| (site.center, smoothstep(outer, pad, distance.sqrt())))
    })
}

pub(super) fn clear_footprint(at: Vec2, clearance: f32) -> bool {
    LANDMARKS
        .into_iter()
        .any(|site| at.distance_squared(site.center) < (site.pad() + clearance).powi(2))
}

pub(super) fn grove(at: Vec2) -> f32 {
    LANDMARKS.into_iter().fold(0.0_f32, |density, site| {
        let radius = site.pad() + 52.0;
        let distance = at.distance_squared(site.center);
        density.max(smoothstep(
            radius * radius,
            (radius * 0.65).powi(2),
            distance,
        ))
    })
}

pub(super) fn preload_sites(server: Res<AssetServer>, mut preloaded: ResMut<PreloadedAssets>) {
    for model in &MODELS {
        let (mesh, paint) = assets::load_single_primitive(&server, model.path);
        preloaded.0.extend([mesh.untyped(), paint.untyped()]);
    }
}

pub(super) fn spawn_sites(
    mut commands: Commands,
    terrain: Res<Terrain>,
    server: Res<AssetServer>,
    paints: Res<Assets<GltfMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let models = MODELS.each_ref().map(|model| {
        let (mesh, paint) = assets::load_single_primitive(&server, model.path);
        let paint = paints.get(&paint).expect("preloaded landmark material");
        (
            mesh,
            materials.add(standard_material_from_gltf_material(paint)),
        )
    });
    for site in LANDMARKS {
        let (mesh, paint) = &models[site.kind as usize];
        commands.spawn((
            WorldRoot,
            Name::new(site.model().name),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(paint.clone()),
            site.pose(&terrain),
            detail_range(385.0, 45.0),
        ));
    }
}

pub(super) fn spawn_solids(parent: &mut ChildSpawnerCommands, terrain: &Terrain, tile: IVec2) {
    for site in LANDMARKS {
        if (site.center / TILE_SIZE).floor().as_ivec2() == tile {
            parent.spawn((site.pose(terrain), Collider(PILLARS[site.kind as usize])));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Body, TERRAIN_HALF_SIZE, push_out};

    #[test]
    fn both_copies_have_dry_clear_pads_in_separate_regions() {
        let terrain = Terrain::new();
        let mut count = [0; MODELS.len()];
        let mut quadrants = [0; 4];
        for (i, site) in LANDMARKS.into_iter().enumerate() {
            count[site.kind as usize] += 1;
            let quadrant = usize::from(site.center.x > 0.0) + 2 * usize::from(site.center.y > 0.0);
            quadrants[quadrant] += 1;
            assert!(site.center.abs().max_element() < TERRAIN_HALF_SIZE - 110.0);
            assert!(
                LANDMARKS[..i]
                    .iter()
                    .all(|other| other.center.distance(site.center) > 235.0)
            );

            let origin = terrain.mesh_height_at(site.center);
            let pose = site.pose(&terrain);
            for x in [-0.5, 0.0, 0.5] {
                for z in [-0.5, 0.0, 0.5] {
                    let at = pose.transform_point(Vec3::new(x, 0.0, z)).xz();
                    assert!(
                        (terrain.mesh_height_at(at) - origin).abs() < 0.025,
                        "{:?} pad tilts",
                        site.kind
                    );
                    assert_eq!(
                        terrain.water_depth_at(at.x, at.y),
                        0.0,
                        "{:?} flooded",
                        site.kind
                    );
                    assert!(clear_footprint(at, 0.0));
                }
            }
            assert!(terrain.road_mask(site.entrance().x, site.entrance().y) > 0.99);
        }
        assert_eq!(count, [2; MODELS.len()]);
        assert!(
            quadrants.into_iter().all(|count| count >= 3),
            "{quadrants:?}"
        );
    }

    #[test]
    fn the_rotated_garage_has_an_open_bay_and_a_solid_wall() {
        let terrain = Terrain::new();
        let body = Body {
            radius: 0.35,
            height: 1.8,
        };
        let site = LANDMARKS[0];
        let pose = site.pose(&terrain);
        let global = GlobalTransform::from(pose);
        let collider = Collider(PILLARS[Kind::Garage as usize]);
        let feet = terrain.mesh_height_at(site.center);
        for i in 6..=35 {
            let at = pose
                .transform_point(Vec3::new(i as f32 * 0.02, 0.0, -0.16))
                .xz();
            assert_eq!(push_out(at, feet, body, [(&collider, &global)]), at);
        }
        let back = pose.transform_point(Vec3::new(-0.32, 0.0, 0.0)).xz();
        assert_ne!(push_out(back, feet, body, [(&collider, &global)]), back);
    }
}
