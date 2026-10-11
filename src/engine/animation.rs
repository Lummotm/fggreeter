mod animation_queue;
mod animations;
mod impact_frame;
mod particle_system;
mod screen_shake;
mod ui;

use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

pub use impact_frame::HiddenOnImpact;
pub use screen_shake::ShakeOnHit;

/// The animation event is an event sent by the state
/// machine that drives the fighting game animation
#[derive(bevy::prelude::Event)]
#[derive(Debug, Clone, Copy)]
pub enum AnimationEvent {
    /// The protagonist sends a hit to the antagonist
    Hit,
    /// The antagonist sends a hit to the protagonist
    Block,
    /// Both fighter backflip back into positions
    Reset,
    /// The protagonist charges up a super attack
    Windup,
    /// The protagonist deliver the super attack,
    /// this is followed by a nice impact frames with color changes
    /// that eventually leads to a black screen for the session start
    Finisher,
    /// The antagonist counters the charged attack
    Counter,
}

/// Fired by a clip when the fighters fall back out of contact
#[derive(bevy::animation::AnimationEvent, Clone)]
pub struct BackToDistance {
    pub node: AnimationNodeIndex,
}

/// Event emitted the instant the hit occurs
#[derive(bevy::animation::AnimationEvent, Clone)]
pub struct HitImpact;

/// Event emitted the instant the hit occurs in the finisher
#[derive(bevy::animation::AnimationEvent, Clone)]
pub struct FinisherImpact;

/// Event emitted the instant the hit occurs in the counter
#[derive(bevy::animation::AnimationEvent, Clone)]
pub struct CounterImpact;

/// Event emitted when the protagonist starts charging in the windup
#[derive(bevy::animation::AnimationEvent, Clone)]
pub struct WindupCharge;

/// The GLTF file with both fighters models and their animations.
/// This is a handle while the file is loading.
#[derive(Resource)]
struct FightersGltf(Handle<Gltf>);

/// Controller of the animations of the fight
#[derive(Resource)]
struct FightAnimator {
    graph: Handle<AnimationGraph>,
    fighting_state: FightingState,
    collection: FightAnimationCollection,
}

enum FightingState {
    Idle,
    Engaged,
    Windup { windup: WindupClip },
    Finished,
    Countered,
}

/// Storage for all loaded animation clips of the fight
struct FightAnimationCollection {
    idle: AnimationNodeIndex,
    hits: Vec<AnimationNodeIndex>,
    windups: Vec<WindupClip>,
}

#[derive(Clone)]
struct WindupClip {
    windup: AnimationNodeIndex,
    finisher: AnimationNodeIndex,
    counter: AnimationNodeIndex,
}

/// Marker for the hit point bone entity
#[derive(Component)]
struct HitPoint;

/// Path of the fighters model asset
const FIGHTERS_ASSET_PATH: &str = "models/fighters.glb";
/// Name of the bone used to position hit points
const HIT_POINT_BONE_NAME: &str = "HitPoint";

/// Duration of an animation transition
const ANIM_TRANSITION_DURATION: std::time::Duration = std::time::Duration::from_millis(50);

/// Frame rate of the imported animations
const ANIM_FPS: u64 = 24;
/// Frame of the animation at which the characters hit (spawn particles)
const HIT_FRAME: u64 = 4;
/// Frame of the animation at which the characters start to go back to idle
const REST_FRAME: u64 = 12;

/// Duration of a frame, to set the hit event the frame after the hit
const FRAME_TIME: std::time::Duration = std::time::Duration::from_millis(1000 / ANIM_FPS);
/// Time in the hit animation it takes for the characters to close the distance
const HIT_ANIM_CLOSEUP_DURATION: std::time::Duration = std::time::Duration::from_millis(1000 * HIT_FRAME / ANIM_FPS);
/// Time in the hit animation it takes for the characters to fully perform the hit the distance
const HIT_ANIM_HIT_DURATION: std::time::Duration = std::time::Duration::from_millis(1000 * REST_FRAME / ANIM_FPS);

pub fn add_plugins(app: &mut App) {
    app.add_systems(Startup, setup);
    app.add_systems(Update, wait_for_assets);
    app.add_observer(handle_animation_event);
    app.add_observer(on_back_to_distance);

    animation_queue::add_plugins(app);
    animations::add_plugins(app);
    impact_frame::add_plugins(app);
    particle_system::add_plugins(app);
    screen_shake::add_plugins(app);
    ui::add_plugins(app);
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    /* Start loading the GLTF file with the fighters */
    let fighters_handle: Handle<Gltf> = asset_server.load(FIGHTERS_ASSET_PATH);
    let fighters_gltf = FightersGltf(fighters_handle);
    commands.insert_resource(fighters_gltf);
}

/// System that will run every frame when there is a fighter gltf file getting loaded.
/// Once we built all we need from the asset, we remove the resource and the system will never run again.
fn wait_for_assets(
    fighters_gltf: If<Res<FightersGltf>>,
    gltfs: Res<Assets<Gltf>>,
    mut clips: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut commands: Commands,
) {
    /* Check whether the asset is loaded yet */
    let handle: &Handle<Gltf> = &fighters_gltf.0.0;
    let Some(gltf) = gltfs.get(handle) else { return };

    /* Create the animator */
    match create_animator(&gltf, &mut graphs, &mut clips) {
        Ok(animator) => {
            spawn_fighters(&gltf, &animator, &mut commands);
            commands.insert_resource(animator);
        }
        Err(e) => tracing::warn!("Unable to spawn fighers, could not create the animator: {e}"),
    };

    /* Asset loaded, we can throw the loading handle */
    commands.remove_resource::<FightersGltf>();
}

/// Create the animator controller for the fight from the gltf file
fn create_animator(
    fighters_gltf: &Gltf,
    graphs: &mut Assets<AnimationGraph>,
    clips: &mut Assets<AnimationClip>,
) -> Result<FightAnimator, String> {
    /* Build the animation graph */
    let mut graph = AnimationGraph::new();

    for (name, _) in fighters_gltf.named_animations.iter() {
        tracing::debug!("Found animation clip: {name}");
    }

    const IDLE_ANIM_NAME: &'static str = "Idle";
    const HIT_ANIM_PREFIX: &'static str = "Hit.";
    const WINDUP_ANIM_PREFIX: &'static str = "Windup.";
    const FINISHER_ANIM_PREFIX: &'static str = "Windup.Finisher.";
    const COUNTER_ANIM_PREFIX: &'static str = "Windup.Counter.";

    let Some(idle_clip) = fighters_gltf.named_animations.get(IDLE_ANIM_NAME) else {
        tracing::warn!("Failed to find an \"{IDLE_ANIM_NAME}\" animation, unable to create the animator");
        return Err(format!("Failed to find an \"{IDLE_ANIM_NAME}\""));
    };
    let hit_clips = fighters_gltf
        .named_animations
        .iter()
        .filter_map(|(name, clip)| match name.starts_with(HIT_ANIM_PREFIX) {
            true => Some(clip),
            false => None,
        });

    let windup_clips = fighters_gltf.named_animations.iter().filter_map(|(name, clip)| {
        match (
            name.strip_prefix(WINDUP_ANIM_PREFIX),
            name.starts_with(FINISHER_ANIM_PREFIX),
            name.starts_with(COUNTER_ANIM_PREFIX),
        ) {
            (Some(windup_name), false, false) => Some((windup_name, clip)),
            _ => None,
        }
    });
    let windup_clips = windup_clips.filter_map(|(name, clip)| {
        let Some(finisher) = fighters_gltf
            .named_animations
            .get(format!("{FINISHER_ANIM_PREFIX}{name}").as_str())
        else {
            tracing::warn!("No finisher animation for windup {name}");
            return None;
        };
        let Some(counter) = fighters_gltf
            .named_animations
            .get(format!("{COUNTER_ANIM_PREFIX}{name}").as_str())
        else {
            tracing::warn!("No counter animation for windup {name}");
            return None;
        };
        Some((clip, finisher, counter))
    });

    let idle = graph.add_clip(idle_clip.clone(), 1.0, graph.root);
    let hits: Vec<_> = hit_clips
        .map(|clip_handle| {
            let node = graph.add_clip(clip_handle.clone(), 1.0, graph.root);
            /* Add an event enimted by the clip to tell when we can change fighting state */
            if let Some(mut clip) = clips.get_mut(clip_handle) {
                clip.add_event(HIT_ANIM_HIT_DURATION.as_secs_f32(), BackToDistance { node });
                clip.add_event((HIT_ANIM_CLOSEUP_DURATION + FRAME_TIME).as_secs_f32(), HitImpact);
            }
            node
        })
        .collect();
    let windups: Vec<_> = windup_clips
        .map(|(windup, finisher, counter)| {
            let windup_node = graph.add_clip(windup.clone(), 1.0, graph.root);
            if let Some(mut clip) = clips.get_mut(windup) {
                /* Hacky way to avoid reading the hit bone mid transition */
                clip.add_event((ANIM_TRANSITION_DURATION + FRAME_TIME).as_secs_f32(), WindupCharge);
            }
            let finisher_node = graph.add_clip(finisher.clone(), 1.0, graph.root);
            if let Some(mut clip) = clips.get_mut(finisher) {
                /* Hacky way to avoid reading the hit bone mid transition */
                clip.add_event((ANIM_TRANSITION_DURATION + FRAME_TIME).as_secs_f32(), FinisherImpact);
            }
            let counter_node = graph.add_clip(counter.clone(), 1.0, graph.root);
            if let Some(mut clip) = clips.get_mut(counter) {
                /* Hacky way to avoid reading the hit bone mid transition */
                clip.add_event((ANIM_TRANSITION_DURATION + FRAME_TIME).as_secs_f32(), CounterImpact);
            }
            WindupClip {
                windup: windup_node,
                finisher: finisher_node,
                counter: counter_node,
            }
        })
        .collect();

    tracing::info!("Loaded {} animation hit clips", hits.len());

    let animator = FightAnimator {
        graph: graphs.add(graph),
        fighting_state: FightingState::Idle,
        collection: FightAnimationCollection { idle, hits, windups },
    };

    Ok(animator)
}

/// Spawn the two fighters from the glb model
fn spawn_fighters(fighters_gltf: &Gltf, animator: &FightAnimator, commands: &mut Commands) {
    let fighters_prefab = fighters_gltf.scenes[0].clone();
    let world_asset = WorldAssetRoot(fighters_prefab);

    let graph = animator.graph.clone();
    let idle = animator.collection.idle;

    /* Spawn the fighters, and whenever they are spawned start the animation */
    let root = commands.spawn(world_asset).id();
    commands.entity(root).observe(
        move |_: On<WorldInstanceReady>,
              names: Query<(Entity, &Name)>,
              parents: Query<&ChildOf>,
              children: Query<&Children>,
              meshes: Query<&MeshMaterial3d<StandardMaterial>>,
              mut animation_player: Single<(Entity, &mut AnimationPlayer)>,
              mut commands: Commands| {
            let (entity, player) = &mut *animation_player;
            let mut transitions = AnimationTransitions::new();
            transitions.play(player, idle, std::time::Duration::ZERO).repeat();

            let new_components = (AnimationGraphHandle(graph.clone()), transitions);
            commands.entity(*entity).insert(new_components);

            match names.iter().find(|(_, name)| name.as_str() == HIT_POINT_BONE_NAME) {
                Some((bone, _)) => _ = commands.entity(bone).insert(HitPoint),
                None => tracing::warn!("No \"{HIT_POINT_BONE_NAME}\" bone in the model, hit particles disabled"),
            }

            /* Tag every fighter mesh with its fighter and original material */
            for entity in children.iter_descendants(root) {
                let Ok(material) = meshes.get(entity) else { continue };
                let fighter_material = std::iter::once(entity)
                    .chain(parents.iter_ancestors(entity))
                    .filter_map(|e| names.get(e).ok())
                    .find_map(|(_, name)| impact_frame::FighterMaterial::from_node_name(name, material.0.clone()));
                match fighter_material {
                    Some(fighter_material) => _ = commands.entity(entity).insert(fighter_material),
                    None => tracing::warn!("Mesh {entity} is not under a fighter node, ignored by impact frames"),
                }
            }
        },
    );
}

/// Observer to update the fight animation based on the emitted animation events
fn handle_animation_event(
    event: On<AnimationEvent>,
    mut animator: If<ResMut<FightAnimator>>,
    mut fight: Single<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    let (animation_player, animation_transition) = &mut *fight;
    match *event {
        AnimationEvent::Hit => hit(&mut *animator, animation_player, animation_transition),
        AnimationEvent::Windup => windup(&mut *animator, animation_player, animation_transition),
        AnimationEvent::Finisher => finisher(&mut *animator, animation_player, animation_transition),
        AnimationEvent::Counter => counter(&mut *animator, animation_player, animation_transition),
        AnimationEvent::Reset => animator.0.fighting_state = FightingState::Idle,
        other => tracing::warn!("Unhandled animation event: {other:?}"),
    }
}

/// Hit animation event, the protagonsit hits the antagonist
fn hit(animator: &mut FightAnimator, animation_player: &mut AnimationPlayer, animation_transition: &mut AnimationTransitions) {
    /* Pick a random hit to play */
    use rand::seq::IndexedRandom;
    let mut rng = rand::rng();
    let Some(hit_clip) = animator.collection.hits.choose(&mut rng) else {
        tracing::warn!("No hit animations clip: unable to play hit animation");
        return;
    };

    /* If the characters are already close to each other, start the anim at the hit */
    let start_duration = match animator.fighting_state {
        FightingState::Engaged => HIT_ANIM_CLOSEUP_DURATION,
        _ => std::time::Duration::ZERO,
    };

    /* Play the hit */
    let active_anim = animation_transition.play(animation_player, *hit_clip, ANIM_TRANSITION_DURATION);
    active_anim.seek_to(start_duration.as_secs_f32());

    /* Engaged from the moment a hit starts, so rapid typing always chains */
    animator.fighting_state = FightingState::Engaged;
}

/// Hit animation event, the protagonsit hits the antagonist
fn windup(animator: &mut FightAnimator, animation_player: &mut AnimationPlayer, animation_transition: &mut AnimationTransitions) {
    /* Pick a random windup to play */
    use rand::seq::IndexedRandom;
    let mut rng = rand::rng();
    let Some(windup_clip) = animator.collection.windups.choose(&mut rng) else {
        tracing::warn!("No windup animations clip: unable to play windup animation");
        return;
    };

    /* Play the windup */
    animation_transition.play(animation_player, windup_clip.windup, ANIM_TRANSITION_DURATION);

    /* Engaged from the moment a hit starts, so rapid typing always chains */
    animator.fighting_state = FightingState::Windup {
        windup: windup_clip.clone(),
    };
}

/// Hit animation event, the protagonsit hits the antagonist
fn finisher(
    animator: &mut FightAnimator,
    animation_player: &mut AnimationPlayer,
    animation_transition: &mut AnimationTransitions,
) {
    /* Get the correct windup anim from the fighting state */
    let finisher_clip = match &animator.fighting_state {
        FightingState::Windup { windup } => windup.finisher,
        _ => {
            tracing::warn!("Finisher animation from non windup state, picking random animation");
            use rand::seq::IndexedRandom;
            let mut rng = rand::rng();
            let Some(windup_clip) = animator.collection.windups.choose(&mut rng) else {
                tracing::warn!("No windup animations clip: unable to play windup animation");
                return;
            };
            windup_clip.finisher
        }
    };

    /* Play the windup */
    animation_transition.play(animation_player, finisher_clip, ANIM_TRANSITION_DURATION);

    /* Engaged from the moment a hit starts, so rapid typing always chains */
    animator.fighting_state = FightingState::Finished;
}

/// Hit animation event, the protagonsit hits the antagonist
fn counter(
    animator: &mut FightAnimator,
    animation_player: &mut AnimationPlayer,
    animation_transition: &mut AnimationTransitions,
) {
    /* Get the correct windup anim from the fighting state */
    let counter_clip = match &animator.fighting_state {
        FightingState::Windup { windup } => windup.counter,
        _ => {
            tracing::warn!("Counter animation from non windup state, picking random animation");
            use rand::seq::IndexedRandom;
            let mut rng = rand::rng();
            let Some(windup_clip) = animator.collection.windups.choose(&mut rng) else {
                tracing::warn!("No windup animations clip: unable to play windup animation");
                return;
            };
            windup_clip.counter
        }
    };

    /* Play the windup */
    animation_transition.play(animation_player, counter_clip, ANIM_TRANSITION_DURATION);

    /* Engaged from the moment a hit starts, so rapid typing always chains */
    animator.fighting_state = FightingState::Countered;
}

/// Observer to handle animation events to get the state back to distant
fn on_back_to_distance(event: On<BackToDistance>, rig: Single<&AnimationTransitions>, mut animator: If<ResMut<FightAnimator>>) {
    if rig.get_main_animation() == Some(event.node) {
        animator.fighting_state = FightingState::Idle;
    }
}
