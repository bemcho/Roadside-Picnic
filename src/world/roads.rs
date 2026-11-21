//! Graded landmark tracks and their shared visual and scenery masks.

use bevy::asset::RenderAssetUsages;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::cordon::GATE;
use super::hydrology::{EAST_FORD_X, FORD_X, WEST_FORD_X};
use super::sites::LANDMARKS;
use super::terrain::{BASE, TILE_SIZE, lerp, smoothstep};

const HALF_WIDTH: f32 = 8.5;
const SHOULDER: f32 = 11.0;
const QUARRY_JUNCTION: Vec2 = Vec2::ZERO;
const BALD_HILL_JUNCTION: Vec2 = Vec2::new(25.0, -33.0);
const WEST_JUNCTION: Vec2 = Vec2::new(-12.0, 122.0);
const SWAMP_JUNCTION: Vec2 = Vec2::new(-121.0, 138.0);
const WEST_TRUNK: Vec2 = Vec2::new(-308.0, 177.0);
const EAST_TRUNK: Vec2 = Vec2::new(18.0, 10.0);
const SOUTH_TRUNK: Vec2 = Vec2::new(508.0, 90.0);
const WEST_BYPASS: Vec2 = Vec2::new(-846.0, 190.0);
const LOWER_SOUTH: Vec2 = Vec2::new(570.0, -500.0);
const EAST_CROSSING: Vec2 = Vec2::new(EAST_FORD_X, 133.0);
const WEST_CROSSING: Vec2 = Vec2::new(WEST_FORD_X, 175.0);
pub(super) const WEST_GARAGE: Vec2 = Vec2::new(-172.0, -34.0);
pub(super) const SOUTH_GARAGE: Vec2 = Vec2::new(440.0, -463.0);
pub(super) const EAST_SILOS: Vec2 = Vec2::new(270.0, 65.0);
pub(super) const NORTH_SILOS: Vec2 = Vec2::new(881.0, 976.0);
pub(super) const WEST_FACTORY: Vec2 = Vec2::new(-1070.0, 112.0);
pub(super) const EAST_FACTORY: Vec2 = Vec2::new(1205.0, 160.0);
pub(super) const WEST_CEMETERY: Vec2 = Vec2::new(-1285.0, 854.0);
pub(super) const SOUTH_CEMETERY: Vec2 = Vec2::new(1330.0, -1190.0);
pub(super) const SOUTH_PLAGUE: Vec2 = Vec2::new(-292.0, -518.0);
pub(super) const NORTH_PLAGUE: Vec2 = Vec2::new(-900.0, 1180.0);
pub(super) const WEST_RAIL: Vec2 = Vec2::new(-1260.0, -730.0);
pub(super) const NORTH_RAIL: Vec2 = Vec2::new(822.0, 853.0);
pub(super) const NORTH_SCHOOL: Vec2 = Vec2::new(-216.0, 451.0);
pub(super) const SOUTH_SCHOOL: Vec2 = Vec2::new(1188.0, -665.0);
pub(super) const NORTH_SUBSTATION: Vec2 = Vec2::new(1110.0, 465.0);
pub(super) const SOUTH_SUBSTATION: Vec2 = Vec2::new(250.0, -1273.0);
pub(super) const NORTHEAST_SUBSTATION: Vec2 = Vec2::new(1500.0, 1200.0);

const ROUTES: &[&[Vec2]] = &[
    &[
        Vec2::new(BASE.0, BASE.1 + 18.0),
        GATE,
        BALD_HILL_JUNCTION,
        QUARRY_JUNCTION,
        Vec2::new(-8.0, 57.0),
        WEST_JUNCTION,
        Vec2::new(4.0, 164.0),
        Vec2::new(22.0, 202.0),
        Vec2::new(FORD_X, 245.0),
        Vec2::new(FORD_X, 408.0),
        Vec2::new(51.0, 456.0),
    ],
    &[QUARRY_JUNCTION, EAST_TRUNK, Vec2::new(36.0, 20.0)],
    &[
        BALD_HILL_JUNCTION,
        Vec2::new(9.0, -28.0),
        Vec2::new(-17.0, -27.0),
        Vec2::new(-40.0, -27.0),
        Vec2::new(-62.0, -23.0),
    ],
    &[
        WEST_JUNCTION,
        Vec2::new(-47.0, 131.0),
        Vec2::new(-84.0, 134.0),
        SWAMP_JUNCTION,
        Vec2::new(-178.0, 141.0),
        Vec2::new(-225.0, 149.0),
        Vec2::new(-270.0, 161.0),
        WEST_TRUNK,
        Vec2::new(-335.0, 196.0),
        Vec2::new(-355.0, 224.0),
    ],
    &[
        SWAMP_JUNCTION,
        Vec2::new(-140.0, 150.0),
        Vec2::new(-155.0, 172.0),
    ],
    &[
        EAST_TRUNK,
        Vec2::new(61.0, -18.0),
        Vec2::new(126.0, -24.0),
        Vec2::new(178.0, 0.0),
        Vec2::new(230.0, 32.0),
        EAST_SILOS,
        Vec2::new(376.0, 76.0),
        SOUTH_TRUNK,
        Vec2::new(642.0, 89.0),
        Vec2::new(842.0, 112.0),
        Vec2::new(1015.0, 118.0),
        EAST_CROSSING,
        EAST_FACTORY,
    ],
    &[
        SOUTH_TRUNK,
        Vec2::new(480.0, -81.0),
        Vec2::new(460.0, -250.0),
        SOUTH_GARAGE,
        LOWER_SOUTH,
        Vec2::new(700.0, -596.0),
        Vec2::new(910.0, -627.0),
        SOUTH_SCHOOL,
        Vec2::new(1280.0, -740.0),
        Vec2::new(1320.0, -890.0),
        SOUTH_CEMETERY,
    ],
    &[
        LOWER_SOUTH,
        Vec2::new(590.0, -730.0),
        Vec2::new(490.0, -910.0),
        Vec2::new(370.0, -1080.0),
        Vec2::new(280.0, -1235.0),
        SOUTH_SUBSTATION,
    ],
    &[
        WEST_TRUNK,
        Vec2::new(-440.0, 162.0),
        Vec2::new(-530.0, 168.0),
        WEST_CROSSING,
        Vec2::new(-730.0, 208.0),
        WEST_BYPASS,
        Vec2::new(-1000.0, 118.0),
        WEST_FACTORY,
    ],
    &[
        WEST_BYPASS,
        Vec2::new(-880.0, 65.0),
        Vec2::new(-910.0, -100.0),
        Vec2::new(-930.0, -300.0),
        Vec2::new(-1000.0, -532.0),
        Vec2::new(-1120.0, -630.0),
        WEST_RAIL,
    ],
    &[
        Vec2::new(-62.0, -23.0),
        Vec2::new(-118.0, -31.0),
        WEST_GARAGE,
        Vec2::new(-247.0, -62.0),
        Vec2::new(-280.0, -137.0),
        Vec2::new(-294.0, -239.0),
        Vec2::new(-296.0, -378.0),
        SOUTH_PLAGUE,
    ],
    &[
        WEST_CROSSING,
        Vec2::new(WEST_FORD_X, 400.0),
        Vec2::new(-708.0, 444.0),
        Vec2::new(-810.0, 495.0),
        Vec2::new(-900.0, 600.0),
        Vec2::new(-1060.0, 730.0),
        Vec2::new(-1280.0, 790.0),
        WEST_CEMETERY,
        Vec2::new(-1180.0, 990.0),
        Vec2::new(-1080.0, 1100.0),
        NORTH_PLAGUE,
    ],
    &[
        Vec2::new(FORD_X, 408.0),
        Vec2::new(-64.0, 420.0),
        Vec2::new(-145.0, 427.0),
        NORTH_SCHOOL,
    ],
    &[
        EAST_CROSSING,
        Vec2::new(EAST_FORD_X, 415.0),
        NORTH_SUBSTATION,
        Vec2::new(1000.0, 564.0),
        Vec2::new(942.0, 662.0),
        Vec2::new(875.0, 770.0),
        NORTH_RAIL,
        Vec2::new(859.0, 933.0),
        NORTH_SILOS,
    ],
    &[
        Vec2::new(EAST_FORD_X, 415.0),
        Vec2::new(1350.0, 550.0),
        Vec2::new(1450.0, 850.0),
        NORTHEAST_SUBSTATION,
    ],
];

#[derive(Clone)]
struct Leg {
    start: Vec2,
    along: Vec2,
    length: f32,
    levels: [f32; 2],
    min: Vec2,
    max: Vec2,
}

#[derive(Clone)]
pub(super) struct RoadNetwork {
    tiles: HashMap<IVec2, Vec<Leg>>,
    min: Vec2,
    max: Vec2,
}

pub(super) struct RoadSample {
    distance: f32,
    pub height: f32,
}

impl RoadSample {
    pub fn mask(&self) -> f32 {
        smoothstep(SHOULDER, HALF_WIDTH, self.distance)
    }

    fn surface(&self) -> [u8; 2] {
        let road = smoothstep(7.5, 4.6, self.distance);
        let rut = smoothstep(0.83, 0.32, (self.distance - 1.51).abs());
        [(road * 255.0).round() as u8, (rut * 255.0).round() as u8]
    }
}

impl RoadNetwork {
    pub fn new(level: impl Fn(Vec2) -> f32) -> Self {
        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        let mut tiles: HashMap<_, Vec<_>> = HashMap::default();
        let drives = LANDMARKS.map(|site| [site.junction, site.entrance()]);
        for path in ROUTES
            .iter()
            .copied()
            .chain(drives.iter().map(|path| path.as_slice()))
        {
            for &[a, b] in path.array_windows() {
                let delta = b - a;
                let length = delta.length();
                let leg = Leg {
                    start: a,
                    along: delta / length,
                    length,
                    levels: [level(a), level(b)],
                    min: a.min(b) - Vec2::splat(SHOULDER),
                    max: a.max(b) + Vec2::splat(SHOULDER),
                };
                min = min.min(leg.min);
                max = max.max(leg.max);
                let first = tile_at(leg.min);
                let last = tile_at(leg.max);
                for z in first.y..=last.y {
                    for x in first.x..=last.x {
                        tiles.entry(IVec2::new(x, z)).or_default().push(leg.clone());
                    }
                }
            }
        }
        Self { tiles, min, max }
    }

    pub fn sample(&self, at: Vec2) -> Option<RoadSample> {
        if at.cmplt(self.min).any() || at.cmpgt(self.max).any() {
            return None;
        }
        Self::sample_legs(self.tiles.get(&tile_at(at))?, at)
    }

    fn sample_legs(legs: &[Leg], at: Vec2) -> Option<RoadSample> {
        let mut nearest = None;
        let mut best = SHOULDER * SHOULDER;
        for leg in legs {
            if at.cmplt(leg.min).any() || at.cmpgt(leg.max).any() {
                continue;
            }
            let run = (at - leg.start).dot(leg.along).clamp(0.0, leg.length);
            let distance_squared = at.distance_squared(leg.start + leg.along * run);
            if distance_squared < best {
                nearest = Some((leg, run / leg.length));
                best = distance_squared;
            }
        }
        nearest.map(|(leg, t)| RoadSample {
            distance: best.sqrt(),
            height: lerp(leg.levels[0], leg.levels[1], t),
        })
    }

    pub fn surface_texture(&self) -> (Image, Vec4) {
        // A world-space mask keeps the same road width across coarse and fine terrain tiles.
        const TEXELS_PER_METRE: f32 = 1.5;
        let origin = self.min.floor();
        let size = ((self.max.ceil() - origin) * TEXELS_PER_METRE).as_uvec2();
        let mut pixels = vec![0; (size.x * size.y * 2) as usize];
        for (&tile, legs) in &self.tiles {
            let first = tile.as_vec2() * TILE_SIZE;
            let pixel = |at: Vec2| ((at - origin) * TEXELS_PER_METRE - 0.5).ceil().as_ivec2();
            let low = pixel(first).max(IVec2::ZERO);
            let high = pixel(first + TILE_SIZE).min(size.as_ivec2());
            for z in low.y..high.y {
                for x in low.x..high.x {
                    let at = origin + (Vec2::new(x as f32, z as f32) + 0.5) / TEXELS_PER_METRE;
                    if let Some(road) = Self::sample_legs(legs, at) {
                        let i = (z as usize * size.x as usize + x as usize) * 2;
                        pixels[i..i + 2].copy_from_slice(&road.surface());
                    }
                }
            }
        }
        let image = Image::new(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            TextureFormat::Rg8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        let uv_scale = Vec2::splat(TEXELS_PER_METRE) / size.as_vec2();
        (image, Vec4::new(origin.x, origin.y, uv_scale.x, uv_scale.y))
    }
}

fn tile_at(point: Vec2) -> IVec2 {
    (point / TILE_SIZE).floor().as_ivec2()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Terrain;

    #[test]
    fn every_landmark_track_has_a_continuous_drivable_route_back_to_the_gate() {
        let terrain = Terrain::new();
        assert!(ROUTES[0].contains(&GATE));
        let drives = LANDMARKS.map(|site| [site.junction, site.entrance()]);
        let mut bad = Vec::new();
        for (i, path) in ROUTES
            .iter()
            .copied()
            .chain(drives.iter().map(|path| path.as_slice()))
            .enumerate()
        {
            if i > 0 {
                assert!(
                    ROUTES[..i.min(ROUTES.len())]
                        .iter()
                        .any(|road| road.contains(&path[0]))
                );
            }
            for &[a, b] in path.array_windows() {
                let mut worst_grade = (0.0, a);
                let mut deepest = (0.0, a);
                let steps = a.distance(b).ceil() as usize;
                let across = (b - a).normalize().perp();
                for lane in [-2.4, 0.0, 2.4] {
                    let offset = across * lane;
                    let mut last = terrain.mesh_height_at(a + offset);
                    for step in 0..=steps {
                        let at = a.lerp(b, step as f32 / steps as f32) + offset;
                        assert!(terrain.road_mask(at.x, at.y) > 0.99);
                        let height = terrain.mesh_height_at(at);
                        let grade = (height - last).abs() * steps as f32 / a.distance(b);
                        if grade > worst_grade.0 {
                            worst_grade = (grade, at);
                        }
                        let depth = terrain.water_depth_at(at.x, at.y);
                        if depth > deepest.0 {
                            deepest = (depth, at);
                        }
                        last = height;
                    }
                }
                if worst_grade.0 > 0.57 || deepest.0 > 0.53 {
                    bad.push((i, worst_grade, deepest));
                }
            }
        }
        assert!(
            bad.is_empty(),
            "route, grade and position, depth and position: {bad:#?}"
        );
    }

    #[test]
    fn the_material_mask_draws_a_lane_and_two_ruts_without_painting_the_yard() {
        let terrain = Terrain::new();
        let (image, bounds) = terrain.roads.surface_texture();
        let read = |at: Vec2| {
            let uv = (at - bounds.xy()) * bounds.zw();
            let texel = (uv * Vec2::new(image.width() as f32, image.height() as f32))
                .floor()
                .as_uvec2();
            let i = ((texel.y * image.width() + texel.x) * 2) as usize;
            let bytes = image.data.as_ref().unwrap();
            (bytes[i], bytes[i + 1])
        };
        assert_eq!(read(GATE), (255, 0));
        assert_eq!(read(GATE + Vec2::new(-10.0, -4.0)), (0, 0));
        assert_eq!(read(Vec2::new(FORD_X, 300.0)), (255, 0));
        for side in [-1.0, 1.0] {
            assert!(read(Vec2::new(FORD_X + side * 1.5, 300.0)).1 >= 190);
            assert_eq!(read(Vec2::new(FORD_X + side * 8.0, 300.0)), (0, 0));
        }
        for &path in ROUTES {
            assert!(path.iter().all(|&at| read(at).0 == 255));
        }
    }
}
