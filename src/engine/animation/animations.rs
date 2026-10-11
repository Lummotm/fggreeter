use crate::engine::animation;
use animation::animation_queue::AnimationQueue;
use animation::particle_system::Fx;
use animation::particle_system::FxAssets;
use animation::particle_system::Orient;
use animation::particle_system::Profile;
use bevy::prelude::*;
use rand::Rng;
use rand::RngExt;
use std::f32::consts::TAU;
use std::time::Duration;

/// How long the impact frame lasts
pub const FRAME_DURATION: Duration = Duration::from_millis(1000 * 1 / animation::ANIM_FPS);
/// Duration of the final animation
pub const FINISHER_ANIM_TIME: Duration = Duration::from_millis(1000 * 20 / animation::ANIM_FPS);

/// Global size multiplier of the impact effect, tune to the scene scale
const IMPACT_SCALE: f32 = 1.0;

pub fn add_plugins(app: &mut App) {
    app.add_observer(on_hit_impact);
    app.add_observer(on_finisher_impact);
    app.add_observer(on_counter_impact);
    app.add_observer(on_windup_charge);
}

/// Time of the n-th animation frame after the impact
fn frame(n: u32) -> Duration {
    FRAME_DURATION * n
}

/// Where the impact happens and how it faces the camera
struct Impact {
    origin: Vec3,
    dir: Vec3,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
    s: f32,
}

impl Impact {
    fn read(
        hit_point: &Query<&GlobalTransform, With<super::HitPoint>>,
        camera: &Query<&GlobalTransform, With<Camera3d>>,
    ) -> Option<Self> {
        let Ok(hit_point) = hit_point.single() else {
            tracing::warn!("Impact fx: no HitPoint bone");
            return None;
        };
        let Ok(camera) = camera.single() else {
            tracing::warn!("Impact fx: expected exactly one Camera3d, found {}", camera.iter().count());
            return None;
        };
        Some(Impact {
            origin: hit_point.translation(),
            dir: *hit_point.up(),
            right: *camera.right(),
            up: *camera.up(),
            forward: *camera.forward(),
            s: IMPACT_SCALE,
        })
    }
}

/// Which fighters an impact frame lights up, relative to who strikes
#[derive(Clone, Copy)]
enum Lit {
    Attacker,
    Defender,
    Both,
}

/// Queue an impact frame; `swapped` means the antagonist is the attacker
fn impact_frame(queue: &mut AnimationQueue, at: Duration, lit: Lit, swapped: bool, restore_bg: bool) {
    let (attacker, defender) = match lit {
        Lit::Attacker => (true, false),
        Lit::Defender => (false, true),
        Lit::Both => (true, true),
    };
    let (protagonist, antagonist) = match swapped {
        true => (defender, attacker),
        false => (attacker, defender),
    };
    queue.impact_frame(at, FRAME_DURATION, protagonist, antagonist, restore_bg);
}

fn on_hit_impact(
    _event: On<super::HitImpact>,
    hit_point: Query<&GlobalTransform, With<super::HitPoint>>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    assets: If<Res<FxAssets>>,
    mut animation_queue: ResMut<AnimationQueue>,
) {
    let Some(impact) = Impact::read(&hit_point, &camera) else {
        return;
    };
    let queue = &mut *animation_queue;
    let mut rng = rand::rng();

    impact_frame(queue, frame(0), Lit::Attacker, false, true);
    core_flash(queue, &assets, &impact, frame(0));
    impact_star(queue, &assets, &impact, frame(0), &mut rng);
    ellipses(queue, &assets, &impact, frame(0), 2, &mut rng);

    sparks(queue, &assets, &impact, frame(1), &mut rng);
    embers(queue, &assets, &impact, frame(1), &mut rng);

    impact_frame(queue, frame(2), Lit::Defender, false, true);
}

fn on_finisher_impact(
    _event: On<super::FinisherImpact>,
    hit_point: Query<&GlobalTransform, With<super::HitPoint>>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    assets: If<Res<FxAssets>>,
    mut animation_queue: ResMut<AnimationQueue>,
) {
    let Some(impact) = Impact::read(&hit_point, &camera) else {
        return;
    };
    finisher_counter_impact(&mut animation_queue, &assets, &impact, false, &mut rand::rng());
}

fn on_counter_impact(
    _event: On<super::CounterImpact>,
    hit_point: Query<&GlobalTransform, With<super::HitPoint>>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    assets: If<Res<FxAssets>>,
    mut animation_queue: ResMut<AnimationQueue>,
) {
    let Some(impact) = Impact::read(&hit_point, &camera) else {
        return;
    };
    finisher_counter_impact(&mut animation_queue, &assets, &impact, true, &mut rand::rng());
}

fn on_windup_charge(
    _event: On<super::WindupCharge>,
    hit_point: Query<&GlobalTransform, With<super::HitPoint>>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    assets: If<Res<FxAssets>>,
    mut animation_queue: ResMut<AnimationQueue>,
) {
    let Some(impact) = Impact::read(&hit_point, &camera) else {
        return;
    };
    charge(&mut animation_queue, &assets, &impact, frame(0), &mut rand::rng());
}

/// The big blow sequence, shared by the finisher and the counter (`swapped`)
fn finisher_counter_impact(queue: &mut AnimationQueue, assets: &FxAssets, impact: &Impact, swapped: bool, rng: &mut impl Rng) {
    let mut fc = 0;

    impact_frame(queue, frame(fc), Lit::Attacker, swapped, false);
    ellipses(queue, assets, impact, frame(fc), 1, rng);

    fc += 2;
    impact_frame(queue, frame(fc), Lit::Attacker, swapped, false);
    ellipses(queue, assets, impact, frame(fc), 1, rng);

    fc += 2;
    impact_frame(queue, frame(fc), Lit::Both, swapped, false);
    ellipses(queue, assets, impact, frame(fc), 2, rng);

    fc += 2;
    impact_frame(queue, frame(fc), Lit::Defender, swapped, false);
    core_flash(queue, assets, impact, frame(fc));
    impact_star(queue, assets, impact, frame(fc), rng);
    sparks(queue, assets, impact, frame(fc), rng);
    embers(queue, assets, impact, frame(fc), rng);

    for _ in 0..6 {
        fc += 3;
        impact_frame(queue, frame(fc), Lit::Defender, swapped, false);
    }

    fc += 2;
    /* Hacky: if swapped, we restore the background since that's a counter */
    impact_frame(queue, frame(fc), Lit::Defender, swapped, swapped);
    ellipses(queue, assets, impact, frame(fc), 2, rng);
}

/// Core flash at the impact point
fn core_flash(queue: &mut AnimationQueue, assets: &FxAssets, impact: &Impact, at: Duration) {
    queue.particles(
        at,
        assets.sphere.clone(),
        assets.hot.clone(),
        Fx {
            size: Vec3::splat(0.6 * impact.s),
            lifetime: 0.1,
            profile: Profile::Pop,
            ..Fx::at(impact.origin)
        },
    );
}

/// Impact star, in screen space, leaning in the hit direction
fn impact_star(queue: &mut AnimationQueue, assets: &FxAssets, impact: &Impact, at: Duration, rng: &mut impl Rng) {
    let s = impact.s;
    let spikes = rng.random_range(10..=16);
    let phase = rng.random_range(0.0..TAU);
    for i in 0..spikes {
        let angle = phase + TAU * i as f32 / spikes as f32 + rng.random_range(-0.25..0.25);
        let spike_dir = (impact.right * angle.cos() + impact.up * angle.sin() + impact.dir * 0.6).normalize();
        let length = rng.random_range(1.0..2.4) * s;
        let width = rng.random_range(0.02..0.05) * s;
        let material = if i % 2 == 0 {
            assets.hot.clone()
        } else {
            assets.yellow.clone()
        };
        queue.particles(
            at,
            assets.cube.clone(),
            material,
            Fx {
                size: vec3(width, length, width),
                lifetime: 0.15,
                orient: Orient::Fixed(Quat::from_rotation_arc(Vec3::Y, spike_dir)),
                profile: Profile::Pop,
                anchored: true,
                ..Fx::at(impact.origin)
            },
        );
    }
}

/// Shockwave rings, tilted toward the camera so they read as ellipses
fn ellipses(queue: &mut AnimationQueue, assets: &FxAssets, impact: &Impact, at: Duration, count: usize, rng: &mut impl Rng) {
    const RING_SHARDS: usize = 28;
    let ring_normal = (impact.dir - impact.forward * 0.8).normalize();
    let (ru, rv) = ring_normal.any_orthonormal_pair();
    for _ in 0..count {
        for i in 0..RING_SHARDS {
            let angle = TAU * i as f32 / RING_SHARDS as f32;
            let radial = ru * angle.cos() + rv * angle.sin();
            let tangent = ring_normal.cross(radial);
            let scale = rng.random_range(0.8..1.0) * impact.s;
            queue.particles(
                at,
                assets.cube.clone(),
                assets.hot.clone(),
                Fx {
                    velocity: radial * 14.0 * scale,
                    drag: 9.0,
                    size: vec3(0.05 * scale, 0.35 * scale, 0.05 * scale),
                    lifetime: 0.22,
                    orient: Orient::Fixed(Quat::from_rotation_arc(Vec3::Y, tangent)),
                    ..Fx::at(impact.origin + radial * 0.2 * scale)
                },
            );
        }
    }
}

/// Wide sparks plus a few long fast ones along the hit direction
fn sparks(queue: &mut AnimationQueue, assets: &FxAssets, impact: &Impact, at: Duration, rng: &mut impl Rng) {
    let s = impact.s;

    /* Wide sparks */
    for i in 0..16 {
        let material = if i % 3 == 0 {
            assets.hot.clone()
        } else {
            assets.yellow.clone()
        };
        let length = rng.random_range(0.1..0.2) * s;
        let width = rng.random_range(0.02..0.04) * s;
        queue.particles(
            at,
            assets.cube.clone(),
            material,
            Fx {
                velocity: in_cone(rng, impact.dir, 1.4) * rng.random_range(6.0..12.0) * s,
                drag: 5.0,
                gravity: 3.0 * s,
                size: vec3(width, length, width),
                stretch: 0.04,
                lifetime: rng.random_range(0.18..0.3),
                ..Fx::at(impact.origin)
            },
        );
    }

    /* Less sparks, narrow and fast */
    for i in 0..8 {
        let material = if i % 3 == 0 {
            assets.hot.clone()
        } else {
            assets.yellow.clone()
        };
        let length = rng.random_range(0.2..0.5) * s;
        let width = rng.random_range(0.02..0.04) * s;
        queue.particles(
            at,
            assets.cube.clone(),
            material,
            Fx {
                velocity: in_cone(rng, impact.dir, 0.2) * rng.random_range(4.0..30.0) * s,
                drag: 5.0,
                gravity: 3.0 * s,
                size: vec3(width, length, width),
                stretch: 0.04,
                lifetime: rng.random_range(0.18..0.35),
                ..Fx::at(impact.origin)
            },
        );
    }
}

/// Embers falling after the burst
fn embers(queue: &mut AnimationQueue, assets: &FxAssets, impact: &Impact, at: Duration, rng: &mut impl Rng) {
    let s = impact.s;
    for _ in 0..10 {
        queue.particles(
            at,
            assets.cube.clone(),
            assets.orange.clone(),
            Fx {
                velocity: in_cone(rng, impact.dir, 1.4) * rng.random_range(2.0..6.0) * s,
                drag: 1.5,
                gravity: 9.0 * s,
                size: Vec3::splat(0.06 * s),
                lifetime: rng.random_range(0.2..0.8),
                ..Fx::at(impact.origin)
            },
        );
    }
}

/// Charging burst: rays converging on the impact point, then a small flash
fn charge(queue: &mut AnimationQueue, assets: &FxAssets, impact: &Impact, at: Duration, rng: &mut impl Rng) {
    const RAYS: usize = 14;
    const RADIUS: f32 = 2.2;
    let s = impact.s;

    for i in 0..RAYS {
        let angle = TAU * i as f32 / RAYS as f32 + rng.random_range(-0.2..0.2);
        let out = impact.right * angle.cos() + impact.up * angle.sin();
        let radius = rng.random_range(0.8..1.0) * RADIUS * s;
        let lifetime = rng.random_range(0.12..0.2);
        let length = rng.random_range(0.4..0.9) * s;
        let width = rng.random_range(0.02..0.04) * s;
        let material = if i % 2 == 0 {
            assets.hot.clone()
        } else {
            assets.yellow.clone()
        };
        /* Constant speed so each ray reaches the center exactly as it vanishes */
        queue.particles(
            at + frame(rng.random_range(0..3)),
            assets.cube.clone(),
            material,
            Fx {
                velocity: -out * radius / lifetime,
                size: vec3(width, length, width),
                lifetime,
                ..Fx::at(impact.origin + out * radius)
            },
        );
    }

    /* Charged: small flash as the last rays arrive */
    queue.particles(
        at + frame(6),
        assets.sphere.clone(),
        assets.yellow.clone(),
        Fx {
            size: Vec3::splat(0.3 * s),
            lifetime: 0.1,
            profile: Profile::Pop,
            ..Fx::at(impact.origin)
        },
    );
}

/// Random unit vector in a cone of half-angle `spread` around `dir`
fn in_cone(rng: &mut impl Rng, dir: Vec3, spread: f32) -> Vec3 {
    let theta = rng.random_range(0.0..TAU);
    let phi = rng.random_range(0.0..spread);
    let local = Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
    Quat::from_rotation_arc(Vec3::Y, dir) * local
}
