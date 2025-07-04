use std::sync::Arc;

use crate::bevy_scene_plugin::BevyScenePlugin;
use bevy::{
    prelude::*,
    render::{
        camera::{ManualTextureView, ManualTextureViewHandle, ManualTextureViews, RenderTarget},
        render_resource::TextureFormat,
        renderer::{
            RenderAdapter, RenderAdapterInfo, RenderDevice, RenderInstance, RenderQueue,
            WgpuWrapper,
        },
        settings::{RenderCreation, RenderResources},
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
    wgpu_device: wgpu::Device,
    wgpu_queue: wgpu::Queue,
    last_texture_size: (u32, u32),
    wgpu_texture: Option<wgpu::Texture>,
    texture_handle: Option<TextureHandle>,
    manual_texture_view_handle: Option<ManualTextureViewHandle>,
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
        app.insert_resource(ManualTextureViews::default());

        // Add the scene
        app.add_plugins(BevyScenePlugin {});

        // Initialize the app to set up render world properly
        app.finish();
        app.cleanup();

        Self {
            app,
            wgpu_device: device_handle.device.clone(),
            wgpu_queue: device_handle.queue.clone(),
            last_texture_size: (0, 0),
            wgpu_texture: None,
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
        // Update the UI data
        if let Some(mut ui) = self.app.world_mut().get_resource_mut::<UIData>() {
            ui.width = width;
            ui.height = height;
            ui.color = color;
        }

        self.create_texture(ctx, width, height);

        // Run one frame of the Bevy app to render the 3D scene, and update the texture.
        self.app.update();

        self.texture_handle
    }

    fn create_texture(&mut self, mut ctx: CustomPaintCtx<'_>, width: u32, height: u32) {
        let current_size = (width, height);
        if self.texture_handle.is_some() && self.last_texture_size == current_size {
            return;
        }

        if self
            .app
            .world_mut()
            .query::<&Camera>()
            .single(self.app.world_mut())
            .ok()
            .is_none()
        {
            return;
        }

        if let Some(mut manual_texture_views) = self
            .app
            .world_mut()
            .get_resource_mut::<ManualTextureViews>()
        {
            if let Some(texture_handle) = self.texture_handle {
                ctx.unregister_texture(texture_handle);
                self.wgpu_texture = None;
                self.texture_handle = None;
            }

            if let Some(old_handle) = self.manual_texture_view_handle {
                manual_texture_views.remove(&old_handle);
            }

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
            self.texture_handle = Some(ctx.register_texture(wgpu_texture.clone()));
            self.wgpu_texture = Some(wgpu_texture);

            let wgpu_texture = self.wgpu_texture.as_ref().unwrap();
            let wgpu_texture_view =
                wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
            let manual_texture_view = ManualTextureView {
                texture_view: wgpu_texture_view.into(),
                size: bevy::math::UVec2::new(width, height),
                format,
            };
            let manual_texture_view_handle = ManualTextureViewHandle(1235078584);
            manual_texture_views.insert(manual_texture_view_handle, manual_texture_view);

            if let Some(mut camera_query) = self
                .app
                .world_mut()
                .query::<&mut Camera>()
                .single_mut(self.app.world_mut())
                .ok()
            {
                camera_query.target = RenderTarget::TextureView(manual_texture_view_handle);
            }

            self.last_texture_size = current_size;
            self.manual_texture_view_handle = Some(manual_texture_view_handle);
        }
    }
}
