//! UI: HUD and menus.
//!
//! All player-facing text lives in [`text`] as localization-ready constants —
//! never hardcode display strings in systems.

pub mod text;

use bevy::prelude::*;

use crate::states::{AppState, InGamePhase};
use crate::zone::ZoneRun;

pub struct UiPlugin;

/// Root marker for the in-game HUD.
#[derive(Component)]
struct UiRoot;

/// Marks the HUD text node that shows per-run Zone statistics.
#[derive(Component)]
struct HudStats;

/// Root marker for the pause menu overlay.
#[derive(Component)]
struct PauseMenuRoot;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), spawn_hud)
            .add_systems(OnExit(AppState::InGame), despawn_hud)
            .add_systems(
                Update,
                update_hud_stats.run_if(in_state(InGamePhase::Running)),
            )
            .add_systems(OnEnter(InGamePhase::Paused), spawn_pause_menu)
            .add_systems(OnExit(InGamePhase::Paused), despawn_pause_menu);
    }
}

fn spawn_hud(mut commands: Commands) {
    let root = commands
        .spawn((UiRoot, Transform::default(), Visibility::default()))
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

        // Bottom-left corner: controls hint.
        hud.spawn((
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
