//! Demonstrate how to use dioxus as a child window for use in alternative renderers like wgpu.
//!
//! The code here is borrowed from wry's example:
//! https://github.com/tauri-apps/wry/blob/dev/examples/wgpu.rs
//!
//! To use this feature set `with_as_child_window()` on your desktop config which will then let you

use dioxus::prelude::*;
use dioxus::{
    desktop::tao::{event::Event as WryEvent, window::Window},
    desktop::{Config, tao::window::WindowBuilder, use_wry_event_handler, window},
};
use std::sync::Arc;

fn main() {
    let config = Config::new()
        .with_window(WindowBuilder::new().with_transparent(true))
        .with_on_window(|window, dom| {
            let resources = Arc::new(pollster::block_on(async {
                let resource = GraphicsContextAsyncBuilder {
                    desktop: window,
                    resources_builder: |ctx| Box::pin(GraphicsResources::new(ctx.clone())),
                }
                .build()
                .await;

                resource.with_resources(|resources| resources.render());

                resource
            }));

            dom.provide_root_context(resources);
        })
        .with_as_child_window();

    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(app);
}

fn app() -> Element {
    let graphics_resources = consume_context::<Arc<GraphicsContext>>();

    // on first render request a redraw
    use_effect(|| {
        window().window.request_redraw();
    });

    use_wry_event_handler(move |event, _| {
        use dioxus::desktop::tao::event::WindowEvent;

        if let WryEvent::WindowEvent {
            event: WindowEvent::Resized(new_size),
            ..
        } = event
        {
            graphics_resources.with_resources(|srcs| {
                let mut cfg = srcs.config.clone();
                cfg.width = new_size.width;
                cfg.height = new_size.height;
                srcs.surface.configure(&srcs.device, &cfg);
            });

            window().window.request_redraw();
        }
    });

    rsx! {
        div {
            color: "blue",
            width: "100vw",
            height: "100vh",
            display: "flex",
            justify_content: "center",
            align_items: "center",
            font_size: "20px",
            div { "text overlaid on a wgpu surface!" }
        }
    }
}

/// This borrows from the `window` which is contained within an `Arc` so we need to wrap it in a self-borrowing struct
/// to be able to borrow the window for the wgpu::Surface
#[ouroboros::self_referencing]
struct GraphicsContext {
    desktop: Arc<Window>,
    #[borrows(desktop)]
    #[not_covariant]
    resources: GraphicsResources<'this>,
}

struct GraphicsResources<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    pipeline: wgpu::RenderPipeline,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl<'a> GraphicsResources<'a> {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::default();

        let surface: wgpu::Surface<'a> = instance.create_surface(window).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::default(),
                ..Default::default()
            })
            .await
            .expect("Failed to create device");

        // Load the shaders from disk
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                r#"
@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(in_vertex_index) - 1);
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
"#
                .into(),
            ),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(swapchain_format.into())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::PostMultiplied,
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        GraphicsResources {
            surface,
            device,
            pipeline,
            queue,
            config,
        }
    }

    fn render(&self) {
        let GraphicsResources {
            surface,
            device,
            pipeline,
            queue,
            ..
        } = self;

        let frame = surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(pipeline);
            rpass.draw(0..3, 0..1);
        }

        queue.submit(Some(encoder.finish()));
        frame.present();
    }
}
