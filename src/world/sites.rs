//! Grounded, roadside landmarks distributed across the Zone.

use std::f32::consts::FRAC_PI_2;

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
    CraterAmphitheater,
    LandmarkPlagueQuarter,
    QuarryPit,
    Roundhouse,
    LandmarkSubstation,
}

macro_rules! model {
    ($name:literal, $file:literal, $scale:literal, $datum:literal, $front:literal) => {
        Model {
            name: $name,
            path: concat!("models/sites/site-", $file, ".glb"),
            scale: $scale,
            datum: $datum,
            front: $front,
            front_angle: 0.0,
        }
    };
}

macro_rules! landmark_model {
    ($name:literal, $file:literal, $scale:literal, $datum:literal, $front:literal) => {
        Model {
            name: $name,
            path: concat!("models/landmarks/landmark-", $file, ".glb"),
            scale: $scale,
            datum: $datum,
            front: $front,
            front_angle: FRAC_PI_2,
        }
    };
}

struct Model {
    name: &'static str,
    path: &'static str,
    scale: f32,
    datum: f32,
    front: f32,
    front_angle: f32,
}

#[cfg(test)]
const SITE_PLACEMENT_COUNT: usize = 16;
#[cfg(test)]
const SUPPLIED_LANDMARK_COUNT: usize = 5;

const TRAIN_SCALE: f32 = 40.0;
const TRAIN_PAD: f32 = 32.0;
const DERAILED_RAIL_CENTER: Vec2 = Vec2::new(1420.0, -980.0);
const DERAILED_RAIL_JUNCTION: Vec2 = roads::SOUTH_CEMETERY;
const DERAILED_TRAIN_CENTER: Vec2 = Vec2::new(1436.5, -987.1);
const TRAIN_YAW_OFFSET: f32 = 0.18;
const TRAIN_ROLL: f32 = 0.10;
const TRAIN_LIFT: f32 = 0.45;

const MODELS: [Model; 13] = [
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
    landmark_model!(
        "Crater amphitheater",
        "crater-amphitheater",
        42.0,
        0.0,
        0.50
    ),
    landmark_model!(
        "Plague quarter landmark",
        "plague-quarter",
        34.0,
        0.0,
        0.234
    ),
    landmark_model!("Quarry pit", "quarry-pit", 48.0, 0.0, 0.50),
    landmark_model!("Roundhouse", "roundhouse", 34.0, 0.0, 0.362),
    landmark_model!("Abandoned substation", "substation", 24.0, 0.0, 0.406),
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
    include!("pillars/landmark_crater_amphitheater.rs"),
    include!("pillars/landmark_plague_quarter.rs"),
    include!("pillars/landmark_quarry_pit.rs"),
    include!("pillars/landmark_roundhouse.rs"),
    include!("pillars/landmark_substation.rs"),
];

const TRAIN_PILLARS: &[Pillar] = include!("pillars/vehicle_train.rs");

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
        .with_rotation(Quat::from_rotation_y(
            self.model().front_angle - facing.y.atan2(facing.x),
        ))
        .with_scale(Vec3::splat(scale))
    }
}

pub(super) const LANDMARKS: [Landmark; 25] = [
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
    // One fixed-seed scatter copy of each supplied landmark. Keeping the
    // coordinates fixed makes terrain, roads, and streaming deterministic.
    Landmark::new(
        Kind::CraterAmphitheater,
        -1505.0,
        166.0,
        roads::WEST_FACTORY,
    ),
    Landmark::new(
        Kind::LandmarkPlagueQuarter,
        -826.0,
        -988.0,
        roads::WEST_RAIL,
    ),
    Landmark::new(Kind::QuarryPit, 594.0, 1392.0, roads::NORTH_SILOS),
    Landmark::new(Kind::Roundhouse, 1392.0, 932.0, roads::NORTH_SILOS),
    Landmark::new(
        Kind::LandmarkSubstation,
        866.0,
        -1116.0,
        roads::SOUTH_SCHOOL,
    ),
    Landmark::new(Kind::Railway, -1430.0, 1180.0, roads::WEST_CEMETERY),
    Landmark::new(
        Kind::Railway,
        DERAILED_RAIL_CENTER.x,
        DERAILED_RAIL_CENTER.y,
        DERAILED_RAIL_JUNCTION,
    ),
    Landmark::new(Kind::LandmarkSubstation, -1450.0, -300.0, roads::WEST_RAIL),
    Landmark::new(
        Kind::LandmarkSubstation,
        1460.0,
        1360.0,
        roads::NORTHEAST_SUBSTATION,
    ),
];

pub(super) fn foundation(at: Vec2) -> Option<(Vec2, f32)> {
    let distance = at.distance_squared(DERAILED_TRAIN_CENTER);
    let outer = TRAIN_PAD + 26.0;
    let mut foundation = (distance < outer * outer).then(|| {
        (
            DERAILED_RAIL_CENTER,
            smoothstep(outer, TRAIN_PAD, distance.sqrt()),
        )
    });
    for site in LANDMARKS {
        let distance = at.distance_squared(site.center);
        let pad = site.pad();
        let outer = pad + 26.0;
        if distance >= outer * outer {
            continue;
        }
        let weight = smoothstep(outer, pad, distance.sqrt());
        if foundation.is_none_or(|(_, prior)| weight > prior) {
            foundation = Some((site.center, weight));
        }
    }
    foundation
}

pub(super) fn clear_footprint(at: Vec2, clearance: f32) -> bool {
    at.distance_squared(DERAILED_TRAIN_CENTER) < (TRAIN_PAD + clearance).powi(2)
        || LANDMARKS
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
    let (mesh, paint) = assets::load_single_primitive(&server, assets::VEHICLE_TRAIN);
    preloaded.0.extend([mesh.untyped(), paint.untyped()]);
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
    let (mesh, paint) = assets::load_single_primitive(&server, assets::VEHICLE_TRAIN);
    let paint = paints.get(&paint).expect("preloaded train material");
    commands.spawn((
        WorldRoot,
        Name::new("Derailed train"),
        Mesh3d(mesh),
        MeshMaterial3d(materials.add(standard_material_from_gltf_material(paint))),
        train_pose(&terrain),
        detail_range(385.0, 45.0),
    ));
}

pub(super) fn spawn_solids(parent: &mut ChildSpawnerCommands, terrain: &Terrain, tile: IVec2) {
    for site in LANDMARKS {
        if (site.center / TILE_SIZE).floor().as_ivec2() == tile {
            parent.spawn((site.pose(terrain), Collider(PILLARS[site.kind as usize])));
        }
    }
    if (DERAILED_TRAIN_CENTER / TILE_SIZE).floor().as_ivec2() == tile {
        parent.spawn((train_pose(terrain), Collider(TRAIN_PILLARS)));
    }
}

fn train_pose(terrain: &Terrain) -> Transform {
    let track = (DERAILED_RAIL_JUNCTION - DERAILED_RAIL_CENTER).normalize();
    let yaw = -track.y.atan2(track.x) + TRAIN_YAW_OFFSET;
    Transform::from_xyz(
        DERAILED_TRAIN_CENTER.x,
        terrain.mesh_height_at(DERAILED_TRAIN_CENTER) + TRAIN_LIFT,
        DERAILED_TRAIN_CENTER.y,
    )
    .with_rotation(Quat::from_rotation_y(yaw) * Quat::from_rotation_x(TRAIN_ROLL))
    .with_scale(Vec3::splat(TRAIN_SCALE))
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
                        "{:?} at {:?} pad tilts: origin {origin}, sample {:?}",
                        site.kind,
                        site.center,
                        at
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
        assert_eq!(count, [2, 2, 2, 2, 4, 2, 2, 2, 1, 1, 1, 1, 3]);
        assert!(
            quadrants.into_iter().all(|count| count >= 3),
            "{quadrants:?}"
        );
    }

    #[test]
    fn supplied_landmarks_are_a_sparse_four_quadrant_scatter() {
        let added =
            &LANDMARKS[SITE_PLACEMENT_COUNT..SITE_PLACEMENT_COUNT + SUPPLIED_LANDMARK_COUNT];
        let mut quadrants = [0; 4];
        for (i, landmark) in added.iter().enumerate() {
            assert!(
                LANDMARKS[..SITE_PLACEMENT_COUNT + i]
                    .iter()
                    .all(|other| other.center.distance(landmark.center) >= 400.0),
                "{:?} is too close to another structure",
                landmark.kind
            );
            assert!(
                added[..i]
                    .iter()
                    .map(|other| (other.center / TILE_SIZE).floor().as_ivec2())
                    .all(|tile| tile != (landmark.center / TILE_SIZE).floor().as_ivec2())
            );
            quadrants[usize::from(landmark.center.x > 0.0)
                + 2 * usize::from(landmark.center.y > 0.0)] += 1;
        }
        assert!(
            quadrants.into_iter().all(|count| count > 0),
            "{quadrants:?}"
        );
    }

    #[test]
    fn railway_and_substations_span_distinct_regions() {
        let railways: Vec<_> = LANDMARKS
            .iter()
            .filter(|site| site.kind == Kind::Railway)
            .collect();
        assert_eq!(railways.len(), 4);
        let mut rail_quadrants = [0; 4];
        for site in railways {
            rail_quadrants
                [usize::from(site.center.x > 0.0) + 2 * usize::from(site.center.y > 0.0)] += 1;
        }
        assert_eq!(rail_quadrants, [1; 4]);

        let substations: Vec<_> = LANDMARKS
            .iter()
            .filter(|site| site.kind == Kind::LandmarkSubstation)
            .collect();
        assert_eq!(substations.len(), 3);
        for (i, station) in substations.iter().enumerate() {
            assert!(
                LANDMARKS
                    .iter()
                    .filter(|other| other.kind != Kind::LandmarkSubstation)
                    .all(|other| other.center.distance(station.center) >= 400.0)
            );
            assert!(
                substations[..i]
                    .iter()
                    .all(|other| other.center.distance(station.center) >= 400.0)
            );
        }
    }

    #[test]
    fn the_train_is_grounded_beside_a_level_derailed_railway() {
        let terrain = Terrain::new();
        assert!((DERAILED_TRAIN_CENTER.distance(DERAILED_RAIL_CENTER) - 18.0).abs() < 0.1);
        let (foundation, weight) = super::foundation(DERAILED_TRAIN_CENTER).expect("train pad");
        assert_eq!(foundation, DERAILED_RAIL_CENTER);
        assert_eq!(weight, 1.0);

        let pose = train_pose(&terrain);
        assert!(pose.translation.y > terrain.mesh_height_at(DERAILED_TRAIN_CENTER));
        assert!(TRAIN_PILLARS.len() > 100);
        assert!(super::clear_footprint(DERAILED_TRAIN_CENTER, 0.0));
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
