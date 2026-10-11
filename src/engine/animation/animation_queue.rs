use crate::engine::animation;
use bevy::prelude::*;

#[derive(Clone, Default)]
#[derive(Resource)]
pub struct AnimationQueue {
    queue: Vec<(std::time::Duration, AnimationQueueEvent)>,
}

impl AnimationQueue {
    pub fn impact_frame(
        &mut self,
        timer: std::time::Duration,
        impact_frame_duration: std::time::Duration,
        protagonist: bool,
        antagonist: bool,
        restore_bg: bool,
    ) {
        self.queue.push((
            timer,
            AnimationQueueEvent::ImpactFrame {
                duration: impact_frame_duration,
                protagonist,
                antagonist,
                restore_bg,
            },
        ));
    }

    pub fn particles(
        &mut self,
        timer: std::time::Duration,
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        particle_fx: animation::particle_system::Fx,
    ) {
        let event = AnimationQueueEvent::Particles {
            mesh,
            material,
            particle_fx,
        };
        self.queue.push((timer, event));
    }

    fn reached_events(&mut self, dt: std::time::Duration) -> impl Iterator<Item = AnimationQueueEvent> {
        self.queue
            .extract_if(.., move |(remaining, _)| match remaining.checked_sub(dt) {
                Some(new_rem) => {
                    *remaining = new_rem;
                    false
                }
                None => true,
            })
            .map(|(_, event)| event)
    }
}

#[derive(Clone)]
pub enum AnimationQueueEvent {
    ImpactFrame {
        duration: std::time::Duration,
        protagonist: bool,
        antagonist: bool,
        restore_bg: bool,
    },
    Particles {
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        particle_fx: crate::engine::animation::particle_system::Fx,
    },
}

pub fn add_plugins(app: &mut App) {
    app.init_resource::<AnimationQueue>();
    app.add_systems(Update, update_animation_queue);
}

fn update_animation_queue(
    time: Res<Time>,
    mut queue: ResMut<AnimationQueue>,
    mut commands: Commands,
    mut impact_frame: animation::impact_frame::ImpactFrameStarter,
) {
    for event in queue.reached_events(time.delta()) {
        match event {
            AnimationQueueEvent::ImpactFrame {
                duration,
                protagonist,
                antagonist,
                restore_bg,
            } => impact_frame.start(&mut commands, duration, protagonist, antagonist, restore_bg),
            AnimationQueueEvent::Particles {
                mesh,
                material,
                particle_fx,
            } => animation::particle_system::spawn_fx(&mut commands, &mesh, &material, particle_fx),
        }
    }
}
