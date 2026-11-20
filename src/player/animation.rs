//! Skeletal animation of the stalker: Idle, Walk, Run, and Jump clips baked
//! into `stalker.glb` (see `tools/rig_stalker.py`), cross-faded by gait.

use std::time::Duration;

use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

use crate::assets::MODELS;

/// How the stalker is moving; selects the animation clip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Gait {
    #[default]
    Idle,
    Walk,
    Run,
    Jump,
}

/// Clip order in `stalker.glb` (the Blender exporter sorts actions by name).
const CLIPS: [Gait; 4] = [Gait::Idle, Gait::Jump, Gait::Run, Gait::Walk];

/// Cross-fade time between clips.
const BLEND: Duration = Duration::from_millis(180);

/// The stalker's animation graph and the node playing each [`Gait`].
#[derive(Resource)]
pub(super) struct StalkerAnimations {
    graph: Handle<AnimationGraph>,
    nodes: Vec<(Gait, AnimationNodeIndex)>,
}

impl StalkerAnimations {
    fn node(&self, gait: Gait) -> Option<AnimationNodeIndex> {
        self.nodes
            .iter()
            .find_map(|&(g, node)| (g == gait).then_some(node))
    }
}

/// Points from the player's model child to the gait driving it.
#[derive(Component)]
pub(super) struct StalkerModel {
    pub player: Entity,
}

/// The animation player inside the stalker's model, and the gait it shows.
#[derive(Component)]
pub(super) struct StalkerAnimator {
    player: Entity,
    shown: Gait,
}

pub(super) fn load_stalker_animations(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let path = format!("{MODELS}/stalker.glb");
    let (graph, nodes) = AnimationGraph::from_clips(
        (0..CLIPS.len())
            .map(|i| asset_server.load(GltfAssetLabel::Animation(i).from_asset(path.clone()))),
    );
    commands.insert_resource(StalkerAnimations {
        graph: graphs.add(graph),
        nodes: CLIPS.into_iter().zip(nodes).collect(),
    });
}

/// Hooks the animation graph up to the model's `AnimationPlayer` once the
/// glTF scene has spawned, starting in [`Gait::Idle`].
pub(super) fn attach_stalker_animator(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    animations: Res<StalkerAnimations>,
    models: Query<&StalkerModel>,
    children: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
) {
    let Ok(model) = models.get(ready.entity) else {
        return;
    };
    for entity in children.iter_descendants(ready.entity) {
        let Ok(mut animation_player) = players.get_mut(entity) else {
            continue;
        };
        let mut transitions = AnimationTransitions::new();
        if let Some(idle) = animations.node(Gait::Idle) {
            transitions
                .play(&mut animation_player, idle, Duration::ZERO)
                .repeat();
        }
        commands.entity(entity).insert((
            AnimationGraphHandle(animations.graph.clone()),
            transitions,
            StalkerAnimator {
                player: model.player,
                shown: Gait::Idle,
            },
        ));
    }
}

/// Cross-fades to the clip for the stalker's current gait.
pub(super) fn animate_stalker(
    animations: Res<StalkerAnimations>,
    gaits: Query<&super::Locomotion>,
    mut animators: Query<(
        &mut StalkerAnimator,
        &mut AnimationPlayer,
        &mut AnimationTransitions,
    )>,
) {
    for (mut animator, mut animation_player, mut transitions) in &mut animators {
        let Ok(locomotion) = gaits.get(animator.player) else {
            continue;
        };
        let gait = locomotion.gait;
        if gait == animator.shown {
            continue;
        }
        let Some(node) = animations.node(gait) else {
            continue;
        };
        let clip = transitions.play(&mut animation_player, node, BLEND);
        // The jump clip tucks once and holds the pose until landing.
        if gait != Gait::Jump {
            clip.repeat();
        }
        animator.shown = gait;
    }
}

pub(super) fn pause_stalker_animation(
    mut players: Query<&mut AnimationPlayer, With<StalkerAnimator>>,
) {
    for mut player in &mut players {
        player.pause_all();
    }
}

pub(super) fn resume_stalker_animation(
    mut players: Query<&mut AnimationPlayer, With<StalkerAnimator>>,
) {
    for mut player in &mut players {
        player.resume_all();
    }
}
