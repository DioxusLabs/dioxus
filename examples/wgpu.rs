use dioxus::desktop::tao::event::Event as WryEvent;
use dioxus::desktop::tao::window::WindowBuilder;
use dioxus::desktop::{use_window, use_wry_event_handler, window, DesktopContext};
use dioxus::prelude::*;
use ouroboros::self_referencing;
use std::borrow::Cow;
use wgpu;

fn main() {
    let config = dioxus::desktop::Config::new()
        .with_window(WindowBuilder::new().with_transparent(true))
        .with_as_child_window();
    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(app);
}

async fn setup_triangle<'a>(
    context: &'a DesktopContext,
) -> (
    wgpu::Surface<'a>,
    wgpu::Device,
    wgpu::RenderPipeline,
    wgpu::Queue,
) {
    let window = &context.window;
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
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
            },
            None,
        )
        .await
        .expect("Failed to create device");

    // Load the shaders from disk
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
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
"#,
        )),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(swapchain_format.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
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
    (surface, device, render_pipeline, queue)
}

fn render_triangle(
    surface: &wgpu::Surface,
    device: &wgpu::Device,
    render_pipeline: &wgpu::RenderPipeline,
    queue: &wgpu::Queue,
) {
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
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        rpass.set_pipeline(&render_pipeline);
        rpass.draw(0..3, 0..1);
    }

    queue.submit(Some(encoder.finish()));
    frame.present();
}

struct GraphicsResources<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    pipeline: wgpu::RenderPipeline,
    queue: wgpu::Queue,
}

#[self_referencing]
struct GraphicsContext {
    desktop: DesktopContext,
    #[borrows(desktop)]
    #[not_covariant]
    resources: GraphicsResources<'this>,
}

fn app() -> Element {
    let mut graphics_resources = use_resource(move || async {
        let context = GraphicsContextAsyncBuilder {
            desktop: window(),
            resources_builder: |desktop: &DesktopContext| {
                Box::pin(async move {
                    let (surface, device, pipeline, queue) = setup_triangle(desktop).await;
                    GraphicsResources {
                        surface: surface,
                        device: device,
                        pipeline: pipeline,
                        queue: queue,
                    }
                })
            },
        }
        .build()
        .await;
        println!("finished allocation of resources");
        context
    });

    let desktop_context = use_window();
    graphics_resources.read().as_ref().map(|_| {
        desktop_context.window.request_redraw();
    });

    use_wry_event_handler(move |event, _| {
        if let WryEvent::RedrawRequested(id) = event {
            println!("Redraw requested for window with id: {:?}", id);
            graphics_resources.read().as_ref().map(|resources| {
                resources.with_resources(|resources| {
                    render_triangle(
                        &resources.surface,
                        &resources.device,
                        &resources.pipeline,
                        &resources.queue,
                    );
                })
            });
        } else if let WryEvent::WindowEvent {
            event: dioxus::desktop::tao::event::WindowEvent::Resized(_new_size),
            ..
        } = event
        {
            // TODO: use the new_size to update the existing surface instead of recreating the entire graphics resource
            graphics_resources.restart();
        }
    });

    rsx! {
        div {
            p {
                color: "red",
                "hello world"
            }
        }
    }
}
