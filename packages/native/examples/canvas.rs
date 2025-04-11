use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_native::SharedNativeTexture;
use wgpu::{
    core::instance, Extent3d, ImageCopyTexture, ImageCopyTextureBase, InstanceDescriptor, Origin3d,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let width = 500;
    let height = 500;

    use_future(move || attach_egui(width, height));

    rsx! {
        div {
            h1 { "Hello native canvas" }
            canvas {
                id: "egui-demo",
                width: "{width}",
                height: "{height}",
                style: "border: 1px solid black;",
            }
        }
    }
}

async fn attach_egui(width: u32, height: u32) {
    let document = dioxus_native::document();

    let instance = wgpu::Instance::default();
    let surface = instance.create_surface(document.window_handle()).unwrap();

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .unwrap();

    let (device, _queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let texture = device.create_texture(&TextureDescriptor {
        label: Some("egui-demo"),
        format: TextureFormat::Rgba32Float,
        usage: TextureUsages::RENDER_ATTACHMENT,
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        view_formats: &[],
    });

    let copy_texture = ImageCopyTextureBase {
        texture: Arc::new(texture),
        mip_level: 1,
        origin: Origin3d { x: 0, y: 0, z: 0 },
        aspect: TextureAspect::All,
    };

    document.set_custom_texture(
        "egui-demo",
        SharedNativeTexture {
            inner: copy_texture,
        },
    );

    // todo - handle resize events!
    futures_util::future::pending::<()>().await;
}
