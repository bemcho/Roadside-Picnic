//! UI: HUD and menus.
//!
//! All player-facing text lives in [`text`] as localization-ready constants —
//! never hardcode display strings in systems.

mod splash;
pub mod text;

use bevy::diagnostic::{Diagnostic, DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::audio::AudioMode;
use crate::player::{
    Locomotion, Player,
    vehicle::{Driving, Vehicle, can_enter_vehicle},
};
use crate::states::{AppState, InGamePhase};
use crate::zone::ZoneRun;

pub struct UiPlugin;

/// Root marker for the in-game HUD.
#[derive(Component)]
struct UiRoot;

/// Marks the HUD text node that shows per-run Zone statistics.
#[derive(Component)]
struct HudStats;

#[derive(Component)]
struct HudFps;

#[derive(Component)]
struct VehiclePrompt;

#[derive(Component)]
struct HudControls;

/// Root marker for the pause menu overlay.
#[derive(Component)]
struct PauseMenuRoot;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((splash::SplashPlugin, FrameTimeDiagnosticsPlugin::new(20)))
            .add_systems(OnEnter(AppState::InGame), spawn_hud)
            .add_systems(OnExit(AppState::InGame), despawn_hud)
            .add_systems(
                Update,
                update_hud_stats.run_if(in_state(InGamePhase::Running)),
            )
            .add_systems(Update, update_fps.run_if(in_state(AppState::InGame)))
            .add_systems(
                PostUpdate,
                update_vehicle_prompt.run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnEnter(InGamePhase::Paused), spawn_pause_menu)
            .add_systems(OnExit(InGamePhase::Paused), despawn_pause_menu);
    }
}

fn spawn_hud(mut commands: Commands) {
    let root = commands
        .spawn((
            UiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ))
        .id();

    commands.entity(root).with_children(|hud| {
        // Top-left corner: place title.
        hud.spawn((
            Text::new(text::HUD_TITLE),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(Color::srgb(0.85, 0.85, 0.8)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                ..default()
            },
        ));

        // Below the title: live Zone statistics, refreshed each frame.
        hud.spawn((
            HudStats,
            Text::new(text::hud_zone_stats(0, 0)),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::srgb(0.75, 0.75, 0.7)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(44.0),
                left: Val::Px(12.0),
                ..default()
            },
        ));

        hud.spawn((
            HudFps,
            Text::new(text::HUD_FPS_PENDING),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(Color::srgb(0.85, 0.85, 0.8)),
            TextShadow {
                offset: Vec2::splat(1.0),
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                right: Val::Px(12.0),
                ..default()
            },
        ));

        // Bottom-left corner: controls hint.
        hud.spawn((
            HudControls,
            Text::new(text::HUD_CONTROLS_HINT),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.7, 0.65)),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(12.0),
                left: Val::Px(12.0),
                ..default()
            },
        ));

        hud.spawn((
            VehiclePrompt,
            Text::default(),
            TextFont {
                font_size: FontSize::Px(22.0),
                ..default()
            },
            TextColor(Color::srgb(0.97, 0.9, 0.66)),
            TextLayout::justify(Justify::Center),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                bottom: Val::Percent(14.0),
                ..default()
            },
        ));
    });
}

fn despawn_hud(mut commands: Commands, roots: Query<Entity, With<UiRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

/// Refreshes the HUD statistics line from [`ZoneRun`].
fn update_hud_stats(run: Res<ZoneRun>, mut stats: Query<&mut Text, With<HudStats>>) {
    let Ok(mut text) = stats.single_mut() else {
        return;
    };
    **text = text::hud_zone_stats(run.artifacts_collected, run.anomalies_triggered);
}

fn update_fps(diagnostics: Res<DiagnosticsStore>, mut fps: Single<&mut Text, With<HudFps>>) {
    let Some(milliseconds) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(Diagnostic::smoothed)
        .filter(|&value| value.is_finite() && value > 0.0)
    else {
        return;
    };
    let line = text::hud_fps(1000.0 / milliseconds);
    if fps.0 != line {
        fps.0 = line;
    }
}

fn update_vehicle_prompt(
    phase: Res<State<InGamePhase>>,
    audio: Res<AudioMode>,
    player: Query<(&Transform, &Locomotion, Option<&Driving>), With<Player>>,
    vehicles: Query<&Transform, With<Vehicle>>,
    mut prompts: Query<&mut Text, (With<VehiclePrompt>, Without<HudControls>)>,
    mut controls: Query<&mut Text, With<HudControls>>,
) {
    let (Ok((feet, locomotion, driving)), Ok(mut prompt), Ok(mut controls)) =
        (player.single(), prompts.single_mut(), controls.single_mut())
    else {
        return;
    };
    let driving = driving.is_some();
    let hint = if *phase.get() == InGamePhase::Paused {
        ""
    } else if driving {
        text::HUD_EXIT_VEHICLE
    } else if vehicles
        .iter()
        .any(|at| can_enter_vehicle(locomotion, at, feet.translation))
    {
        text::HUD_ENTER_VEHICLE
    } else {
        ""
    };
    let controls_hint = match (driving, *audio == AudioMode::Muted) {
        (false, false) => text::HUD_CONTROLS_HINT,
        (false, true) => text::HUD_CONTROLS_MUTED,
        (true, false) => text::HUD_DRIVER_CONTROLS,
        (true, true) => text::HUD_DRIVER_MUTED,
    };
    if prompt.0 != hint {
        prompt.0 = hint.to_owned();
    }
    if controls.0 != controls_hint {
        controls.0 = controls_hint.to_owned();
    }
}

fn spawn_pause_menu(mut commands: Commands) {
    let root = commands
        .spawn((PauseMenuRoot, Transform::default(), Visibility::default()))
        .id();

    commands.entity(root).with_children(|menu| {
        // Dimmed full-screen backdrop.
        menu.spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        });

        // Centered "PAUSED" label.
        menu.spawn((
            Text::new(text::PAUSE_TITLE),
            TextFont {
                font_size: FontSize::Px(40.0),
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.85)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(45.0),
                left: Val::Percent(0.0),
                width: Val::Percent(100.0),
                ..default()
            },
        ));

        // Below it: resume hint.
        menu.spawn((
            Text::new(text::PAUSE_HINT),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.7, 0.65)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(52.0),
                left: Val::Percent(0.0),
                width: Val::Percent(100.0),
                ..default()
            },
        ));
    });
}

fn despawn_pause_menu(mut commands: Commands, roots: Query<Entity, With<PauseMenuRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    #[test]
    fn hud_displays_frames_per_second_from_real_frame_time() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, FrameTimeDiagnosticsPlugin::new(20)))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                40,
            )))
            .add_systems(Startup, spawn_hud)
            .add_systems(Update, update_fps);
        for _ in 0..4 {
            app.update();
        }
        let world = app.world_mut();
        let line = world
            .query_filtered::<&Text, With<HudFps>>()
            .single(world)
            .expect("one FPS counter in the HUD");
        assert_eq!(line.0, "FPS: 25");
    }
}
