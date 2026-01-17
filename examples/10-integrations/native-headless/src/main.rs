//! A "sketch" of how to integrate a Dioxus Native app to into a wider application
//! by rendering the UI to a texture and driving it with your own event loop
//!
//! (this example is not really intended to be run as-is, and requires you to fill
//! in the missing pieces)
use anyrender_vello::VelloScenePainter;
use blitz_dom::{Document as _, DocumentConfig};
use blitz_paint::paint_scene;
use blitz_traits::{
    events::{BlitzMouseButtonEvent, MouseEventButton, MouseEventButtons, UiEvent},
    shell::{ColorScheme, Viewport},
};
use dioxus::prelude::*;
use dioxus_native_dom::DioxusDocument;
use pollster::FutureExt as _;
use std::sync::Arc;
use std::task::Context;
use vello::{
    peniko::color::AlphaColor, RenderParams, Renderer as VelloRenderer, RendererOptions, Scene,
};
use wgpu::TextureFormat;
use wgpu_context::WGPUContext;

// Constant width, height, scale factor and color schemefor example purposes
const SCALE_FACTOR: f32 = 1.0;
const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const COLOR_SCHEME: ColorScheme = ColorScheme::Light;

// Example Dioxus app.
fn app() -> Element {
    rsx! {
        div { "Hello, world!" }
    }
}

fn main() {
    #[cfg(feature = "tracing")]
    tracing_subscriber::fmt::init();

    // =============
    // INITIAL SETUP
    // =============

    let waker = create_waker(Box::new(|| {
        // This should wake up and "poll" your event loop
    }));

    // Create the dioxus virtual dom and the dioxus-native document
    // It is important to set the width, height, and scale factor on the document as these are used for layout.
    let vdom = VirtualDom::new(app);
    let mut dioxus_doc = DioxusDocument::new(
        vdom,
        DocumentConfig {
            viewport: Some(Viewport::new(WIDTH, HEIGHT, SCALE_FACTOR, COLOR_SCHEME)),
            ..Default::default()
        },
    );

    // Setup a WGPU Device and Queue
    //
    // There is nothing special about WGPUContext. It is just used to
    // reduce the amount of boilerplate associated with setting up WGPU
    let mut wgpu_context = WGPUContext::new();
    let device_id = wgpu_context
        .find_or_create_device(None)
        .block_on()
        .expect("Failed to create WGPU device");
    let device_handle = &wgpu_context.device_pool[device_id];
    let device = device_handle.device.clone();
    let queue = device_handle.queue.clone();

    // Create Vello renderer
    // Note: creating a VelloRenderer is expensive, so it should be done once per Device.
    let mut vello_renderer = VelloRenderer::new(&device, RendererOptions::default()).unwrap();

    // =============
    // CREATE TEXTURE (RECREATE ON RESIZE)
    // =============

    // Create texture and texture view
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        format: TextureFormat::Rgba8Unorm,
        view_formats: &[],
    });
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // =============
    // EACH FRAME
    // =============

    // Poll the vdom
    dioxus_doc.poll(Some(Context::from_waker(&waker)));

    // Create a `vello::Scene` to paint into
    let mut scene = Scene::new();

    // Paint the document using `blitz_paint::paint_scene`
    //
    // Note: the `paint_scene` will work with any implementation of `anyrender::PaintScene`
    // so you could also write your own implementation if you want more control over rendering
    // (i.e. to render a custom renderer instead of Vello)
    paint_scene(
        &mut VelloScenePainter::new(&mut scene),
        &dioxus_doc,
        SCALE_FACTOR as f64,
        WIDTH,
        HEIGHT,
    );

    // Render the `vello::Scene` to the Texture using the `VelloRenderer`
    vello_renderer
        .render_to_texture(
            &device,
            &queue,
            &scene,
            &texture_view,
            &RenderParams {
                base_color: AlphaColor::TRANSPARENT,
                width: WIDTH,
                height: HEIGHT,
                antialiasing_method: vello::AaConfig::Msaa16,
            },
        )
        .expect("failed to render to texture");

    // `texture` will now contain the rendered Scene

    // =============
    // EVENT HANDLING
    // =============

    let event = UiEvent::MouseDown(BlitzMouseButtonEvent {
        x: 30.0,
        y: 40.0,
        button: MouseEventButton::Main,
        buttons: MouseEventButtons::Primary, // keep track of all pressed buttons
        mods: Modifiers::empty(),            // ctrl, alt, shift, etc
    });
    dioxus_doc.handle_ui_event(event);

    // Trigger a poll via your event loop (or wait for next frame)
}

/// Create a waker that will call an arbitrary callback
pub fn create_waker(callback: Box<dyn Fn() + 'static + Send + Sync>) -> std::task::Waker {
    struct DomHandle {
        callback: Box<dyn Fn() + 'static + Send + Sync>,
    }

    impl futures_util::task::ArcWake for DomHandle {
        fn wake_by_ref(arc_self: &Arc<Self>) {
            (arc_self.callback)()
        }
    }

    futures_util::task::waker(Arc::new(DomHandle { callback }))
}
