use std::sync::Arc;

use crate::bevy_scene_plugin::BevyScenePlugin;
use bevy::{
    prelude::*,
    render::{
        camera::RenderTarget,
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        renderer::{
            RenderAdapter, RenderAdapterInfo, RenderDevice, RenderInstance, RenderQueue,
            WgpuWrapper,
        },
        settings::{RenderCreation, RenderResources},
        view::screenshot::{Screenshot, ScreenshotCaptured},
        RenderPlugin,
    },
};
use dioxus_native::{CustomPaintCtx, DeviceHandle, TextureHandle};
use wgpu::Instance;

#[derive(Resource, Default)]
pub struct UIData {
    pub width: u32,
    pub height: u32,
    pub color: [f32; 3],
}

pub struct BevyRenderer {
    app: App,
    texture_handle: Option<TextureHandle>,
    wgpu_texture: Option<wgpu::Texture>,
    wgpu_device: wgpu::Device,
    wgpu_queue: wgpu::Queue,
    last_texture_size: (u32, u32),
}

impl BevyRenderer {
    pub fn new(instance: &Instance, device_handle: &DeviceHandle) -> Self {
        // Create a headless Bevy App
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .set(RenderPlugin {
                    render_creation: RenderCreation::Manual(RenderResources(
                        RenderDevice::new(WgpuWrapper::new(device_handle.device.clone())),
                        RenderQueue(Arc::new(WgpuWrapper::new(device_handle.queue.clone()))),
                        RenderAdapterInfo(WgpuWrapper::new(device_handle.adapter.get_info())),
                        RenderAdapter(Arc::new(WgpuWrapper::new(device_handle.adapter.clone()))),
                        RenderInstance(Arc::new(WgpuWrapper::new(instance.clone()))),
                    )),
                    synchronous_pipeline_compilation: true,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    close_when_requested: false,
                })
                .disable::<bevy::winit::WinitPlugin>(),
        );

        // Add data from the UI
        app.insert_resource(UIData::default());

        // Setup the rendering to texture
        let render_target_image = Handle::<Image>::default();
        app.insert_resource(RenderTargetImage(render_target_image))
            .insert_resource(RenderedTextureData::default())
            .insert_resource(SceneReady::default())
            .add_systems(
                Update,
                (
                    mark_scene_ready,
                    update_render_target_size,
                    request_screenshot,
                ),
            )
            .add_systems(Last, update_camera_render_target)
            .add_observer(handle_screenshot_captured);

        // Add the scene
        app.add_plugins(BevyScenePlugin {});

        // Initialize the app to set up render world properly
        app.finish();
        app.cleanup();

        Self {
            app,
            texture_handle: None,
            wgpu_texture: None,
            wgpu_device: device_handle.device.clone(),
            wgpu_queue: device_handle.queue.clone(),
            last_texture_size: (0, 0),
        }
    }

    pub fn render(
        &mut self,
        ctx: CustomPaintCtx<'_>,
        color: [f32; 3],
        width: u32,
        height: u32,
        _start_time: &std::time::Instant,
    ) -> Option<TextureHandle> {
        // Update the UI data
        if let Some(mut ui) = self.app.world_mut().get_resource_mut::<UIData>() {
            ui.width = width;
            ui.height = height;
            ui.color = color;
        }

        // Run one frame of the Bevy app to render the 3D scene, and update the texture.
        self.app.update();
        self.update_texture(ctx);

        self.texture_handle
    }

    fn update_texture(&mut self, mut ctx: CustomPaintCtx<'_>) {
        // Copy the rendered content from Bevy's render target to our WGPU texture
        if let Some(rendered_data) = self.app.world().get_resource::<RenderedTextureData>() {
            if let Some(image_data) = &rendered_data.data {
                let width = rendered_data.width;
                let height = rendered_data.height;

                // Create/recreate texture if it doesn't exist or size changed
                let current_size = (width, height);
                if self.texture_handle.is_none() || self.last_texture_size != current_size {
                    println!("Creating WGPU texture {width}x{height}");
                    self.last_texture_size = current_size;

                    // Create a WGPU texture for dioxus
                    let wgpu_texture = self.wgpu_device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("Bevy 3D Render Target"),
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING
                            | wgpu::TextureUsages::COPY_DST
                            | wgpu::TextureUsages::COPY_SRC,
                        view_formats: &[],
                    });
                    self.texture_handle = Some(ctx.register_texture(wgpu_texture.clone()));
                    self.wgpu_texture = Some(wgpu_texture);
                }

                // Copy texture data to WGPU
                let bytes_per_row = width * 4; // 4 bytes per pixel (RGBA8)
                self.wgpu_queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: self.wgpu_texture.as_ref().unwrap(),
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    image_data,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: None,
                    },
                    wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                );

                return;
            }
        }

        self.texture_handle = None;
    }
}

#[derive(Resource)]
struct RenderTargetImage(pub Handle<Image>);

#[derive(Resource, Default)]
struct RenderedTextureData {
    pub data: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub pending_screenshot: bool,
}

#[derive(Resource, Default)]
struct SceneReady(pub bool);

fn mark_scene_ready(mut scene_ready: ResMut<SceneReady>, camera_query: Query<&Camera>) {
    if !scene_ready.0 {
        scene_ready.0 = camera_query.iter().count() > 0;
    }
}

fn update_render_target_size(
    mut images: ResMut<Assets<Image>>,
    mut render_target_res: ResMut<RenderTargetImage>,
    ui: Res<UIData>,
    mut last_size: Local<(u32, u32)>,
) {
    // Only recreate the render target if the size changed and we have valid dimensions
    if ui.width == 0 || ui.height == 0 {
        return;
    }

    let current_size = (ui.width, ui.height);
    if *last_size == current_size {
        return;
    }

    println!("Updating render target size to {}x{}", ui.width, ui.height);
    *last_size = current_size;

    // Create the render target image with the new size
    let mut image = Image::new_fill(
        Extent3d {
            width: ui.width,
            height: ui.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 4], // Black fill
        TextureFormat::bevy_default(),
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.texture_descriptor.usage =
        TextureUsages::COPY_SRC | TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;

    // Update the handle with the new image
    let handle = images.add(image);
    render_target_res.0 = handle;
}

fn request_screenshot(
    mut commands: Commands,
    render_target: Res<RenderTargetImage>,
    mut texture_data: ResMut<RenderedTextureData>,
    scene_ready: Res<SceneReady>,
) {
    if scene_ready.0 && !texture_data.pending_screenshot {
        commands.spawn(Screenshot::image(render_target.0.clone()));
        texture_data.pending_screenshot = true;
    }
}

fn update_camera_render_target(
    mut cameras: Query<&mut Camera>,
    render_target_res: Res<RenderTargetImage>,
    scene_ready: Res<SceneReady>,
    images: Res<Assets<Image>>,
    mut last_handle: Local<Option<Handle<Image>>>,
) {
    // Only set camera target after scene is ready and image exists
    if !scene_ready.0 || images.get(&render_target_res.0).is_none() {
        return;
    }

    // Update camera target if the render target handle changed
    if last_handle.as_ref() != Some(&render_target_res.0) {
        for mut camera in cameras.iter_mut() {
            camera.target = RenderTarget::Image(render_target_res.0.clone().into());
            println!("Updated camera target to render target image");
        }
        *last_handle = Some(render_target_res.0.clone());
    }
}

fn handle_screenshot_captured(
    trigger: Trigger<ScreenshotCaptured>,
    mut texture_data: ResMut<RenderedTextureData>,
) {
    // Get raw data from Bevy Image
    let captured_image = &trigger.event().0;
    if let Some(data) = &captured_image.data {
        texture_data.data = Some(data.clone());
        texture_data.width = captured_image.width();
        texture_data.height = captured_image.height();
        texture_data.pending_screenshot = false;
    } else {
        texture_data.pending_screenshot = false;
    }
}
