//! Third-person orbit camera: hold the right mouse button and drag to look
//! around the stalker, scroll to zoom.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::states::InGamePhase;
use crate::world::Terrain;

use super::{Player, response, vehicle::Driving};

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
const MAX_FOLLOW_LAG: f32 = 1.4;

/// Orbit around the stalker. `yaw` 0 places the camera behind a stalker
/// walking toward -Z; positive `pitch` raises it.
#[derive(Component)]
#[require(CameraFollow)]
pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub(super) pending_yaw: f32,
    pub(super) reset_follow: bool,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.18,
            distance: 9.0,
            pending_yaw: 0.0,
            reset_follow: true,
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

    pub(super) fn face_vehicle(&mut self, forward: Vec3) {
        let target = (-forward.x).atan2(-forward.z);
        self.pending_yaw = (target - self.yaw + PI).rem_euclid(TAU) - PI;
    }

    pub(super) fn cancel_assist(&mut self) {
        self.pending_yaw = 0.0;
    }

    pub(super) fn reset_heading(&mut self, yaw: f32) {
        self.yaw = yaw;
        self.cancel_assist();
        self.reset_follow = true;
    }

    pub(super) fn record_turn(&mut self, turn: f32) {
        self.pending_yaw += turn;
    }

    fn ease_heading(&mut self, dt: f32) {
        let yaw = self.pending_yaw * response(10.0, dt);
        self.yaw += yaw;
        if self.yaw.abs() > PI {
            self.yaw = (self.yaw + PI).rem_euclid(TAU) - PI;
        }
        self.pending_yaw -= yaw;
    }
}

#[derive(Component, Default)]
pub(super) struct CameraFollow {
    focus: Vec3,
    distance: f32,
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
    if mouse.pressed(MouseButton::Right) {
        orbit.pending_yaw = 0.0;
        if !mouse.just_pressed(MouseButton::Right) {
            orbit.yaw -= motion.delta.x * MOUSE_SENSITIVITY;
            orbit.pitch = (orbit.pitch + motion.delta.y * MOUSE_SENSITIVITY)
                .clamp(PITCH_LIMITS.0, PITCH_LIMITS.1);
        }
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
    time: Res<Time>,
    terrain: Res<Terrain>,
    players: Query<(&Transform, Option<&Driving>), With<Player>>,
    mut cameras: Query<(&mut OrbitCamera, &mut CameraFollow, &mut Transform), Without<Player>>,
) {
    let Ok((player, driving)) = players.single() else {
        return;
    };
    let Ok((mut orbit, mut follow, mut camera)) = cameras.single_mut() else {
        return;
    };
    let seated = driving.is_some();
    let dt = time.delta_secs();
    orbit.ease_heading(dt);
    let height = if seated { 2.1 } else { LOOK_HEIGHT };
    let focus = player.translation + Vec3::Y * height;
    if orbit.reset_follow {
        follow.focus = focus;
        follow.distance = orbit.distance;
        orbit.reset_follow = false;
    } else {
        let follow_rate = if seated {
            Vec3::new(12.0, 7.0, 12.0)
        } else {
            Vec3::splat(22.0)
        };
        let blend = follow_rate.to_array().map(|rate| response(rate, dt));
        let lag = (follow.focus - focus) * (Vec3::ONE - Vec3::from_array(blend));
        follow.focus = focus + lag.clamp_length_max(MAX_FOLLOW_LAG);
        follow.distance += (orbit.distance - follow.distance) * response(14.0, dt);
    }
    let mut position = follow.focus + orbit.offset() * (follow.distance / orbit.distance);
    let floor = terrain
        .height_at(position.x, position.z)
        .max(terrain.mesh_height_at(position.xz()))
        + GROUND_CLEARANCE;
    position.y = position.y.max(floor);

    camera.translation = position;
    camera.look_at(follow.focus, Vec3::Y);
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
    fn vehicle_alignment_eases_along_the_short_arc() {
        let mut orbit = OrbitCamera {
            yaw: PI - 0.1,
            ..default()
        };
        let target = -PI + 0.1;
        orbit.face_vehicle(Quat::from_rotation_y(target) * Vec3::NEG_Z);
        orbit.ease_heading(0.02);
        assert!(orbit.yaw > PI - 0.1 && orbit.yaw < PI);
        for _ in 0..50 {
            orbit.ease_heading(0.02);
        }
        assert!((orbit.yaw - target).abs() < 0.001);
    }

    #[test]
    fn chase_focus_and_zoom_ease_but_respawn_resets_follow() {
        let terrain = Terrain::new();
        let base = Vec3::new(40.0, terrain.height_at(40.0, -128.0), -128.0);
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs_f32(0.05));
        app.insert_resource(time)
            .insert_resource(terrain)
            .add_systems(Update, follow_player);
        let world = app.world_mut();
        let car = world.spawn_empty().id();
        let player = world
            .spawn((
                Player::default(),
                Transform::from_translation(base),
                Driving(car),
            ))
            .id();
        let camera = world
            .spawn((OrbitCamera::default(), Transform::IDENTITY))
            .id();
        app.update();

        let world = app.world_mut();
        world
            .get_mut::<Transform>(player)
            .expect("player")
            .translation
            .x += 1.0;
        world
            .get_mut::<OrbitCamera>(camera)
            .expect("orbit")
            .distance = 12.0;
        app.update();
        let follow = app.world().get::<CameraFollow>(camera).expect("follow");
        assert!((base.x..base.x + 0.8).contains(&follow.focus.x));
        assert!((9.0..12.0).contains(&follow.distance));

        let world = app.world_mut();
        world
            .get_mut::<Transform>(player)
            .expect("player")
            .translation = base;
        world.entity_mut(player).remove::<Driving>();
        world
            .get_mut::<OrbitCamera>(camera)
            .expect("orbit")
            .reset_heading(0.0);
        app.update();
        let follow = app.world().get::<CameraFollow>(camera).expect("follow");
        assert_eq!(follow.focus, base + Vec3::Y * LOOK_HEIGHT);
        assert_eq!(follow.distance, 12.0);
    }

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
