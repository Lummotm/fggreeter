use bevy::prelude::*;

/// Path of the background image asset
const BACKGROUND_ASSET_PATH: &str = "images/background.png";

/// Background waiting for its image to load, to be sized from the image aspect ratio
#[derive(Component)]
struct UnfittedBackground(Handle<Image>);

pub fn add_plugins(app: &mut App) {
    app.add_systems(Startup, spawn_background);
    app.add_systems(Update, fit_background);
}

fn spawn_background(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let image: Handle<Image> = asset_server.load(BACKGROUND_ASSET_PATH);
    let quad_pos = crate::engine::camera::SCENE_CENTER + vec3(0.0, 0.0, -4.0);
    commands.spawn((
        crate::engine::animation::HiddenOnImpact,
        UnfittedBackground(image.clone()),
        Mesh3d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(image),
            unlit: true,
            ..default()
        })),
        Transform::from_translation(quad_pos),
        /* Hidden until sized, so the quad doesn't flash at the wrong size */
        Visibility::Hidden,
    ));
}

/// Once the image is loaded, scale the quad to fill the scene height and keep the image proportions
fn fit_background(
    images: Res<Assets<Image>>,
    mut backgrounds: Query<(Entity, &UnfittedBackground, &mut Transform, &mut Visibility)>,
    mut commands: Commands,
) {
    for (entity, background, mut transform, mut visibility) in &mut backgrounds {
        let Some(image) = images.get(&background.0) else { continue };
        if image.height() == 0 {
            continue;
        }

        let h = crate::engine::camera::SCENE_HEIGHT;
        let aspect = image.width() as f32 / image.height() as f32;
        transform.scale = vec3(h * aspect, h, 1.0);
        *visibility = Visibility::Inherited;

        commands.entity(entity).remove::<UnfittedBackground>();
    }
}
