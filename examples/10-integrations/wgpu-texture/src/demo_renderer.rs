// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT
use crate::Color;
use dioxus_native::{CustomPaintCtx, CustomPaintSource, DeviceHandle, TextureHandle};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Instant;
use wgpu::{
    CommandEncoderDescriptor, Device, Extent3d, FragmentState, LoadOp, MultisampleState,
    Operations, PipelineLayoutDescriptor, PrimitiveState, PushConstantRange, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, StoreOp, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor, VertexState,
};

pub struct DemoPaintSource {
    state: DemoRendererState,
    start_time: std::time::Instant,
    tx: Sender<DemoMessage>,
    rx: Receiver<DemoMessage>,
    color: Color,
}

impl CustomPaintSource for DemoPaintSource {
    fn resume(&mut self, device_handle: &DeviceHandle) {
        // Extract device and queue from device_handle
        let device = &device_handle.device;
        let queue = &device_handle.queue;
        let active_state = ActiveDemoRenderer::new(device, queue);
        self.state = DemoRendererState::Active(Box::new(active_state));
    }

    fn suspend(&mut self) {
        self.state = DemoRendererState::Suspended;
    }

    fn render(
        &mut self,
        ctx: CustomPaintCtx<'_>,
        width: u32,
        height: u32,
        _scale: f64,
    ) -> Option<TextureHandle> {
        self.process_messages();
        self.render(ctx, width, height)
    }
}

pub enum DemoMessage {
    // Color in RGB format
    SetColor(Color),
}

enum DemoRendererState {
    Active(Box<ActiveDemoRenderer>),
    Suspended,
}

#[derive(Clone)]
struct TextureAndHandle {
    texture: Texture,
    handle: TextureHandle,
}

struct ActiveDemoRenderer {
    device: Device,
    queue: Queue,
    pipeline: RenderPipeline,
    displayed_texture: Option<TextureAndHandle>,
    next_texture: Option<TextureAndHandle>,
}

impl DemoPaintSource {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self::with_channel(tx, rx)
    }

    pub fn with_channel(tx: Sender<DemoMessage>, rx: Receiver<DemoMessage>) -> Self {
        Self {
            state: DemoRendererState::Suspended,
            start_time: std::time::Instant::now(),
            tx,
            rx,
            color: Color::WHITE,
        }
    }

    pub fn sender(&self) -> Sender<DemoMessage> {
        self.tx.clone()
    }

    fn process_messages(&mut self) {
        loop {
            match self.rx.try_recv() {
                Err(_) => return,
                Ok(msg) => match msg {
                    DemoMessage::SetColor(color) => self.color = color,
                },
            }
        }
    }

    fn render(
        &mut self,
        ctx: CustomPaintCtx<'_>,
        width: u32,
        height: u32,
    ) -> Option<TextureHandle> {
        if width == 0 || height == 0 {
            return None;
        }
        let DemoRendererState::Active(state) = &mut self.state else {
            return None;
        };

        state.render(ctx, self.color.components, width, height, &self.start_time)
    }
}

impl ActiveDemoRenderer {
    pub(crate) fn new(device: &Device, queue: &Queue) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::FRAGMENT,
                range: 0..16, // full size in bytes, aligned
            }],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(TextureFormat::Rgba8Unorm.into())],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            device: device.clone(),
            queue: queue.clone(),
            pipeline,
            displayed_texture: None,
            next_texture: None,
        }
    }

    pub(crate) fn render(
        &mut self,
        mut ctx: CustomPaintCtx<'_>,
        light: [f32; 3],
        width: u32,
        height: u32,
        start_time: &Instant,
    ) -> Option<TextureHandle> {
        // If "next texture" size doesn't match specified size then unregister and drop texture
        if let Some(next) = &self.next_texture {
            if next.texture.width() != width || next.texture.height() != height {
                ctx.unregister_texture(self.next_texture.take().unwrap().handle);
            }
        }

        // If there is no "next texture" then create one and register it.
        let texture_and_handle = match &self.next_texture {
            Some(next) => next,
            None => {
                let texture = create_texture(&self.device, width, height);
                let handle = ctx.register_texture(texture.clone());
                self.next_texture = Some(TextureAndHandle { texture, handle });
                self.next_texture.as_ref().unwrap()
            }
        };

        let next_texture = &texture_and_handle.texture;
        let next_texture_handle = texture_and_handle.handle.clone();

        let elapsed: f32 = start_time.elapsed().as_millis() as f32 / 500.;
        let [light_red, light_green, light_blue] = light;
        let push_constants = PushConstants {
            light_color_and_time: [light_red, light_green, light_blue, elapsed],
        };

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &next_texture.create_view(&TextureViewDescriptor::default()),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::GREEN),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.pipeline);
            rpass.set_push_constants(
                ShaderStages::FRAGMENT, // Stage (your constants are for fragment shader)
                0,                      // Offset in bytes (start at 0)
                bytemuck::bytes_of(&push_constants),
            );
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));

        std::mem::swap(&mut self.next_texture, &mut self.displayed_texture);
        Some(next_texture_handle)
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PushConstants {
    light_color_and_time: [f32; 4],
}

fn create_texture(device: &Device, width: u32, height: u32) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: None,
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
