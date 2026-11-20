//! Top-level application states and their sub-states.
//!
//! Systems must run only in the state they belong to. Use `OnEnter`/`OnExit`
//! to spawn and despawn scene objects, and `run_if(in_state(...))` for
//! per-frame logic.

use bevy::prelude::*;

/// Global game states. Never model state with boolean resources.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Bootstrapping: future home of asset preloading before entering the Zone.
    #[default]
    Loading,
    /// Inside the Zone. Gameplay sub-states live in [`InGamePhase`].
    InGame,
}

/// Sub-states of [`AppState::InGame`].
///
/// Pausing is an overlay on top of the Zone: world, player, and HUD stay
/// spawned while paused, so this is a sub-state rather than a top-level one.
#[derive(SubStates, Debug, Clone, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::InGame)]
pub enum InGamePhase {
    /// Gameplay active.
    #[default]
    Running,
    /// Pause menu overlay on top of the Zone.
    Paused,
}

/// Advances the app out of [`AppState::Loading`] into [`AppState::InGame`].
///
/// Placeholder for real asset preloading: once assets actually need loading,
/// those gates go here before the state transition.
pub fn advance_loading(state: Res<State<AppState>>, mut next_state: ResMut<NextState<AppState>>) {
    if *state.get() == AppState::Loading {
        next_state.set(AppState::InGame);
    }
}

/// Toggles [`InGamePhase`] between `Running` and `Paused` on Escape.
pub fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<InGamePhase>>,
    mut next_phase: ResMut<NextState<InGamePhase>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    match phase.get() {
        InGamePhase::Running => next_phase.set(InGamePhase::Paused),
        InGamePhase::Paused => next_phase.set(InGamePhase::Running),
    }
}
