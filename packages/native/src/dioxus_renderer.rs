use std::rc::Rc;
use std::sync::Arc;
use std::{any::Any, cell::RefCell};

use anyrender::{RenderContext, WindowRenderer};

// Renderer imports
cfg_if::cfg_if! {
    if #[cfg(feature = "vello")] {
        pub use anyrender_vello::{
            VelloRendererOptions as InnerRendererOptions, VelloWindowRenderer as InnerRenderer,
            wgpu::{Features, Limits},
        };
    } else if #[cfg(feature = "vello-cpu-base")] {
        use anyrender_vello_cpu::VelloCpuWindowRenderer as InnerRenderer;
    } else if #[cfg(feature = "skia")] {
        use anyrender_skia::SkiaWindowRenderer as InnerRenderer;
        } else if #[cfg(feature = "vello-hybrid")] {
        pub use anyrender_vello_hybrid::{
            VelloHybridRendererOptions as InnerRendererOptions, VelloHybridWindowRenderer as InnerRenderer,
            wgpu::{Features, Limits},
        };
    } else {
        compile_error!("At least one renderer feature must be enabled");
    }
}

#[derive(Clone)]
pub struct DioxusNativeWindowRenderer {
    inner: Rc<RefCell<InnerRenderer>>,
}

impl Default for DioxusNativeWindowRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl DioxusNativeWindowRenderer {
    pub fn new() -> Self {
        let vello_renderer = InnerRenderer::new();
        Self::with_inner_renderer(vello_renderer)
    }

    #[cfg(any(feature = "vello-hybrid", feature = "vello"))]
    pub fn with_features_and_limits(features: Option<Features>, limits: Option<Limits>) -> Self {
        let vello_renderer = InnerRenderer::with_options(InnerRendererOptions {
            features,
            limits,
            ..Default::default()
        });
        Self::with_inner_renderer(vello_renderer)
    }

    fn with_inner_renderer(vello_renderer: InnerRenderer) -> Self {
        Self {
            inner: Rc::new(RefCell::new(vello_renderer)),
        }
    }
}

impl RenderContext for DioxusNativeWindowRenderer {
    fn try_register_custom_resource(
        &mut self,
        resource: Box<dyn Any>,
    ) -> Result<anyrender::ResourceId, anyrender::RegisterResourceError> {
        self.inner
            .borrow_mut()
            .try_register_custom_resource(resource)
    }

    fn unregister_resource(&mut self, resource_id: anyrender::ResourceId) {
        self.inner.borrow_mut().unregister_resource(resource_id)
    }

    fn renderer_specific_context(&self) -> Option<Box<dyn Any>> {
        self.inner.borrow_mut().renderer_specific_context()
    }
}
impl WindowRenderer for DioxusNativeWindowRenderer {
    type ScenePainter<'a>
        = <InnerRenderer as WindowRenderer>::ScenePainter<'a>
    where
        Self: 'a;

    fn resume<F: FnOnce() + 'static>(
        &mut self,
        window: Arc<dyn anyrender::WindowHandle>,
        width: u32,
        height: u32,
        on_ready: F,
    ) {
        self.inner
            .borrow_mut()
            .resume(window, width, height, on_ready)
    }

    fn complete_resume(&mut self) -> bool {
        self.inner.borrow_mut().complete_resume()
    }

    fn suspend(&mut self) {
        self.inner.borrow_mut().suspend()
    }

    fn is_active(&self) -> bool {
        self.inner.borrow().is_active()
    }

    fn is_pending(&self) -> bool {
        self.inner.borrow().is_pending()
    }

    fn set_size(&mut self, width: u32, height: u32) {
        self.inner.borrow_mut().set_size(width, height)
    }

    fn render<F: FnOnce(&mut Self::ScenePainter<'_>)>(&mut self, draw_fn: F) {
        self.inner.borrow_mut().render(draw_fn)
    }
}
