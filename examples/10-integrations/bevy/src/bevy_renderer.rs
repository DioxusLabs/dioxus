use crate::bevy_scene_plugin::BevyScenePlugin;
use bevy::{
    camera::{ManualTextureViewHandle, RenderTarget},
    prelude::*,
    render::{
        render_resource::TextureFormat,
        renderer::{
            RenderAdapter, RenderAdapterInfo, RenderDevice, RenderInstance, RenderQueue,
            WgpuWrapper,
        },
        settings::{RenderCreation, RenderResources},
        texture::ManualTextureView,
        RenderPlugin,
    },
};
use dioxus_native::{CustomPaintCtx, DeviceHandle, TextureHandle};
use std::sync::Arc;

#[derive(Resource, Default)]
pub struct UIData {
    pub color: [f32; 3],
}

pub struct BevyRenderer {
    app: App,
    wgpu_device: wgpu::Device,
    last_texture_size: (u32, u32),
    texture_handle: Option<TextureHandle>,
    manual_texture_view_handle: Option<ManualTextureViewHandle>,
}

impl BevyRenderer {
    pub fn new(device_handle: &DeviceHandle) -> Self {
        // Create a headless Bevy App.
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .set(RenderPlugin {
                    // Reuse the render resources from the Dioxus native renderer.
                    render_creation: RenderCreation::Manual(RenderResources(
                        RenderDevice::new(WgpuWrapper::new(device_handle.device.clone())),
                        RenderQueue(Arc::new(WgpuWrapper::new(device_handle.queue.clone()))),
                        RenderAdapterInfo(WgpuWrapper::new(device_handle.adapter.get_info())),
                        RenderAdapter(Arc::new(WgpuWrapper::new(device_handle.adapter.clone()))),
                        RenderInstance(Arc::new(WgpuWrapper::new(device_handle.instance.clone()))),
                    )),
                    synchronous_pipeline_compilation: true,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    close_when_requested: false,
                    ..Default::default()
                })
                .disable::<bevy::winit::WinitPlugin>(),
        );

        // Setup the rendering to texture.
        app.insert_resource(ManualTextureViews::default());

        // Add data from the UI.
        app.insert_resource(UIData::default());

        // Add the Bevy scene.
        app.add_plugins(BevyScenePlugin {});

        // Initialize the app to set up the render world properly.
        app.finish();
        app.cleanup();

        Self {
            app,
            wgpu_device: device_handle.device.clone(),
            last_texture_size: (0, 0),
            texture_handle: None,
            manual_texture_view_handle: None,
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
        // Update the UI data.
        if let Some(mut ui) = self.app.world_mut().get_resource_mut::<UIData>() {
            ui.color = color;
        }

        // Init self.texture_handle if None or if width/height changed.
        self.init_texture(ctx, width, height);
        // Run one frame of the Bevy app to render the 3D scene.
        self.app.update();

        self.texture_handle.clone()
    }

    fn init_texture(&mut self, mut ctx: CustomPaintCtx<'_>, width: u32, height: u32) {
        // Reuse self.texture_handle if already initialized to the correct size.
        let current_size = (width, height);
        if self.texture_handle.is_some() && self.last_texture_size == current_size {
            return;
        }

        let world = self.app.world_mut();

        // Skip if no camera.
        if world.query::<&Camera>().single(world).is_err() {
            return;
        }

        if let Some(mut manual_texture_views) = world.get_resource_mut::<ManualTextureViews>() {
            // Clean previous texture if any.
            if self.texture_handle.is_some() {
                ctx.unregister_texture(self.texture_handle.take().unwrap());
            }
            if let Some(old_handle) = self.manual_texture_view_handle {
                manual_texture_views.remove(&old_handle);
                self.manual_texture_view_handle = None;
            }

            // Create the texture for the camera target and the CustomPaintCtx.
            let format = TextureFormat::Rgba8UnormSrgb;
            let wgpu_texture = self.wgpu_device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let wgpu_texture_view =
                wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
            let manual_texture_view = ManualTextureView {
                texture_view: wgpu_texture_view.into(),
                size: bevy::math::UVec2::new(width, height),
                format,
            };
            let manual_texture_view_handle = ManualTextureViewHandle(0);
            manual_texture_views.insert(manual_texture_view_handle, manual_texture_view);

            if let Ok(mut camera) = world.query::<&mut Camera>().single_mut(world) {
                camera.target = RenderTarget::TextureView(manual_texture_view_handle);

                self.last_texture_size = current_size;
                self.manual_texture_view_handle = Some(manual_texture_view_handle);
                self.texture_handle = Some(ctx.register_texture(wgpu_texture));
            }
        }
    }
}
