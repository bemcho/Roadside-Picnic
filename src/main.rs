//! Roadside-Picnic — a sci-fi game set in the Zone.
//!
//! See `CLAUDE.md` for architecture conventions.

mod assets;
mod player;
mod states;
mod ui;
mod world;
mod zone;

use bevy::prelude::*;

use player::PlayerPlugin;
use states::{AppState, InGamePhase, advance_loading, toggle_pause};
use ui::UiPlugin;
use world::WorldPlugin;
use zone::ZonePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<AppState>()
        .add_sub_state::<InGamePhase>()
        .add_plugins((WorldPlugin, PlayerPlugin, ZonePlugin, UiPlugin))
        .add_systems(Update, advance_loading.run_if(in_state(AppState::Loading)))
        .add_systems(Update, toggle_pause.run_if(in_state(AppState::InGame)))
        .add_systems(Update, quit_on_q)
        .run();
}

/// Quits the app when Q is pressed.
fn quit_on_q(keys: Res<ButtonInput<KeyCode>>, mut exits: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::KeyQ) {
        exits.write(AppExit::Success);
    }
}
