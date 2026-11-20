//! Procedural terrain for the Zone.
//!
//! Topography inspired by the Zone from *Roadside Picnic*: rolling, overgrown
//! hills around Harmont, the old quarry where prospectors dig, a scorched
//! bald hill, a marshy lowland, and the worn dirt road stalkers take in.
//!
//! Everything is deterministic — same seed, same Zone on every run.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// World-space extent of the terrain, in units, centered on the origin.
const SIZE: f32 = 300.0;

/// Distance from the origin to each terrain edge along X and Z.
pub const HALF_SIZE: f32 = SIZE / 2.0;

/// Number of grid cells per side (vertices = cells + 1).
const CELLS: usize = 300;

/// Fixed noise seed.
const SEED: u32 = 0x1972_2016;

/// The quarry: an elliptical pit dug out of the hills.
const QUARRY_CENTER: (f32, f32) = (70.0, 40.0);
const QUARRY_RADII: (f32, f32) = (45.0, 30.0);
const QUARRY_DEPTH: f32 = 14.0;

/// The bald hill: a scorched, bare mound.
const BALD_CENTER: (f32, f32) = (-75.0, -55.0);
const BALD_HEIGHT: f32 = 12.0;
const BALD_SIGMA: f32 = 22.0;

/// The marsh: a broad, stagnant lowland.
const MARSH_CENTER: (f32, f32) = (-35.0, 78.0);
const MARSH_RADIUS: f32 = 48.0;
const MARSH_DEPTH: f32 = 2.5;

/// The road: from the spawn point to the near rim of the quarry, where
/// stalkers stop and look down into the pit.
const ROAD_START: (f32, f32) = (0.0, 0.0);
const ROAD_END: (f32, f32) = (36.0, 20.0);
const ROAD_HALF_WIDTH: f32 = 5.0;

/// The stalkers' base: an old industrial hangar on the south edge, on a
/// levelled pad (wide enough for the whole hall) that fades back into the hills.
pub const BASE: (f32, f32) = (40.0, -128.0);
const BASE_PAD_RADIUS: f32 = 27.0;
const BASE_PAD_BLEND: f32 = 12.0;

/// Where the stalker starts and respawns: inside the hangar, by its door.
pub const SPAWN: (f32, f32) = (40.0, -114.0);

/// Deterministic terrain model. Pure math until [`Terrain::build_mesh`] is
/// called — insert as a resource so any system can sample the ground.
#[derive(Resource)]
pub struct Terrain {
    spacing: f32,
}

impl Terrain {
    pub fn new() -> Self {
        Self {
            spacing: SIZE / CELLS as f32,
        }
    }

    /// Height of the terrain surface at world position `(x, z)`.
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let mut h = self.hills(x, z);

        // Bald hill: a smooth scorched mound.
        let bdx = x - BALD_CENTER.0;
        let bdz = z - BALD_CENTER.1;
        let bd2 = bdx * bdx + bdz * bdz;
        h += BALD_HEIGHT * (-bd2 / (2.0 * BALD_SIGMA * BALD_SIGMA)).exp();

        // Marsh: broad, nearly flat depression.
        let mdx = (x - MARSH_CENTER.0) / MARSH_RADIUS;
        let mdz = (z - MARSH_CENTER.1) / MARSH_RADIUS;
        let marsh_d = (mdx * mdx + mdz * mdz).sqrt();
        h -= MARSH_DEPTH * smoothstep(1.0, 0.35, marsh_d);

        // Quarry: excavated bowl with a raised lip and micro-relief on the floor.
        let qdx = (x - QUARRY_CENTER.0) / QUARRY_RADII.0;
        let qdz = (z - QUARRY_CENTER.1) / QUARRY_RADII.1;
        let quarry_d = (qdx * qdx + qdz * qdz).sqrt();
        let cut = QUARRY_DEPTH * smoothstep(1.08, 0.64, quarry_d);
        let lip = 0.9 * (-((quarry_d - 1.08) / 0.13).powi(2)).exp();
        let floor_noise =
            (value_noise(x * 0.3, z * 0.3, SEED + 11) - 0.5) * 0.9 * smoothstep(0.9, 0.2, quarry_d);
        h += -cut + lip + floor_noise;

        // Road: the track is flattened toward a gentle ramp between its ends.
        let road_mask = self.road_mask(x, z);
        let t = self.road_progress(x, z);
        let ramp_h = self.hills(ROAD_START.0, ROAD_START.1) * (1.0 - t)
            + self.hills(ROAD_END.0, ROAD_END.1) * t;
        h += road_mask * (ramp_h - h) * 0.85;

        // Base: levelled flat for the yard.
        h += self.base_mask(x, z) * (self.hills(BASE.0, BASE.1) - h);

        h
    }

    /// 1.0 on the base's levelled pad, fading to 0.0 around it.
    fn base_mask(&self, x: f32, z: f32) -> f32 {
        let distance = Vec2::new(x - BASE.0, z - BASE.1).length();
        smoothstep(BASE_PAD_RADIUS + BASE_PAD_BLEND, BASE_PAD_RADIUS, distance)
    }

    /// Rolling hills: fractal value noise, no external crates.
    fn hills(&self, x: f32, z: f32) -> f32 {
        6.5 * fbm(x, z, SEED, 3, 1.0 / 48.0)
    }

    /// 1.0 on the road centerline, 0.0 away from it.
    fn road_mask(&self, x: f32, z: f32) -> f32 {
        let (sx, sz) = ROAD_START;
        let (ex, ez) = ROAD_END;
        let dx = ex - sx;
        let dz = ez - sz;
        let len2 = dx * dx + dz * dz;
        let t = ((x - sx) * dx + (z - sz) * dz) / len2;
        let t = t.clamp(0.0, 1.0);
        let px = sx + t * dx;
        let pz = sz + t * dz;
        let dist = ((x - px) * (x - px) + (z - pz) * (z - pz)).sqrt();
        smoothstep(ROAD_HALF_WIDTH + 6.0, ROAD_HALF_WIDTH, dist)
    }

    /// 0.0 at the road start, 1.0 at the road end (clamped along the axis).
    fn road_progress(&self, x: f32, z: f32) -> f32 {
        let (sx, sz) = ROAD_START;
        let (ex, ez) = ROAD_END;
        let dx = ex - sx;
        let dz = ez - sz;
        (((x - sx) * dx + (z - sz) * dz) / (dx * dx + dz * dz)).clamp(0.0, 1.0)
    }

    fn normal_at(&self, x: f32, z: f32) -> Vec3 {
        let e = 0.25;
        let hx = (self.height_at(x + e, z) - self.height_at(x - e, z)) / (2.0 * e);
        let hz = (self.height_at(x, z + e) - self.height_at(x, z - e)) / (2.0 * e);
        Vec3::new(-hx, 1.0, -hz).normalize()
    }

    /// How readily trees grow at `(x, z)`, from 0.0 (never) to 1.0: none on
    /// the road, in the quarry, at the base, or on steep ground; few on the
    /// scorched hill.
    pub fn woodland_at(&self, x: f32, z: f32) -> f32 {
        let qdx = (x - QUARRY_CENTER.0) / QUARRY_RADII.0;
        let qdz = (z - QUARRY_CENTER.1) / QUARRY_RADII.1;
        let outside_quarry = smoothstep(1.1, 1.3, (qdx * qdx + qdz * qdz).sqrt());

        let bdx = x - BALD_CENTER.0;
        let bdz = z - BALD_CENTER.1;
        let ash = (-(bdx * bdx + bdz * bdz) / (2.0 * BALD_SIGMA * BALD_SIGMA)).exp();

        let normal = self.normal_at(x, z);
        let flat_enough = smoothstep(0.7, 0.4, normal.xz().length() / normal.y);
        let off_road = 1.0 - smoothstep(0.05, 0.3, self.road_mask(x, z));

        let off_base = 1.0 - self.base_mask(x, z);

        outside_quarry * flat_enough * off_road * off_base * (1.0 - 0.85 * ash)
    }

    /// Shader data, not a tint: exposed earth, wetness, ash, macro variation.
    fn surface_weights_at(&self, x: f32, z: f32, h: f32, slope: f32) -> [f32; 4] {
        let qdx = (x - QUARRY_CENTER.0) / QUARRY_RADII.0;
        let qdz = (z - QUARRY_CENTER.1) / QUARRY_RADII.1;
        let quarry = smoothstep(1.12, 0.85, (qdx * qdx + qdz * qdz).sqrt());

        let mdx = (x - MARSH_CENTER.0) / MARSH_RADIUS;
        let mdz = (z - MARSH_CENTER.1) / MARSH_RADIUS;
        let marsh = smoothstep(1.0, 0.6, (mdx * mdx + mdz * mdz).sqrt());

        let bdx = x - BALD_CENTER.0;
        let bdz = z - BALD_CENTER.1;
        let ash = (-(bdx * bdx + bdz * bdz) / (2.0 * BALD_SIGMA * BALD_SIGMA)).exp();

        let exposed = smoothstep(0.35, 0.85, slope)
            .max(self.road_mask(x, z))
            .max(quarry)
            .max(ash * 0.95)
            .max(self.base_mask(x, z));
        let wetness = marsh.max(smoothstep(2.5, -1.0, h) * 0.3);
        let variation = lerp(0.82, 1.12, value_noise(x * 0.08, z * 0.08, SEED + 5));
        [exposed, wetness, ash, variation]
    }

    /// Shared grid vertices with continuous height-field normals and biome weights.
    pub fn build_mesh(&self) -> Mesh {
        let n = CELLS + 1;
        let mut positions = Vec::with_capacity(n * n);
        let mut normals = Vec::with_capacity(n * n);
        let mut colors = Vec::with_capacity(n * n);

        for iz in 0..n {
            let z = -SIZE / 2.0 + iz as f32 * self.spacing;
            for ix in 0..n {
                let x = -SIZE / 2.0 + ix as f32 * self.spacing;
                let h = self.height_at(x, z);
                let normal = self.normal_at(x, z);
                let slope = normal.xz().length() / normal.y;
                positions.push([x, h, z]);
                normals.push(normal.to_array());
                colors.push(self.surface_weights_at(x, z, h, slope));
            }
        }

        let mut indices = Vec::with_capacity(CELLS * CELLS * 6);
        for iz in 0..CELLS {
            for ix in 0..CELLS {
                let a = (iz * n + ix) as u32;
                let b = a + 1;
                let c = a + n as u32;
                let d = c + 1;
                indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_indices(Indices::U32(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh
    }
}

impl Default for Terrain {
    fn default() -> Self {
        Self::new()
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Deterministic integer-lattice hash in `[0, 1)`.
pub(super) fn lattice_hash(ix: i32, iz: i32, seed: u32) -> f32 {
    let mut h = (ix as u32).wrapping_mul(0x8da6_b343)
        ^ (iz as u32).wrapping_mul(0xd816_3841)
        ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h as f32 / u32::MAX as f32
}

/// Bilinearly interpolated value noise in `[0, 1)`.
pub(super) fn value_noise(x: f32, z: f32, seed: u32) -> f32 {
    let ix = x.floor();
    let iz = z.floor();
    let fx = x - ix;
    let fz = z - iz;
    let (ix, iz) = (ix as i32, iz as i32);

    let a = lattice_hash(ix, iz, seed);
    let b = lattice_hash(ix + 1, iz, seed);
    let c = lattice_hash(ix, iz + 1, seed);
    let d = lattice_hash(ix + 1, iz + 1, seed);

    // Quintic interpolation keeps curvature continuous across lattice boundaries.
    let fade = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let ux = fade(fx);
    let uz = fade(fz);
    let ab = a + (b - a) * ux;
    let cd = c + (d - c) * ux;
    ab + (cd - ab) * uz
}

/// Fractal Brownian motion: octaves of value noise, normalized to `[-1, 1]`.
fn fbm(x: f32, z: f32, seed: u32, octaves: u32, base_freq: f32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 1.0;
    let mut norm = 0.0;
    let mut freq = base_freq;

    for octave in 0..octaves {
        // Per-octave offsets decorrelate the lattice sampling.
        let ox = octave as f32 * 7.13;
        let oz = octave as f32 * 3.17;
        sum += amp * (value_noise(x * freq + ox, z * freq + oz, seed) * 2.0 - 1.0);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }

    sum / norm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_is_deterministic() {
        let terrain = Terrain::new();
        let a = terrain.height_at(12.3, -45.6);
        let b = terrain.height_at(12.3, -45.6);
        assert_eq!(a, b);
    }

    #[test]
    fn value_noise_is_in_unit_range() {
        for i in 0..100 {
            let v = value_noise(i as f32 * 0.7, -i as f32 * 0.3, SEED);
            assert!((0.0..1.0).contains(&v), "value {v} out of range");
        }
    }

    #[test]
    fn quarry_floor_is_deeper_than_rim() {
        let terrain = Terrain::new();
        // Just inside the quarry wall vs. just outside it.
        let floor = terrain.height_at(QUARRY_CENTER.0 + 0.64 * QUARRY_RADII.0, QUARRY_CENTER.1);
        let rim = terrain.height_at(QUARRY_CENTER.0 + 1.05 * QUARRY_RADII.0, QUARRY_CENTER.1);
        assert!(floor < rim - 5.0, "floor {floor} vs rim {rim}");
    }

    #[test]
    fn road_is_flatter_than_its_surroundings() {
        let terrain = Terrain::new();
        let mid = (30.0, 17.3); // on the road centerline
        let off = (30.0, 35.0); // well off the road
        let normal_on = terrain.normal_at(mid.0, mid.1);
        let normal_off = terrain.normal_at(off.0, off.1);
        let slope_on = normal_on.xz().length() / normal_on.y;
        let slope_off = normal_off.xz().length() / normal_off.y;
        assert!(
            slope_on < slope_off * 0.5,
            "road slope {slope_on} vs {slope_off}"
        );
    }

    #[test]
    fn road_entrance_meets_the_surrounding_hills() {
        let terrain = Terrain::new();
        let road_height = terrain.height_at(ROAD_START.0, ROAD_START.1);
        let hill_height = terrain.hills(ROAD_START.0, ROAD_START.1);
        assert!(
            (road_height - hill_height).abs() < 0.1,
            "road entrance should not form a raised platform"
        );
    }

    #[test]
    fn mesh_has_expected_vertex_count() {
        let terrain = Terrain::new();
        let mesh = terrain.build_mesh();
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("positions present");
        assert_eq!(positions.len(), (CELLS + 1) * (CELLS + 1));
        assert_eq!(
            mesh.indices().expect("shared grid indices").len(),
            CELLS * CELLS * 6
        );
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("smooth normals present");
        };
        for normal in normals {
            let normal = Vec3::from_array(*normal);
            assert!((normal.length() - 1.0).abs() < 1e-5);
            assert!(normal.y > 0.0);
        }
    }

    #[test]
    fn terrain_normals_are_continuous_across_noise_and_quarry_boundaries() {
        let terrain = Terrain::new();
        for (x, z) in [
            (48.0, -48.0),
            (0.0, 0.0),
            (BALD_CENTER.0, BALD_CENTER.1),
            (QUARRY_CENTER.0 + QUARRY_RADII.0 * 1.08, QUARRY_CENTER.1),
            (QUARRY_CENTER.0 + QUARRY_RADII.0 * 0.64, QUARRY_CENTER.1),
        ] {
            for offset in [Vec2::X, Vec2::Y] {
                let before = terrain.normal_at(x - offset.x * 0.01, z - offset.y * 0.01);
                let after = terrain.normal_at(x + offset.x * 0.01, z + offset.y * 0.01);
                assert!(before.dot(after) > 0.999, "normal seam near ({x}, {z})");
            }
        }
    }

    #[test]
    fn surface_layers_blend_continuously_with_slope_and_elevation() {
        let terrain = Terrain::new();
        for slope in [0.35, 0.55, 0.85] {
            let before = terrain.surface_weights_at(100.0, -100.0, 1.499, slope - 0.001);
            let after = terrain.surface_weights_at(100.0, -100.0, 1.501, slope + 0.001);
            for channel in 0..4 {
                assert!((before[channel] - after[channel]).abs() < 0.01);
            }
        }
    }

    #[test]
    fn base_stands_on_a_level_pad_clear_of_trees() {
        let terrain = Terrain::new();
        let level = terrain.height_at(BASE.0, BASE.1);
        for (dx, dz) in [(-17.5, -19.5), (17.5, 19.5), (0.0, 22.0), (-10.0, 12.0)] {
            let h = terrain.height_at(BASE.0 + dx, BASE.1 + dz);
            assert!((h - level).abs() < 1e-4, "pad not level at ({dx}, {dz})");
            assert_eq!(terrain.woodland_at(BASE.0 + dx, BASE.1 + dz), 0.0);
        }
    }
}
