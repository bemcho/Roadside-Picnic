//! The Zone: anomalies, artifacts, and per-run state.
//!
//! Anomalies are dangerous; artifacts are valuable. Both are spawned as
//! placeholder geometry for now, to be replaced with generated content.

use bevy::prelude::*;

use crate::states::AppState;
use crate::world::{Collider, Pillar, Terrain};

pub struct ZonePlugin;

/// Per-run Zone statistics. Shown in the HUD and used for scoring.
#[derive(Resource, Default)]
pub struct ZoneRun {
    pub artifacts_collected: u32,
    pub anomalies_triggered: u32,
}

/// A valuable artifact — the reason stalkers enter the Zone.
#[derive(Component)]
pub struct Artifact;

/// A dangerous anomaly — deadly to touch.
#[derive(Component)]
pub struct Anomaly;

/// Solid shapes of the placeholder artifact sphere and anomaly cylinder.
const ARTIFACT_SHAPE: &[Pillar] = &[Pillar::centered(0.2, 0.4)];
const ANOMALY_SHAPE: &[Pillar] = &[Pillar::centered(0.4, 1.2)];

/// Root marker for everything spawned by this plugin's OnEnter.
#[derive(Component)]
struct ZoneRoot;

impl Plugin for ZonePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ZoneRun>()
            .add_systems(OnEnter(AppState::InGame), spawn_zone_objects)
            .add_systems(OnExit(AppState::InGame), despawn_zone_objects);
    }
}

fn spawn_zone_objects(
    mut commands: Commands,
    terrain: Res<Terrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let root = commands
        .spawn((ZoneRoot, Transform::default(), Visibility::default()))
        .id();

    let artifact_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 1.0, 0.6),
        emissive: LinearRgba::rgb(2.0, 6.0, 2.0),
        ..default()
    });
    let anomaly_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.1, 0.1),
        emissive: LinearRgba::rgb(6.0, 1.0, 1.0),
        ..default()
    });

    // A scatter of placeholder artifacts, resting on the terrain:
    // overgrown flats, the scorched bald hill, and the quarry floor.
    for (x, z) in [
        (6.0, 3.0),
        (-7.0, 5.0),
        (4.0, -8.0),
        (-5.0, -4.0),
        (9.0, -6.0),
        (-75.0, -55.0), // bald hill
        (78.0, 44.0),   // quarry floor
    ] {
        let y = terrain.height_at(x, z) + 0.35;
        commands.entity(root).with_children(|zone| {
            zone.spawn((
                Artifact,
                Collider(ARTIFACT_SHAPE),
                Mesh3d(meshes.add(Sphere::new(0.2))),
                MeshMaterial3d(artifact_material.clone()),
                Transform::from_xyz(x, y, z),
            ));
        });
    }

    // A couple of placeholder anomalies: one in the flats, one at the very
    // bottom of the quarry, where the Zone is said to be meanest.
    for (x, z) in [(-3.0, 7.0), (8.0, -8.0), (70.0, 40.0)] {
        let y = terrain.height_at(x, z) + 0.6;
        commands.entity(root).with_children(|zone| {
            zone.spawn((
                Anomaly,
                Collider(ANOMALY_SHAPE),
                Mesh3d(meshes.add(Cylinder::new(0.4, 1.2))),
                MeshMaterial3d(anomaly_material.clone()),
                Transform::from_xyz(x, y, z),
            ));
        });
    }
}

fn despawn_zone_objects(mut commands: Commands, roots: Query<Entity, With<ZoneRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_run_starts_empty() {
        let run = ZoneRun::default();
        assert_eq!(run.artifacts_collected, 0);
        assert_eq!(run.anomalies_triggered, 0);
    }
}
