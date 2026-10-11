use bevy::prelude::*;

/// On a fighter mesh: which fighter it belongs to, and its original material restored after the impact frame
#[derive(Component)]
pub enum FighterMaterial {
    Antagonist(Handle<StandardMaterial>),
    Protagonist(Handle<StandardMaterial>),
}

impl FighterMaterial {
    /// Build from the name of the fighter node the mesh belongs to, None if it isn't a fighter node
    pub fn from_node_name(name: &str, original: Handle<StandardMaterial>) -> Option<Self> {
        match name {
            "PBody" | "PHead" => Some(FighterMaterial::Protagonist(original)),
            "ABody" | "AHead" => Some(FighterMaterial::Antagonist(original)),
            _ => None,
        }
    }

    fn original(&self) -> &Handle<StandardMaterial> {
        match self {
            FighterMaterial::Antagonist(handle) | FighterMaterial::Protagonist(handle) => handle,
        }
    }

    fn is_lit(&self, protagonist: bool, antagonist: bool) -> bool {
        match self {
            FighterMaterial::Protagonist(_) => protagonist,
            FighterMaterial::Antagonist(_) => antagonist,
        }
    }
}

/// Put this on everything that must disappear during the impact frame (the background, ...)
#[derive(Component)]
pub struct HiddenOnImpact;

/// Present while an impact frame is running
#[derive(Resource)]
struct ImpactFrame {
    timer: Timer,
    restore_bg: bool,
}

/// Shared white material for the impact frame
#[derive(Resource)]
struct ImpactAssets {
    white: Handle<StandardMaterial>,
    black: Handle<StandardMaterial>,
}

pub fn add_plugins(app: &mut App) {
    app.add_systems(Startup, setup_impact);
    app.add_systems(Update, end_impact_frame);
}

fn setup_impact(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.insert_resource(ImpactAssets {
        white: materials.add(StandardMaterial {
            /* Above 1.0 so tonemapping doesn't turn it grey */
            base_color: LinearRgba::rgb(4.0, 4.0, 4.0).into(),
            unlit: true,
            ..default()
        }),
        black: materials.add(StandardMaterial {
            base_color: LinearRgba::rgb(0.0, 0.0, 0.0).into(),
            unlit: true,
            ..default()
        }),
    });
}

/// Everything needed to start an impact frame, usable from any system or observer
#[derive(bevy::ecs::system::SystemParam)]
pub struct ImpactFrameStarter<'w, 's> {
    assets: Res<'w, ImpactAssets>,
    fighters: Query<'w, 's, (&'static mut MeshMaterial3d<StandardMaterial>, &'static FighterMaterial)>,
    hidden: Query<'w, 's, &'static mut Visibility, With<HiddenOnImpact>>,
    cameras: Query<'w, 's, &'static mut Camera, With<Camera3d>>,
}

impl ImpactFrameStarter<'_, '_> {
    /// Start an impact frame: lit fighters go white, the others black, the rest of the scene black.
    /// Calling it again during a running impact frame overrides the colors and restarts the timer.
    pub fn start(
        &mut self,
        commands: &mut Commands,
        duration: std::time::Duration,
        protagonist: bool,
        antagonist: bool,
        restore_bg: bool,
    ) {
        for (mut material, fighter_material) in &mut self.fighters {
            let lit = fighter_material.is_lit(protagonist, antagonist);
            material.0 = if lit {
                self.assets.white.clone()
            } else {
                self.assets.black.clone()
            };
        }
        for mut visibility in &mut self.hidden {
            *visibility = Visibility::Hidden;
        }
        for mut camera in &mut self.cameras {
            camera.clear_color = ClearColorConfig::Custom(Color::BLACK);
        }

        /* Inserting again restarts the timer */
        commands.insert_resource(ImpactFrame {
            timer: Timer::new(duration, TimerMode::Once),
            restore_bg,
        });
    }
}

fn end_impact_frame(
    time: Res<Time>,
    assets: Res<ImpactAssets>,
    mut frame: If<ResMut<ImpactFrame>>,
    mut fighters: Query<(&mut MeshMaterial3d<StandardMaterial>, &FighterMaterial)>,
    mut hidden: Query<&mut Visibility, With<HiddenOnImpact>>,
    mut cameras: Query<&mut Camera, With<Camera3d>>,
    mut commands: Commands,
) {
    if !frame.0.timer.tick(time.delta()).is_finished() {
        return;
    }

    for (mut material, fighter_material) in &mut fighters {
        material.0 = if frame.0.restore_bg {
            fighter_material.original().clone()
        } else {
            assets.black.clone()
        };
    }
    if frame.0.restore_bg {
        for mut visibility in &mut hidden {
            *visibility = Visibility::Inherited;
        }
        for mut camera in &mut cameras {
            camera.clear_color = ClearColorConfig::Default;
        }
    }
    commands.remove_resource::<ImpactFrame>();
}
