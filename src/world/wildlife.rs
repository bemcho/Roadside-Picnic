//! Sparse, decorative wildlife using shared skinned meshes.

mod movement;
mod mutants;
mod rig;
mod sound;

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::pbr::gltf::standard_material_from_gltf_material;
use bevy::prelude::*;

use crate::assets::{self, PreloadedAssets};

use movement::{Flight, Wildlife, dry_ground};
use rig::{Rig, wing_mesh};

use super::terrain::{BASE, HALF_SIZE, Terrain};
use super::{WorldRoot, detail_range};

pub(super) use movement::move_wildlife;
pub(super) use mutants::move_mutants;
pub(super) use rig::pose_wildlife;
pub(crate) use sound::{CreatureSound, Cue};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    Frostwolf,
    Hexadeer,
    Skyglider,
    Chameleon,
    Hawk,
    Heron,
    Owl,
    Raven,
    Sparrow,
    Bovine,
    Chimera,
    Fleshdeer,
    Mutt,
}

impl Kind {
    pub(crate) const ALL: [Self; 13] = [
        Self::Frostwolf,
        Self::Hexadeer,
        Self::Skyglider,
        Self::Chameleon,
        Self::Hawk,
        Self::Heron,
        Self::Owl,
        Self::Raven,
        Self::Sparrow,
        Self::Bovine,
        Self::Chimera,
        Self::Fleshdeer,
        Self::Mutt,
    ];

    pub(crate) fn is_bird(self) -> bool {
        matches!(
            self,
            Self::Skyglider | Self::Hawk | Self::Heron | Self::Owl | Self::Raven | Self::Sparrow
        )
    }

    fn model(self) -> (&'static str, f32, f32) {
        match self {
            Self::Frostwolf => (assets::FROSTWOLF, 2.45, 0.0),
            Self::Hexadeer => (assets::HEXADEER, 1.65, -FRAC_PI_2),
            Self::Skyglider => (assets::SKYGLIDER, 1.75, PI),
            Self::Chameleon => (assets::VENOM_CHAMELEON, 0.95, -FRAC_PI_2),
            Self::Hawk => (assets::HAWK, 1.23, FRAC_PI_2),
            Self::Heron => (assets::HERON, 1.02, -FRAC_PI_2),
            Self::Owl => (assets::OWL, 0.55, FRAC_PI_2),
            Self::Raven => (assets::RAVEN, 0.88, 0.0),
            Self::Sparrow => (assets::SPARROW, 0.38, FRAC_PI_2),
            Self::Bovine => (assets::BOVINE, 3.25, FRAC_PI_2),
            Self::Chimera => (assets::CHIMERA, 3.10, 0.96),
            Self::Fleshdeer => (assets::FLESHDEER, 2.45, -0.34),
            Self::Mutt => (assets::MUTT, 1.92, -0.94),
        }
    }
}

pub(super) fn preload_wildlife(server: Res<AssetServer>, mut preloaded: ResMut<PreloadedAssets>) {
    for kind in Kind::ALL {
        let (mesh, paint) = assets::load_single_primitive(&server, kind.model().0);
        preloaded.0.extend([mesh.untyped(), paint.untyped()]);
    }
}

#[derive(Clone, Copy)]
struct Route {
    center: Vec2,
    radius: Vec2,
    phase: f32,
    speed: f32,
}

impl Route {
    fn sample(self, time: f32) -> (Vec2, Vec2) {
        let (sin, cos) = (self.phase + time * self.speed).sin_cos();
        (
            self.center + self.radius * Vec2::new(cos, sin),
            self.radius * Vec2::new(-sin, cos),
        )
    }

    fn circuit(self) -> impl Iterator<Item = Vec2> {
        (0..24).map(move |i| self.sample(i as f32 * TAU / (24.0 * self.speed)).0)
    }
}

struct WildlifeModels {
    bodies: [(Rig, Handle<StandardMaterial>); Kind::ALL.len()],
    wings: [Handle<Mesh>; 2],
    feathers: [Handle<StandardMaterial>; 2],
}

pub(super) fn spawn_wildlife(
    mut commands: Commands,
    server: Res<AssetServer>,
    terrain: Res<Terrain>,
    paints: Res<Assets<bevy::gltf::GltfMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut binds: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    let models = WildlifeModels {
        bodies: Kind::ALL.map(|kind| {
            let (mesh, paint) = assets::load_single_primitive(&server, kind.model().0);
            let paint = paints
                .get(&paint)
                .expect("wildlife materials are preloaded before the Zone opens");
            let mesh = meshes.get(&mesh).expect("preloaded wildlife mesh").clone();
            (
                Rig::build(kind, mesh, &mut meshes, &mut binds),
                materials.add(standard_material_from_gltf_material(paint)),
            )
        }),
        wings: [-1.0, 1.0].map(|side| meshes.add(wing_mesh(side))),
        feathers: [Color::srgb(0.12, 0.13, 0.16), Color::srgb(0.33, 0.24, 0.16)].map(|color| {
            materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.94,
                cull_mode: None,
                double_sided: true,
                ..default()
            })
        }),
    };
    commands
        .spawn((WorldRoot, Transform::IDENTITY, Visibility::default()))
        .with_children(|wildlife| {
            for (kind, route) in ground_routes(&terrain) {
                spawn_creature(wildlife, &models, &terrain, kind, route, None);
            }
            for (kind, route, flight) in flight_routes(&terrain) {
                spawn_creature(wildlife, &models, &terrain, kind, route, Some(flight));
            }
            for (kind, route) in mutants::routes(&terrain) {
                spawn_creature(wildlife, &models, &terrain, kind, route, None)
                    .insert(mutants::Mutant::new(kind, route));
            }
        });
}

fn ground_routes(terrain: &Terrain) -> Vec<(Kind, Route)> {
    let base = Vec2::from(BASE);
    [
        (Kind::Frostwolf, base + Vec2::new(-32.0, 29.0)),
        (Kind::Hexadeer, base + Vec2::new(-29.0, 35.0)),
        (Kind::Frostwolf, Vec2::new(-237.0, -89.0)),
        (Kind::Frostwolf, Vec2::new(270.0, 165.0)),
        (Kind::Hexadeer, Vec2::new(254.0, 160.0)),
        (Kind::Hexadeer, Vec2::new(-386.0, 453.0)),
        (Kind::Chameleon, Vec2::new(-54.0, 115.0)),
        (Kind::Chameleon, Vec2::new(175.0, 401.0)),
        (Kind::Heron, Vec2::new(-73.0, 63.0)),
        (Kind::Heron, Vec2::new(15.0, 142.0)),
        (Kind::Owl, Vec2::new(-162.0, 54.0)),
        (Kind::Owl, Vec2::new(179.0, -257.0)),
        (Kind::Hexadeer, Vec2::new(1120.0, -470.0)),
        (Kind::Frostwolf, Vec2::new(-1180.0, -770.0)),
        (Kind::Hexadeer, Vec2::new(-1310.0, 950.0)),
        (Kind::Chameleon, Vec2::new(935.0, 420.0)),
        (Kind::Hexadeer, Vec2::new(710.0, 1440.0)),
        (Kind::Frostwolf, Vec2::new(1370.0, 480.0)),
        (Kind::Hexadeer, Vec2::new(HALF_SIZE + 47.0, 60.0)),
        (Kind::Frostwolf, Vec2::new(-HALF_SIZE - 54.0, -550.0)),
        (Kind::Hexadeer, Vec2::new(440.0, HALF_SIZE + 52.0)),
        (Kind::Heron, Vec2::new(-250.0, -HALF_SIZE - 44.0)),
    ]
    .into_iter()
    .enumerate()
    .filter_map(|(i, (kind, center))| {
        let outside = center.abs().max_element() > HALF_SIZE;
        for offset in std::iter::once(Vec2::ZERO).chain(
            (0..16).map(|j| Vec2::from_angle(j as f32 * TAU / 8.0) * (8.0 + (j / 8) as f32 * 8.0)),
        ) {
            let (radius, speed) = match kind {
                Kind::Frostwolf => (Vec2::new(6.6, 4.1), 0.18),
                Kind::Hexadeer => (Vec2::new(5.2, 4.0), 0.15),
                Kind::Chameleon => (Vec2::new(1.7, 1.4), 0.18),
                Kind::Heron => (Vec2::new(3.0, 2.3), 0.14),
                Kind::Owl => (Vec2::new(0.8, 0.58), 0.30),
                _ => (Vec2::splat(0.16), 0.17),
            };
            let route = Route {
                center: center + offset,
                radius,
                phase: i as f32 * 1.63,
                speed,
            };
            if route.circuit().all(|at| {
                let distance = at.abs().max_element();
                let within_range = if outside {
                    (HALF_SIZE + 12.0..HALF_SIZE + 95.0).contains(&distance)
                } else {
                    distance < HALF_SIZE - 4.0
                };
                within_range && dry_ground(terrain, at) && terrain.road_mask(at.x, at.y) < 0.025
            }) {
                return Some((kind, route));
            }
        }
        None
    })
    .collect()
}

fn flight_routes(terrain: &Terrain) -> Vec<(Kind, Route, Flight)> {
    let base = Vec2::from(BASE);
    let lanes: [(Vec2, Vec2, f32, f32, &[Kind]); 8] = [
        (
            base + Vec2::new(4.0, 48.0),
            Vec2::new(19.0, 11.0),
            12.0,
            0.35,
            &[Kind::Raven],
        ),
        (
            Vec2::new(-31.0, 174.0),
            Vec2::new(56.0, 39.0),
            48.0,
            0.18,
            &[Kind::Hawk, Kind::Raven, Kind::Hawk],
        ),
        (
            Vec2::new(276.0, -90.0),
            Vec2::new(63.0, 48.0),
            52.0,
            0.17,
            &[Kind::Skyglider, Kind::Skyglider, Kind::Hawk],
        ),
        (
            Vec2::new(-305.0, 405.0),
            Vec2::new(45.0, 29.0),
            20.0,
            0.34,
            &[Kind::Sparrow, Kind::Sparrow, Kind::Sparrow],
        ),
        (
            Vec2::new(-250.0, -HALF_SIZE - 135.0),
            Vec2::new(56.0, 26.0),
            27.0,
            0.18,
            &[Kind::Hawk, Kind::Raven],
        ),
        (
            Vec2::new(-HALF_SIZE - 155.0, 333.0),
            Vec2::new(39.0, 68.0),
            38.0,
            0.17,
            &[Kind::Skyglider, Kind::Hawk],
        ),
        (
            Vec2::new(HALF_SIZE + 130.0, 75.0),
            Vec2::new(31.0, 66.0),
            47.0,
            0.19,
            &[Kind::Hawk, Kind::Raven],
        ),
        (
            Vec2::new(465.0, HALF_SIZE + 120.0),
            Vec2::new(60.0, 45.0),
            44.0,
            0.19,
            &[Kind::Skyglider, Kind::Hawk],
        ),
    ];
    let mut routes = Vec::new();
    for (lane, (center, radius, clearance, speed, birds)) in lanes.into_iter().enumerate() {
        for (i, &kind) in birds.iter().enumerate() {
            let mut route = Route {
                center,
                radius: radius + Vec2::splat(3.0 * i as f32),
                phase: -0.9 + 0.12 * i as f32,
                speed,
            };
            let mut lands = false;
            if lane == 0 {
                let perches = [
                    Vec2::new(21.0, 46.0),
                    Vec2::new(-24.0, 49.0),
                    Vec2::new(16.0, 60.0),
                ];
                if let Some(landing) = perches
                    .into_iter()
                    .map(|offset| Route {
                        center: base + offset,
                        radius: Vec2::new(9.0, 7.0),
                        speed: 0.42,
                        ..route
                    })
                    .find(|route| {
                        route.circuit().all(|at| {
                            dry_ground(terrain, at) && terrain.road_mask(at.x, at.y) < 0.025
                        })
                    })
                {
                    route = landing;
                    lands = true;
                }
            }
            let top = route
                .circuit()
                .map(|at| terrain.mesh_height_at(at))
                .fold(f32::NEG_INFINITY, f32::max);
            routes.push((
                kind,
                route,
                Flight {
                    cruise: top + clearance + 2.4 * i as f32,
                    lands,
                },
            ));
        }
    }
    routes
}

fn spawn_creature<'a>(
    parent: &'a mut ChildSpawnerCommands,
    models: &WildlifeModels,
    terrain: &Terrain,
    kind: Kind,
    route: Route,
    flight: Option<Flight>,
) -> EntityCommands<'a> {
    let motion = Wildlife::new(kind, route, flight);
    let pose = motion.initial_pose(terrain);
    let distance = if flight.is_some() {
        470.0
    } else if mutants::is_mutant(kind) {
        mutants::VIEW_DISTANCE
    } else {
        100.0
    };
    let (model, paint) = &models.bodies[kind as usize];
    let mut entity = parent.spawn((pose, Visibility::default(), Name::new(kind.model().0)));
    let mut articulation = model.spawn(&mut entity, &motion, paint, distance);
    if flight.is_some() && matches!(kind, Kind::Raven | Kind::Sparrow) {
        let feathers = &models.feathers[usize::from(kind == Kind::Sparrow)];
        articulation.spawn_feathers(&mut entity, &models.wings, feathers);
    }
    entity.insert((motion, articulation));
    entity
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildlife_includes_hangar_patrols_and_distant_flocks() {
        let terrain = Terrain::new();
        let base = Vec2::from(BASE);
        let ground = ground_routes(&terrain);
        for kind in [Kind::Frostwolf, Kind::Hexadeer] {
            assert!(
                ground.iter().any(|(s, route)| {
                    *s == kind && route.circuit().all(|at| at.distance(base) < 60.0)
                }),
                "no {kind:?} patrol near the hangar"
            );
        }

        for kind in [Kind::Chameleon, Kind::Heron, Kind::Owl] {
            assert!(
                ground.iter().any(|(s, _)| *s == kind),
                "no {kind:?} in the Zone"
            );
        }
        assert!(ground.len() <= 22, "avoid crowding the Zone");

        let flights = flight_routes(&terrain);
        assert!(flights.iter().any(|(kind, route, _)| {
            *kind == Kind::Raven && route.circuit().all(|at| at.distance(base) < 75.0)
        }));
        for kind in [Kind::Hawk, Kind::Skyglider, Kind::Sparrow] {
            assert!(flights.iter().any(|(s, route, _)| {
                *s == kind && route.circuit().all(|at| at.distance(base) > 160.0)
            }));
        }
        assert!(flights.iter().any(|(kind, route, flight)| {
            *kind == Kind::Raven
                && flight.lands
                && route.circuit().all(|at| dry_ground(&terrain, at))
        }));
        for (_, route, flight) in flights {
            assert!(
                route
                    .circuit()
                    .all(|at| flight.cruise - terrain.mesh_height_at(at) >= 11.9)
            );
        }
    }

    #[test]
    fn every_border_has_ground_wildlife_and_flying_birds_beyond_it() {
        let terrain = Terrain::new();
        let ground = ground_routes(&terrain);
        let flights = flight_routes(&terrain);
        for direction in [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y] {
            let outside = |at: Vec2| at.dot(direction) > HALF_SIZE;
            assert!(
                ground.iter().any(|(_, route)| route.circuit().all(outside)),
                "no border animal at {direction}"
            );
            assert!(
                flights
                    .iter()
                    .any(|(_, route, _)| route.circuit().all(outside)),
                "no border birds at {direction}"
            );
        }
    }
}
