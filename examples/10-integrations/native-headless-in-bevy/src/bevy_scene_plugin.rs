use crate::ui::{UIMessage, UiState};
use bevy::input::mouse::{MouseButton, MouseMotion};
use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};

#[derive(Component)]
pub struct DynamicCube;

#[derive(Component)]
pub struct OrbitCamera {
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub sensitivity: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            distance: 3.0,
            yaw: 0.0,
            pitch: 0.0,
            sensitivity: 0.01,
        }
    }
}

#[derive(Resource, Deref)]
struct UIMessageSender(Sender<UIMessage>);

#[derive(Resource, Deref)]
struct UIMessageReceiver(Receiver<UIMessage>);

#[derive(Resource)]
struct CubeTranslationSpeed(f32);

impl Default for CubeTranslationSpeed {
    fn default() -> Self {
        Self(UiState::DEFAULT_CUBE_TRANSLATION_SPEED)
    }
}

#[derive(Resource)]
struct CubeRotationSpeed(f32);

impl Default for CubeRotationSpeed {
    fn default() -> Self {
        Self(UiState::DEFAULT_CUBE_ROTATION_SPEED)
    }
}

pub struct BevyScenePlugin {
    pub app_sender: Sender<UIMessage>,
    pub ui_receiver: Receiver<UIMessage>,
}

impl Plugin for BevyScenePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(bevy::color::Color::srgba(0.0, 0.0, 0.0, 0.0)));
        app.insert_resource(UIMessageSender(self.app_sender.clone()));
        app.insert_resource(UIMessageReceiver(self.ui_receiver.clone()));
        app.insert_resource(CubeTranslationSpeed::default());
        app.insert_resource(CubeRotationSpeed::default());
        app.add_systems(Startup, setup);
        app.add_systems(Update, (sync_with_ui, animate, orbit_camera_system));
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
            base_color: Color::Srgba(bevy::color::Srgba::from_f32_array(
                UiState::DEFAULT_CUBE_COLOR,
            )),
            metallic: 0.0,
            perceptual_roughness: 0.5,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        DynamicCube,
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
        OrbitCamera::default(),
    ));
}

fn sync_with_ui(
    sender: Res<UIMessageSender>,
    receiver: Res<UIMessageReceiver>,
    cube_query: Query<&MeshMaterial3d<StandardMaterial>, With<DynamicCube>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut translation_speed: ResMut<CubeTranslationSpeed>,
    mut rotation_speed: ResMut<CubeRotationSpeed>,
    time: Res<Time>,
) {
    let fps = 1000.0 / time.delta().as_millis() as f32;
    sender.send(UIMessage::Fps(fps)).unwrap();

    while let Ok(message) = receiver.try_recv() {
        match message {
            UIMessage::CubeColor(c) => {
                for cube_material in cube_query.iter() {
                    if let Some(material) = materials.get_mut(&cube_material.0) {
                        material.base_color = Color::Srgba(bevy::color::Srgba::from_f32_array(c));
                    }
                }
            }
            UIMessage::CubeTranslationSpeed(speed) => {
                translation_speed.0 = speed;
            }
            UIMessage::CubeRotationSpeed(speed) => {
                rotation_speed.0 = speed;
            }
            _ => {}
        }
    }
}

fn animate(
    time: Res<Time>,
    mut cube_query: Query<&mut Transform, With<DynamicCube>>,
    translation_speed: Res<CubeTranslationSpeed>,
    rotation_speed: Res<CubeRotationSpeed>,
) {
    for mut transform in cube_query.iter_mut() {
        transform.rotation = Quat::from_rotation_y(time.elapsed_secs() * rotation_speed.0);
        transform.translation.x = (time.elapsed_secs() * translation_speed.0).sin() * 0.5;
    }
}

fn orbit_camera_system(
    mut camera_query: Query<(&mut Transform, &mut OrbitCamera), With<Camera3d>>,
    mut mouse_motion_events: MessageReader<MouseMotion>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
) {
    for (mut transform, mut orbit_camera) in camera_query.iter_mut() {
        // Handle mouse input for camera rotation
        if mouse_button_input.pressed(MouseButton::Left) {
            for mouse_motion in mouse_motion_events.read() {
                orbit_camera.yaw -= mouse_motion.delta.x * orbit_camera.sensitivity;
                orbit_camera.pitch -= mouse_motion.delta.y * orbit_camera.sensitivity;

                // Clamp pitch to prevent camera flipping
                orbit_camera.pitch = orbit_camera.pitch.clamp(-1.5, 1.5);
            }
        }

        // Calculate camera position based on spherical coordinates
        let yaw_quat = Quat::from_rotation_y(orbit_camera.yaw);
        let pitch_quat = Quat::from_rotation_x(orbit_camera.pitch);

        let rotation = yaw_quat * pitch_quat;
        let position = rotation * Vec3::new(0.0, 0.0, orbit_camera.distance);

        transform.translation = position;
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}
