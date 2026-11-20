//! Third-person orbit camera: hold the right mouse button and drag to look
//! around the stalker, scroll to zoom.

use std::f32::consts::FRAC_PI_2;

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::states::InGamePhase;
use crate::world::Terrain;

use super::{Player, vehicle::Driving};

/// Height above the stalker's feet the camera orbits around and looks at.
pub(super) const LOOK_HEIGHT: f32 = 1.6;

/// Radians of rotation per pixel of mouse motion.
const MOUSE_SENSITIVITY: f32 = 0.004;

/// Allowed camera pitch: from slightly below the look point to near top-down.
const PITCH_LIMITS: (f32, f32) = (-0.25, FRAC_PI_2 - 0.1);

/// Allowed orbit distance, in meters.
const DISTANCE_LIMITS: (f32, f32) = (3.0, 20.0);

/// Smallest gap kept between the camera and the ground below it.
const GROUND_CLEARANCE: f32 = 0.5;

/// Orbit around the stalker. `yaw` 0 places the camera behind a stalker
/// walking toward -Z; positive `pitch` raises it.
#[derive(Component)]
pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.18,
            distance: 9.0,
        }
    }
}

impl OrbitCamera {
    /// Camera position relative to the look point.
    pub fn offset(&self) -> Vec3 {
        Quat::from_euler(EulerRot::YXZ, self.yaw, -self.pitch, 0.0) * Vec3::Z * self.distance
    }

    /// Ground-plane rotation matching where the camera looks.
    pub fn heading(&self) -> Quat {
        Quat::from_rotation_y(self.yaw)
    }
}

/// Rotates the orbit while the right mouse button is held; zooms on scroll.
pub(super) fn orbit_camera(
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mut cameras: Query<&mut OrbitCamera>,
) {
    let Ok(mut orbit) = cameras.single_mut() else {
        return;
    };

    // Skip the press frame: motion accumulated before the grab (or caused by
    // it) would otherwise jerk the view.
    if mouse.pressed(MouseButton::Right) && !mouse.just_pressed(MouseButton::Right) {
        orbit.yaw -= motion.delta.x * MOUSE_SENSITIVITY;
        orbit.pitch = (orbit.pitch + motion.delta.y * MOUSE_SENSITIVITY)
            .clamp(PITCH_LIMITS.0, PITCH_LIMITS.1);
    }

    let notches = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
    };
    if notches != 0.0 {
        orbit.distance =
            (orbit.distance * 0.9_f32.powf(notches)).clamp(DISTANCE_LIMITS.0, DISTANCE_LIMITS.1);
    }
}

/// Places the camera on its orbit and keeps it from sinking into hillsides.
pub(super) fn follow_player(
    terrain: Res<Terrain>,
    players: Query<(&Transform, Option<&Driving>), With<Player>>,
    mut cameras: Query<(&OrbitCamera, &mut Transform), Without<Player>>,
) {
    let Ok((player, driving)) = players.single() else {
        return;
    };
    let Ok((orbit, mut camera)) = cameras.single_mut() else {
        return;
    };

    let height = if driving.is_some() { 2.1 } else { LOOK_HEIGHT };
    let focus = player.translation + Vec3::Y * height;
    let mut position = focus + orbit.offset();
    let floor = terrain.height_at(position.x, position.z) + GROUND_CLEARANCE;
    position.y = position.y.max(floor);

    camera.translation = position;
    camera.look_at(focus, Vec3::Y);
}

/// Hides and locks the cursor while orbiting, as third-person games do.
pub(super) fn grab_cursor_while_orbiting(
    mouse: Res<ButtonInput<MouseButton>>,
    phase: Option<Res<State<InGamePhase>>>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Ok(mut cursor) = cursors.single_mut() else {
        return;
    };
    let running = phase.is_some_and(|phase| *phase.get() == InGamePhase::Running);
    let orbiting = running && mouse.pressed(MouseButton::Right);
    let grab_mode = if orbiting {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    if cursor.grab_mode != grab_mode {
        cursor.grab_mode = grab_mode;
        cursor.visible = !orbiting;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_orbit_sits_behind_and_above_the_stalker() {
        let offset = OrbitCamera::default().offset();
        assert!(offset.z > 0.0 && offset.y > 0.0, "offset {offset}");
        assert!(offset.x.abs() < 1e-5);
        assert!((offset.length() - 9.0).abs() < 1e-4);
    }

    #[test]
    fn heading_turns_forward_with_the_camera() {
        let orbit = OrbitCamera {
            yaw: FRAC_PI_2,
            ..default()
        };
        // Camera swung round to the stalker's +X side, so forward is -X.
        let forward = orbit.heading() * Vec3::NEG_Z;
        assert!((forward - Vec3::NEG_X).length() < 1e-5, "forward {forward}");
        assert!(orbit.offset().x > 0.0);
    }
}
