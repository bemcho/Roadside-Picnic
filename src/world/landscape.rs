use std::convert::Infallible;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;

use crate::assets::PreloadedAssets;

use super::scenery::{self, Scenery, Sites};
use super::terrain::{TILE_RADIUS, Terrain};
use super::terrain_material::TerrainMaterial;
use super::water::{self, WaterMaterial};

struct TerrainTile {
    ground: Mesh,
    water: Option<Mesh>,
    forest: Option<Mesh>,
    stone: Option<Mesh>,
}

struct TileMeshes {
    ground: Handle<Mesh>,
    water: Option<Handle<Mesh>>,
    forest: Option<Handle<Mesh>>,
    stone: Option<Handle<Mesh>>,
}

#[derive(Asset, TypePath)]
pub(super) struct Landscape {
    sites: HashMap<IVec2, Sites>,
    pending: Vec<TerrainTile>,
    meshes: Vec<TileMeshes>,
}

#[derive(Resource)]
pub(super) struct LandscapeSource(pub Handle<Landscape>);

pub(super) fn prepare_landscape(
    mut commands: Commands,
    server: Res<AssetServer>,
    terrain: Res<Terrain>,
    mut preloaded: ResMut<PreloadedAssets>,
) {
    let terrain = terrain.clone();
    let task = AsyncComputeTaskPool::get().spawn(async move {
        let mut sites = HashMap::default();
        let mut pending = Vec::new();
        for z in -TILE_RADIUS..TILE_RADIUS {
            for x in -TILE_RADIUS..TILE_RADIUS {
                let tile = IVec2::new(x, z);
                let (ground, wet) = terrain.build_tile(tile);
                let (objects, forest, stone) = scenery::build_tile(&terrain, tile);
                sites.insert(tile, objects);
                pending.push(TerrainTile {
                    ground,
                    water: water::build_tile(tile, &wet),
                    forest,
                    stone,
                });
            }
        }
        Landscape {
            sites,
            pending,
            meshes: Vec::new(),
        }
    });
    let source = server.add_async(async move { Ok::<_, Infallible>(task.await) });
    preloaded.0.push(source.clone().untyped());
    commands.insert_resource(LandscapeSource(source));
}

impl Landscape {
    pub(super) fn upload(&mut self, meshes: &mut Assets<Mesh>) {
        for tile in self.pending.drain(..) {
            self.meshes.push(TileMeshes {
                ground: meshes.add(tile.ground),
                water: tile.water.map(|mesh| meshes.add(mesh)),
                forest: tile.forest.map(|mesh| meshes.add(mesh)),
                stone: tile.stone.map(|mesh| meshes.add(mesh)),
            });
        }
    }

    pub(super) fn spawn(
        &self,
        world: &mut ChildSpawnerCommands,
        terrain: &Handle<TerrainMaterial>,
        water_surface: &Handle<WaterMaterial>,
        foliage: &Handle<StandardMaterial>,
    ) {
        for tile in &self.meshes {
            world.spawn((Mesh3d(tile.ground.clone()), MeshMaterial3d(terrain.clone())));
            if let Some(mesh) = &tile.water {
                water::spawn(world, mesh.clone(), water_surface);
            }
            if let Some(mesh) = &tile.forest {
                world.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(foliage.clone())));
            }
            if let Some(mesh) = &tile.stone {
                world.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(terrain.clone())));
            }
        }
    }

    pub(super) fn scenery(&self, root: Entity) -> Scenery {
        Scenery {
            root,
            sites: self.sites.clone(),
            active: HashMap::default(),
            center: None,
        }
    }
}
