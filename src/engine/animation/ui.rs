use crate::engine::ui::MainUiPanel;
use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

/// Anticipation: how long the panel dips down before launching
const EXIT_DIP_DURATION: f32 = 0.08;
/// How far it dips, in % of the viewport height
const EXIT_DIP_VH: f32 = 2.0;
/// Duration of the launch up
const EXIT_DURATION: f32 = 0.35;
/// How far it travels, in % of the viewport height (enough to fully leave the screen)
const EXIT_DISTANCE_VH: f32 = 120.0;

/// Present on the login panel while it's leaving the screen
#[derive(Component)]
struct Leaving {
    age: f32,
}

pub fn add_plugins(app: &mut App) {
    app.add_systems(Update, update_leaving);
    app.add_observer(on_finisher_impact);
}

/// The finisher knocks the panel off the screen
fn on_finisher_impact(_event: On<super::FinisherImpact>, panels: Query<Entity, With<MainUiPanel>>, mut commands: Commands) {
    for panel in &panels {
        commands.entity(panel).insert(Leaving { age: 0.0 });
    }
}

fn update_leaving(time: Res<Time>, mut panels: Query<(&mut Leaving, &mut UiTransform, &mut Visibility)>) {
    for (mut leaving, mut transform, mut visibility) in &mut panels {
        leaving.age += time.delta_secs();

        /* Dip down, then launch up with an ease-in so it accelerates out */
        let t_dip = (leaving.age / EXIT_DIP_DURATION).min(1.0);
        let t_exit = ((leaving.age - EXIT_DIP_DURATION) / EXIT_DURATION).clamp(0.0, 1.0);
        let dip = EXIT_DIP_VH * (t_dip * FRAC_PI_2).sin() * (1.0 - t_exit);
        let launch = EXIT_DISTANCE_VH * t_exit.powi(3);
        transform.translation = Val2::new(Val::ZERO, Val::Vh(dip - launch));

        /* Gone: stop drawing it */
        if t_exit >= 1.0 {
            *visibility = Visibility::Hidden;
        }
    }
}
