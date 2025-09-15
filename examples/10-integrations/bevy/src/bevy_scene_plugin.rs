use crate::bevy_renderer::UIData;
use bevy::prelude::*;

#[derive(Component)]
pub struct DynamicColoredCube;

pub struct BevyScenePlugin {}

impl Plugin for BevyScenePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(bevy::color::Color::srgba(0.0, 0.0, 0.0, 0.0)));
        app.add_systems(Startup, setup);
        app.add_systems(Update, (animate, update_cube_color));
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: bevy::color::Color::srgb(1.0, 0.0, 0.0),
            metallic: 0.0,
            perceptual_roughness: 0.5,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        DynamicColoredCube,
    ));

    commands.spawn((
        DirectionalLight {
            color: bevy::color::Color::WHITE,
            illuminance: 10000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.insert_resource(AmbientLight {
        color: bevy::color::Color::WHITE,
        brightness: 100.0,
        affects_lightmapped_meshes: true,
    });

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 3.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        Name::new("MainCamera"),
    ));
}

fn animate(time: Res<Time>, mut cube_query: Query<&mut Transform, With<DynamicColoredCube>>) {
    for mut transform in cube_query.iter_mut() {
        transform.rotation = Quat::from_rotation_y(time.elapsed_secs());
        transform.translation.x = (time.elapsed_secs() * 2.0).sin() * 0.5;
    }
}

fn update_cube_color(
    ui: Res<UIData>,
    cube_query: Query<&MeshMaterial3d<StandardMaterial>, With<DynamicColoredCube>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if ui.is_changed() {
        for mesh_material in cube_query.iter() {
            if let Some(material) = materials.get_mut(&mesh_material.0) {
                let [r, g, b] = ui.color;
                material.base_color = bevy::color::Color::srgb(r, g, b);
            }
        }
    }
}
