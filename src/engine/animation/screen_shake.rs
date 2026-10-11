use bevy::prelude::*;

/// Max offset of the panel at full trauma, in pixels
const UI_SHAKE_MAX_OFFSET: f32 = 12.0;
/// Trauma added per hit (capped at 1)
const UI_SHAKE_TRAUMA_PER_HIT: f32 = 0.6;
/// Trauma lost per second
const UI_SHAKE_DECAY: f32 = 3.0;

/// Max camera offset at full trauma, in world units
const CAMERA_SHAKE_MAX_OFFSET: f32 = 0.04;
/// Trauma lost per second
const CAMERA_SHAKE_DECAY: f32 = 3.0;
/// Trauma added by a regular hit and by the big blows
const CAMERA_TRAUMA_HIT: f32 = 0.4;
const CAMERA_TRAUMA_SUPER: f32 = 1.0;

/// Put this on the login panel node
#[derive(Component)]
pub struct ShakeOnHit;

#[derive(Resource, Default)]
struct UiShake {
    trauma: f32,
}

pub fn add_plugins(app: &mut App) {
    app.init_resource::<UiShake>();
    app.init_resource::<CameraShake>();
    app.add_systems(Update, update_ui_shake);
    app.add_systems(Update, update_camera_shake);
    app.add_observer(on_hit_ui_shake);
    app.add_observer(on_hit_camera_shake);
    app.add_observer(on_finisher_camera_shake);
    app.add_observer(on_counter_camera_shake);
}

fn on_hit_ui_shake(_: On<super::HitImpact>, mut shake: ResMut<UiShake>) {
    shake.trauma = (shake.trauma + UI_SHAKE_TRAUMA_PER_HIT).min(3.0);
}

fn update_ui_shake(time: Res<Time>, mut shake: ResMut<UiShake>, mut panels: Query<&mut Node, With<ShakeOnHit>>) {
    shake.trauma = (shake.trauma - UI_SHAKE_DECAY * time.delta_secs()).max(0.0);

    use rand::RngExt;
    let mut rng = rand::rng();
    let amount = UI_SHAKE_MAX_OFFSET * shake.trauma * shake.trauma;
    for mut node in &mut panels {
        node.left = Val::Px(amount * rng.random_range(-1.0..1.0));
        node.top = Val::Px(amount * rng.random_range(-1.0..1.0));
    }
}

#[derive(Resource, Default)]
struct CameraShake {
    trauma: f32,
    /// Offset applied last frame, removed before applying the next one
    offset: Vec3,
}

impl CameraShake {
    fn add(&mut self, trauma: f32) {
        self.trauma = (self.trauma + trauma).min(1.0);
    }
}

fn on_hit_camera_shake(_event: On<super::HitImpact>, mut shake: ResMut<CameraShake>) {
    shake.add(CAMERA_TRAUMA_HIT);
}

fn on_finisher_camera_shake(_event: On<super::FinisherImpact>, mut shake: ResMut<CameraShake>) {
    shake.add(CAMERA_TRAUMA_SUPER);
}

fn on_counter_camera_shake(_event: On<super::CounterImpact>, mut shake: ResMut<CameraShake>) {
    shake.add(CAMERA_TRAUMA_SUPER);
}

fn update_camera_shake(time: Res<Time>, mut shake: ResMut<CameraShake>, mut cameras: Query<&mut Transform, With<Camera3d>>) {
    use rand::RngExt;

    shake.trauma = (shake.trauma - CAMERA_SHAKE_DECAY * time.delta_secs()).max(0.0);
    let amount = CAMERA_SHAKE_MAX_OFFSET * shake.trauma * shake.trauma;
    let mut rng = rand::rng();

    let Ok(mut transform) = cameras.single_mut() else { return };
    /* Shake in the camera plane so it never zooms */
    let offset = (*transform.right() * rng.random_range(-1.0..1.0) + *transform.up() * rng.random_range(-1.0..1.0)) * amount;
    transform.translation += offset - shake.offset;
    shake.offset = offset;
}
