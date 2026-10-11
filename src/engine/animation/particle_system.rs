use bevy::prelude::*;

/// How the size evolves over the particle life
#[derive(Clone, Copy)]
pub enum Profile {
    /// Holds its size, then shrinks to nothing
    Shrink,
    /// Overshoots to full size almost instantly, then collapses
    Pop,
}

/// Where the long axis (+Y) of the particle points
#[derive(Clone, Copy)]
pub enum Orient {
    AlongVelocity,
    Fixed(Quat),
}

#[derive(Clone)]
#[derive(Component)]
pub struct Fx {
    pub position: Vec3,
    pub velocity: Vec3,
    /// Exponential velocity damping
    pub drag: f32,
    /// Downward acceleration
    pub gravity: f32,
    pub age: f32,
    pub lifetime: f32,
    /// Base size: x/z thickness, y length
    pub size: Vec3,
    /// Extra length per unit of speed (motion stretch)
    pub stretch: f32,
    pub orient: Orient,
    pub profile: Profile,
    /// Grows out from `position` instead of being centered on it
    pub anchored: bool,
}

impl Fx {
    pub fn at(position: Vec3) -> Self {
        Fx {
            position,
            velocity: Vec3::ZERO,
            drag: 0.0,
            gravity: 0.0,
            age: 0.0,
            lifetime: 0.2,
            size: Vec3::splat(0.05),
            stretch: 0.0,
            orient: Orient::AlongVelocity,
            profile: Profile::Shrink,
            anchored: false,
        }
    }

    pub fn transform(&self) -> Transform {
        let t = (self.age / self.lifetime).clamp(0.0, 1.0);
        let k = match self.profile {
            Profile::Shrink => 1.0 - t * t,
            Profile::Pop if t < 0.15 => 0.6 + 0.7 * t / 0.15,
            Profile::Pop => 1.3 * (1.0 - (t - 0.15) / 0.85).powi(2),
        };
        let rotation = match self.orient {
            Orient::AlongVelocity => Quat::from_rotation_arc(Vec3::Y, self.velocity.try_normalize().unwrap_or(Vec3::Y)),
            Orient::Fixed(rotation) => rotation,
        };
        let length = (self.size.y + self.velocity.length() * self.stretch) * k;
        let offset = match self.anchored {
            true => rotation * Vec3::Y * length * 0.5,
            false => Vec3::ZERO,
        };
        Transform {
            translation: self.position + offset,
            rotation,
            scale: vec3(self.size.x * k, length, self.size.z * k),
        }
    }
}

#[derive(Resource)]
pub struct FxAssets {
    pub cube: Handle<Mesh>,
    pub sphere: Handle<Mesh>,
    pub hot: Handle<StandardMaterial>,
    pub yellow: Handle<StandardMaterial>,
    pub orange: Handle<StandardMaterial>,
}

pub fn add_plugins(app: &mut App) {
    app.add_systems(Startup, setup_fx);
    app.add_systems(Update, update_fx);
}

fn setup_fx(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    /* Values above 1.0 survive tonemapping and glow with bloom */
    let mut unlit = |color: LinearRgba| {
        materials.add(StandardMaterial {
            base_color: color.into(),
            unlit: true,
            ..default()
        })
    };
    commands.insert_resource(FxAssets {
        cube: meshes.add(Cuboid::from_length(1.0)),
        sphere: meshes.add(Sphere::new(0.5)),
        hot: unlit(LinearRgba::rgb(6.0, 6.0, 6.0)),
        yellow: unlit(LinearRgba::rgb(5.0, 3.5, 0.8)),
        orange: unlit(LinearRgba::rgb(4.0, 1.2, 0.2)),
    });
}

pub fn spawn_fx(commands: &mut Commands, mesh: &Handle<Mesh>, material: &Handle<StandardMaterial>, fx: Fx) {
    commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), fx.transform(), fx));
}

fn update_fx(time: Res<Time>, mut particles: Query<(Entity, &mut Fx, &mut Transform)>, mut commands: Commands) {
    let dt = time.delta_secs();
    for (entity, mut fx, mut transform) in &mut particles {
        fx.age += dt;
        if fx.age >= fx.lifetime {
            commands.entity(entity).despawn();
            continue;
        }
        let velocity = fx.velocity;
        fx.position += velocity * dt;
        let vel_factor = (-fx.drag * dt).exp();
        fx.velocity *= vel_factor;
        fx.velocity.y -= fx.gravity * dt;
        *transform = fx.transform();
    }
}
